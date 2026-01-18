use axum::{
    extract::{Json, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Router,
};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use clap_mangen::Man;
use futures::future::join_all;
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fs;
use std::io::{self, BufRead, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error as ThisError;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

type SessionSender = mpsc::UnboundedSender<Result<Event, Infallible>>;
type SessionsMap = Arc<RwLock<HashMap<String, SessionSender>>>;

// =========================================================================
// 0. ERROR HANDLING
// =========================================================================

#[derive(ThisError, Debug)]
pub enum AppError {
    #[error("API Error: {0}")]
    ApiError(String),

    #[error("Configuration Error: {0}")]
    ConfigError(String),

    #[error("Resource Not Found")]
    NotFound,

    #[error("Validation Error: {0}")]
    ValidationError(String),

    #[error("Internal Error: {0}")]
    Internal(String),

    #[error("IO Error: {0}")]
    Io(#[from] io::Error),

    #[error("Network Error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization Error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML Error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("YAML Error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

// Implement conversion for Box<dyn Error + Send + Sync> to make refactoring easier
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}

impl AppError {
    pub fn to_json_rpc_error(&self) -> Value {
        let (code, message) = match self {
            AppError::ValidationError(_) => (-32602, self.to_string()),
            AppError::NotFound => (-32004, self.to_string()),
            AppError::ApiError(_) | AppError::Network(_) => (-32005, self.to_string()),
            AppError::ConfigError(_) => (-32603, self.to_string()),
            AppError::Internal(_)
            | AppError::Io(_)
            | AppError::Serialization(_)
            | AppError::Toml(_)
            | AppError::Yaml(_) => (-32603, self.to_string()),
        };

        json!({
            "code": code,
            "message": message
        })
    }
}

// ... (rest of imports if any)

// =========================================================================
// 1. CONFIGURATION
// =========================================================================

#[derive(Deserialize, Debug, Clone)]
struct ConfigFile {
    api_key: Option<String>,
    postal_code: Option<String>,
    species: Option<String>,
    miles: Option<u32>,
}

#[derive(Parser, Debug)]
#[command(author, version = env!("PROJECT_VERSION"), about)]
struct Cli {
    #[arg(long, env = "RESCUE_GROUPS_API_KEY", hide_env_values = true)]
    api_key: Option<String>,
    #[arg(long, default_value = "config.toml")]
    config: String,

    /// Output raw JSON instead of formatted text
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Start the MCP server (default)
    Server,
    /// Start the MCP server in HTTP mode
    Http(HttpArgs),
    /// Search for adoptable pets
    Search(ToolArgs),
    /// List available species
    ListSpecies,
    /// Get details for a specific animal
    GetAnimal(AnimalIdArgs),
    /// Get contact information for a specific animal
    GetContact(AnimalIdArgs),
    /// Compare multiple animals side-by-side
    Compare(CompareArgs),
    /// Search for rescue organizations
    SearchOrgs(OrgSearchArgs),
    /// Get details for a specific organization
    GetOrg(OrgIdArgs),
    /// List animals at a specific organization
    ListOrgAnimals(OrgIdArgs),
    /// List recently adopted animals (Success Stories)
    ListAdopted(AdoptedAnimalsArgs),
    /// List available breeds for a species
    ListBreeds(SpeciesArgs),
    /// List metadata values (colors, patterns, etc.)
    ListMetadata(MetadataArgs),
    /// Generate shell completions or man pages
    Generate(GenerateArgs),
}

#[derive(Args, Clone, Debug)]
struct HttpArgs {
    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to bind to
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Optional authentication token (Bearer token)
    #[arg(long, env = "MCP_AUTH_TOKEN")]
    auth_token: Option<String>,
}

#[derive(Args, Clone, Debug)]
struct GenerateArgs {
    /// Type of shell completion to generate
    #[arg(short, long)]
    shell: Option<Shell>,

    /// Generate man pages to the specified directory
    #[arg(short, long)]
    man: Option<String>,
}

#[derive(Clone)]
struct Settings {
    api_key: String,
    base_url: String,
    default_postal_code: String,
    default_miles: u32,
    default_species: String,
    cache: Arc<moka::future::Cache<String, Value>>,
}

fn merge_configuration(cli: &Cli) -> Result<Settings, Box<dyn Error + Send + Sync>> {
    let config_path = Path::new(&cli.config);

    let file_config: Option<ConfigFile> = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        let ext = config_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        match ext {
            "toml" => Some(toml::from_str(&content)?),
            "json" => Some(serde_json::from_str(&content)?),
            "yaml" | "yml" => Some(serde_yaml::from_str(&content)?),
            _ => None,
        }
    } else {
        None
    };

    let api_key = cli
        .api_key
        .clone()
        .or(file_config.as_ref().and_then(|c| c.api_key.clone()))
        .ok_or("API Key is missing! Set RESCUE_GROUPS_API_KEY or use config.toml")?;

    let cache = moka::future::Cache::builder()
        .max_capacity(100)
        .time_to_live(std::time::Duration::from_secs(15 * 60)) // 15 minutes
        .build();

    Ok(Settings {
        api_key,
        base_url: "https://api.rescuegroups.org/v5".to_string(),
        default_postal_code: file_config
            .as_ref()
            .and_then(|c| c.postal_code.clone())
            .unwrap_or_else(|| "90210".to_string()),
        default_miles: file_config.as_ref().and_then(|c| c.miles).unwrap_or(50),
        default_species: file_config
            .as_ref()
            .and_then(|c| c.species.clone())
            .unwrap_or_else(|| "dogs".to_string()),
        cache: Arc::new(cache),
    })
}

async fn fetch_with_cache(
    settings: &Settings,
    url: &str,
    method: &str,
    body: Option<Value>,
) -> Result<Value, AppError> {
    let cache_key = format!(
        "{}:{}:{}",
        method,
        url,
        body.as_ref().map(|b| b.to_string()).unwrap_or_default()
    );

    if let Some(cached) = settings.cache.get(&cache_key).await {
        return Ok(cached);
    }

    let client = reqwest::Client::new();
    let mut request = match method {
        "POST" => client.post(url),
        _ => client.get(url),
    };

    request = request
        .header("Authorization", &settings.api_key)
        .header("Content-Type", "application/vnd.api+json");

    if let Some(b) = body {
        request = request.json(&b);
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound);
        }
        return Err(AppError::ApiError(format!("API Error: {}", response.status())));
    }

    let data: Value = response.json().await?;
    settings.cache.insert(cache_key, data.clone()).await;
    Ok(data)
}

// =========================================================================
// 2. CORE LOGIC (The Search Function)
// =========================================================================

#[derive(Args, Deserialize, Clone, Debug)]
struct ToolArgs {
    #[arg(long)]
    postal_code: Option<String>,
    #[arg(long)]
    miles: Option<u32>,
    #[arg(long)]
    species: Option<String>,
    #[arg(long)]
    breeds: Option<String>,
    #[arg(long)]
    sex: Option<String>,
    #[arg(long)]
    age: Option<String>,
    #[arg(long)]
    size: Option<String>,
    #[arg(long)]
    good_with_children: Option<bool>,
    #[arg(long)]
    good_with_dogs: Option<bool>,
    #[arg(long)]
    good_with_cats: Option<bool>,
    #[arg(long)]
    house_trained: Option<bool>,
    #[arg(long)]
    special_needs: Option<bool>,
    #[arg(long)]
    sort_by: Option<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
struct AnimalIdArgs {
    #[arg(long)]
    animal_id: String,
}

#[derive(Args, Deserialize, Clone, Debug)]
struct CompareArgs {
    /// Comma-separated list of animal IDs to compare (max 5)
    #[arg(long, value_delimiter = ',')]
    animal_ids: Vec<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
struct SpeciesArgs {
    #[arg(long)]
    species: String,
}

#[derive(Args, Deserialize, Clone, Debug)]
struct OrgSearchArgs {
    #[arg(long)]
    postal_code: Option<String>,
    #[arg(long)]
    miles: Option<u32>,
}

#[derive(Args, Deserialize, Clone, Debug)]
struct OrgIdArgs {
    #[arg(long)]
    org_id: String,
}

#[derive(Args, Deserialize, Clone, Debug)]
struct AdoptedAnimalsArgs {
    #[arg(long)]
    postal_code: Option<String>,
    #[arg(long)]
    miles: Option<u32>,
    #[arg(long)]
    species: Option<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
struct MetadataArgs {
    #[arg(long)]
    metadata_type: String,
}

fn extract_single_item(data: &Value) -> Option<&Value> {
    match data {
        Value::Array(arr) => arr.first(),
        Value::Object(_) => Some(data),
        _ => None,
    }
}

fn format_single_animal(animal: &Value) -> String {
    let attrs = &animal["attributes"];
    let name = attrs["name"].as_str().unwrap_or("Unknown");
    let breed = attrs["breedString"].as_str().unwrap_or("Mix");
    let description = attrs["descriptionText"]
        .as_str()
        .unwrap_or("No description available.");
    let sex = attrs["sex"].as_str().unwrap_or("Unknown");
    let age = attrs["ageGroup"].as_str().unwrap_or("Unknown");
    let size = attrs["sizeGroup"].as_str().unwrap_or("Unknown");
    let url = attrs["url"].as_str().unwrap_or("");

    let img = attrs["orgsAnimalsPictures"]
        .as_array()
        .and_then(|p| p.first())
        .and_then(|p| p["urlSecureFullsize"].as_str())
        .map(|u| format!("![{}]({})", name, u))
        .unwrap_or_default();

    format!(
        "# {}\n**Breed:** {}\n**Sex:** {}\n**Age:** {}\n**Size:** {}\n\n{}\n\n{}\n\n[View on RescueGroups]({})",
        name, breed, sex, age, size, img, description, url
    )
}

fn format_contact_info(data: &Value) -> Result<String, AppError> {
    let animal_data = data.get("data").ok_or(AppError::NotFound)?;
    let animal = extract_single_item(animal_data).ok_or(AppError::NotFound)?;

    let animal_attrs = &animal["attributes"];
    let animal_name = animal_attrs["name"].as_str().unwrap_or("this pet");

    let mut contact_info = format!("## Contact Information for {}\n\n", animal_name);

    // Try to find org info in "included"
    let org = data
        .get("included")
        .and_then(|inc| inc.as_array()?.iter().find(|item| item["type"] == "orgs"));

    if let Some(o) = org {
        let attrs = &o["attributes"];
        let name = attrs["name"].as_str().unwrap_or("Unknown Organization");
        let email = attrs["email"].as_str().unwrap_or("No email provided");
        let phone = attrs["phone"].as_str().unwrap_or("No phone provided");
        let city = attrs["city"].as_str().unwrap_or("Unknown City");
        let state = attrs["state"].as_str().unwrap_or("");
        let url = attrs["url"].as_str().unwrap_or("");

        contact_info.push_str(&format!("**Organization:** {}\n", name));
        contact_info.push_str(&format!("**Email:** {}\n", email));
        contact_info.push_str(&format!("**Phone:** {}\n", phone));
        contact_info.push_str(&format!("**Location:** {}, {}\n", city, state));
        if !url.is_empty() {
            contact_info.push_str(&format!("**Website:** [{}]({})\n", url, url));
        }
    } else {
        contact_info.push_str(
            "Detailed organization contact information is not available for this animal.\n",
        );
    }

    let animal_url = animal_attrs["url"].as_str().unwrap_or("");
    if !animal_url.is_empty() {
        contact_info.push_str(&format!(
            "\n[View adoption application or more info on RescueGroups]({})\n",
            animal_url
        ));
    }

    Ok(contact_info)
}

fn format_animal_results(data: &Value) -> Result<String, AppError> {
    let animals = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or(AppError::NotFound)?;

    if animals.is_empty() {
        return Ok("No adoptable animals found.".to_string());
    }

    let results: Vec<String> = animals
        .iter()
        .take(5)
        .map(|animal| {
            let attrs = &animal["attributes"];
            let name = attrs["name"].as_str().unwrap_or("Unknown");
            let breed = attrs["breedString"].as_str().unwrap_or("Mix");
            let url = attrs["url"].as_str().unwrap_or("");

            let img = attrs["orgsAnimalsPictures"]
                .as_array()
                .and_then(|p| p.first())
                .and_then(|p| p["urlSecureFullsize"].as_str())
                .map(|u| format!("![{}]({})", name, u))
                .unwrap_or_default();

            format!("### [{}]({})\n**Breed:** {}\n\n{}", name, url, breed, img)
        })
        .collect();

    Ok(results.join("\n\n---\n\n"))
}

fn format_comparison_table(data: &Value) -> Result<String, AppError> {
    let animals = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or(AppError::NotFound)?;

    if animals.is_empty() {
        return Ok("No animals to compare.".to_string());
    }

    let headers = vec![
        "Breed", "Age", "Sex", "Size", "Kids?", "Dogs?", "Cats?", "Trained?", "Special?",
    ];

    let mut markdown = String::new();

    // Header Row
    markdown.push_str("| Feature |");
    for animal in animals {
        let name = animal["attributes"]["name"].as_str().unwrap_or("Unknown");
        let url = animal["attributes"]["url"].as_str().unwrap_or("");
        markdown.push_str(&format!(" [{}]({}) |", name, url));
    }
    markdown.push('\n');

    // Separator Row
    markdown.push_str("| :--- |");
    for _ in animals {
        markdown.push_str(" :--- |");
    }
    markdown.push('\n');

    // Data Rows
    for header in headers {
        markdown.push_str(&format!("| **{}** |", header));
        for animal in animals {
            let attrs = &animal["attributes"];
            let val = match header {
                "Breed" => attrs["breedString"].as_str().unwrap_or("-").to_string(),
                "Age" => attrs["ageGroup"].as_str().unwrap_or("-").to_string(),
                "Sex" => attrs["sex"].as_str().unwrap_or("-").to_string(),
                "Size" => attrs["sizeGroup"].as_str().unwrap_or("-").to_string(),
                "Kids?" => attrs["isGoodWithChildren"]
                    .as_str()
                    .unwrap_or("-")
                    .to_string(),
                "Dogs?" => attrs["isGoodWithDogs"].as_str().unwrap_or("-").to_string(),
                "Cats?" => attrs["isGoodWithCats"].as_str().unwrap_or("-").to_string(),
                "Trained?" => attrs["isHouseTrained"].as_str().unwrap_or("-").to_string(),
                "Special?" => attrs["isSpecialNeeds"].as_str().unwrap_or("-").to_string(),
                _ => "-".to_string(),
            };
            markdown.push_str(&format!(" {} |", val));
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn format_single_org(org: &Value) -> String {
    let attrs = &org["attributes"];
    let name = attrs["name"].as_str().unwrap_or("Unknown");
    let about = attrs["about"]
        .as_str()
        .unwrap_or("No description available.");
    let address = attrs["street"].as_str().unwrap_or("");
    let city = attrs["city"].as_str().unwrap_or("Unknown City");
    let state = attrs["state"].as_str().unwrap_or("");
    let postal_code = attrs["postalcode"].as_str().unwrap_or("");
    let email = attrs["email"].as_str().unwrap_or("No email provided");
    let phone = attrs["phone"].as_str().unwrap_or("No phone provided");
    let url = attrs["url"].as_str().unwrap_or("");
    let facebook = attrs["facebookUrl"].as_str().unwrap_or("");

    format!(
        "# {}\n\n{}\n\n**Address:** {} {}, {} {}\n**Phone:** {}\n**Email:** {}\n**Website:** {}\n**Facebook:** {}",
        name, about, address, city, state, postal_code, phone, email, url, facebook
    )
}

fn format_species_results(data: &Value) -> Result<String, AppError> {
    let species = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or(AppError::NotFound)?;

    if species.is_empty() {
        return Ok("No species found.".to_string());
    }

    let mut names: Vec<String> = species
        .iter()
        .filter_map(|s| s["attributes"]["singular"].as_str().map(|n| n.to_string()))
        .collect();

    names.sort();

    Ok(format!("### Supported Species\n\n{}", names.join("\n")))
}

fn format_metadata_results(
    data: &Value,
    metadata_type: &str,
) -> Result<String, AppError> {
    let items = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or(AppError::NotFound)?;

    if items.is_empty() {
        return Ok(format!("No {} found.", metadata_type));
    }

    let mut names: Vec<String> = items
        .iter()
        .filter_map(|i| i["attributes"]["name"].as_str().map(|n| n.to_string()))
        .collect();

    names.sort();

    Ok(format!(
        "### Supported {}\n\n{}",
        metadata_type,
        names.join("\n")
    ))
}

fn format_org_results(data: &Value) -> Result<String, AppError> {
    let orgs = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or(AppError::NotFound)?;

    if orgs.is_empty() {
        return Ok("No organizations found.".to_string());
    }

    let results: Vec<String> = orgs
        .iter()
        .take(5)
        .map(|org| {
            let attrs = &org["attributes"];
            let name = attrs["name"].as_str().unwrap_or("Unknown");
            let city = attrs["city"].as_str().unwrap_or("Unknown City");
            let state = attrs["state"].as_str().unwrap_or("");
            let email = attrs["email"].as_str().unwrap_or("No email provided");
            let url = attrs["url"].as_str().unwrap_or("");
            let id = org["id"].as_str().unwrap_or("Unknown ID");

            format!(
                "### {}\n**ID:** {}\n**Location:** {}, {}\n**Email:** {}\n**Website:** {}",
                name, id, city, state, email, url
            )
        })
        .collect();

    Ok(results.join("\n\n---\n\n"))
}

fn format_breed_results(
    data: &Value,
    species: &str,
) -> Result<String, AppError> {
    let breeds = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or(AppError::NotFound)?;

    if breeds.is_empty() {
        return Ok(format!("No breeds found for species '{}'.", species));
    }

    let mut breed_names: Vec<String> = breeds
        .iter()
        .filter_map(|b| b["attributes"]["name"].as_str().map(|s| s.to_string()))
        .collect();

    breed_names.sort();

    Ok(format!(
        "### Breeds for {}\n\n{}",
        species,
        breed_names.join("\n")
    ))
}

fn print_output<F>(
    result: Result<Value, AppError>,
    json_mode: bool,
    formatter: F,
) where
    F: Fn(&Value) -> Result<String, AppError>,
{
    match result {
        Ok(value) => {
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&value).unwrap());
            } else {
                match formatter(&value) {
                    Ok(text) => println!("{}", text),
                    Err(e) => error!("Error formatting output: {}", e),
                }
            }
        }
        Err(e) => error!("Error: {}", e),
    }
}

async fn list_breeds(
    settings: &Settings,
    args: SpeciesArgs,
) -> Result<Value, AppError> {
    let species_id = if args.species.chars().all(char::is_numeric) {
        args.species
    } else {
        // Try to resolve name to ID
        let species_list = list_species(settings).await?;
        let data = species_list
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or(AppError::Internal("Failed to fetch species list for resolution".to_string()))?;

        let target = args.species.to_lowercase();
        let found = data.iter().find(|s| {
            let attrs = &s["attributes"];
            let singular = attrs["singular"].as_str().unwrap_or("").to_lowercase();
            let plural = attrs["plural"].as_str().unwrap_or("").to_lowercase();
            singular == target || plural == target
        });

        if let Some(s) = found {
            s["id"].as_str().unwrap_or("").to_string()
        } else {
            return Err(AppError::NotFound);
        }
    };

    let url = format!(
        "{}/public/animals/species/{}/breeds",
        settings.base_url, species_id
    );
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn list_species(settings: &Settings) -> Result<Value, AppError> {
    let url = format!("{}/public/animals/species", settings.base_url);
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn list_metadata(
    settings: &Settings,
    args: MetadataArgs,
) -> Result<Value, AppError> {
    let url = format!(
        "{}/public/animals/{}",
        settings.base_url, args.metadata_type
    );
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn list_animals(settings: &Settings) -> Result<Value, AppError> {
    let url = format!("{}/public/animals", settings.base_url);
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn get_animal_details(
    settings: &Settings,
    args: AnimalIdArgs,
) -> Result<Value, AppError> {
    let url = format!("{}/public/animals/{}", settings.base_url, args.animal_id);
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn get_contact_info(
    settings: &Settings,
    args: AnimalIdArgs,
) -> Result<Value, AppError> {
    let url = format!(
        "{}/public/animals/{}?include=orgs",
        settings.base_url, args.animal_id
    );
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn compare_animals(
    settings: &Settings,
    args: CompareArgs,
) -> Result<Value, AppError> {
    let mut futures = Vec::new();
    // Deduplicate and limit
    let mut ids = args.animal_ids.clone();
    ids.sort();
    ids.dedup();

    for id in ids.iter().take(5) {
        let fut = get_animal_details(
            settings,
            AnimalIdArgs {
                animal_id: id.clone(),
            },
        );
        futures.push(fut);
    }

    let results = join_all(futures).await;

    let mut valid_animals = Vec::new();
    let mut errors = Vec::new();

    for res in results {
        match res {
            Ok(val) => {
                if let Some(data) = val.get("data") {
                    if let Some(animal) = extract_single_item(data) {
                        valid_animals.push(animal.clone());
                    }
                }
            }
            Err(e) => errors.push(e.to_string()),
        }
    }

    Ok(json!({ "data": valid_animals, "errors": errors }))
}

async fn search_organizations(
    settings: &Settings,
    args: OrgSearchArgs,
) -> Result<Value, AppError> {
    let url = format!("{}/public/orgs/search", settings.base_url);
    let miles = args.miles.unwrap_or(settings.default_miles);
    let postal_code = args
        .postal_code
        .as_deref()
        .unwrap_or(&settings.default_postal_code);

    let body = json!({
        "data": {
            "filterRadius": {
                "miles": miles,
                "postalcode": postal_code
            }
        }
    });

    fetch_with_cache(settings, &url, "POST", Some(body)).await
}

async fn get_organization_details(
    settings: &Settings,
    args: OrgIdArgs,
) -> Result<Value, AppError> {
    let url = format!("{}/public/orgs/{}", settings.base_url, args.org_id);
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn list_org_animals(
    settings: &Settings,
    args: OrgIdArgs,
) -> Result<Value, AppError> {
    let url = format!(
        "{}/public/orgs/{}/animals/search/available",
        settings.base_url, args.org_id
    );
    fetch_with_cache(settings, &url, "GET", None).await
}

async fn fetch_pets(
    settings: &Settings,
    args: ToolArgs,
) -> Result<Value, AppError> {
    // Merge Tool Args with Server Defaults
    // This is the "Dynamic Lookup" logic:
    // 1. If AI sends a postal_code, use it.
    // 2. If AI sends null/nothing, use settings.default_postal_code.
    let miles = args.miles.unwrap_or(settings.default_miles);
    let species = args.species.as_deref().unwrap_or(&settings.default_species);
    let postal_code = args
        .postal_code
        .as_deref()
        .unwrap_or(&settings.default_postal_code);

    let sort_param = match args.sort_by.as_deref() {
        Some("Newest") => "?sort=-animals.createdDate",
        Some("Distance") => "?sort=distance",
        Some("Random") => "?sort=random",
        _ => "",
    };

    let url = format!(
        "{}/public/animals/search/available/{}/haspic{}",
        settings.base_url, species, sort_param
    );

    let mut filters = Vec::new();

    if let Some(breeds) = &args.breeds {
        // Handle multiple breeds if separated by comma? The API usually takes an array for "oneOf" or "equal" if singular.
        // For simplicity, let's assume a single breed string or comma-separated for "contain" or similar?
        // RescueGroups filter usually works with ID or Name. Let's try name "contain" or "equal".
        // "breeds.name" is the field.
        filters.push(json!({
            "fieldName": "breeds.name",
            "operation": "contains",
            "criteria": breeds
        }));
    }

    if let Some(sex) = args.sex {
        filters.push(json!({
            "fieldName": "animals.sex",
            "operation": "equal",
            "criteria": sex
        }));
    }

    if let Some(age) = args.age {
        filters.push(json!({
            "fieldName": "animals.ageGroup",
            "operation": "equal",
            "criteria": age
        }));
    }

    if let Some(size) = args.size {
        filters.push(json!({
            "fieldName": "animals.sizeGroup",
            "operation": "equal",
            "criteria": size
        }));
    }

    if let Some(val) = args.good_with_children {
        filters.push(json!({
            "fieldName": "animals.isGoodWithChildren",
            "operation": "equal",
            "criteria": if val { "Yes" } else { "No" }
        }));
    }

    if let Some(val) = args.good_with_dogs {
        filters.push(json!({
            "fieldName": "animals.isGoodWithDogs",
            "operation": "equal",
            "criteria": if val { "Yes" } else { "No" }
        }));
    }

    if let Some(val) = args.good_with_cats {
        filters.push(json!({
            "fieldName": "animals.isGoodWithCats",
            "operation": "equal",
            "criteria": if val { "Yes" } else { "No" }
        }));
    }

    if let Some(val) = args.house_trained {
        filters.push(json!({
            "fieldName": "animals.isHouseTrained",
            "operation": "equal",
            "criteria": if val { "Yes" } else { "No" }
        }));
    }

    if let Some(val) = args.special_needs {
        filters.push(json!({
            "fieldName": "animals.isSpecialNeeds",
            "operation": "equal",
            "criteria": if val { "Yes" } else { "No" }
        }));
    }

    let mut data_obj = json!({
        "filterRadius": {
            "miles": miles,
            "postalcode": postal_code
        }
    });

    if !filters.is_empty() {
        data_obj["filters"] = json!(filters);
    }

    let body = json!({ "data": data_obj });

    fetch_with_cache(settings, &url, "POST", Some(body)).await
}

async fn fetch_adopted_pets(
    settings: &Settings,
    args: AdoptedAnimalsArgs,
) -> Result<Value, AppError> {
    let miles = args.miles.unwrap_or(settings.default_miles);
    let species = args.species.as_deref().unwrap_or(&settings.default_species);
    let postal_code = args
        .postal_code
        .as_deref()
        .unwrap_or(&settings.default_postal_code);

    // Assuming the 'adopted' endpoint mirrors 'available'
    let url = format!(
        "{}/public/animals/search/adopted/{}/haspic",
        settings.base_url, species
    );

    let body = json!({
        "data": {
            "filterRadius": {
                "miles": miles,
                "postalcode": postal_code
            }
        }
    });

    fetch_with_cache(settings, &url, "POST", Some(body)).await
}

// =========================================================================
// 3. MCP SERVER LOOP (JSON-RPC)
// =========================================================================

#[derive(Deserialize, Debug)]
struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Clone)]
struct AppState {
    settings: Settings,
    auth_token: Option<String>,
    sessions: SessionsMap,
}

#[derive(Deserialize)]
struct MessageParams {
    session_id: String,
}

async fn http_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // Auth check
    if let Some(token) = &state.auth_token {
        let auth_header = headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if auth_header != format!("Bearer {}", token) {
            warn!("Unauthorized access attempt");
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }
    }

    debug!("Received HTTP request: method={}", req.method);
    let response = process_mcp_request(req, &state.settings).await;

    if let Some(id) = response.0 {
        let mut output = json!({
            "jsonrpc": "2.0",
            "id": id,
        });
        match response.1 {
            Ok(res) => output["result"] = res,
            Err(err) => output["error"] = err,
        }
        Json(output).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    }
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel();
    let session_id = Uuid::new_v4().to_string();

    // Send initial endpoint event
    let endpoint_url = format!("/message?session_id={}", session_id);
    let _ = tx.send(Ok(Event::default().event("endpoint").data(endpoint_url)));

    state.sessions.write().await.insert(session_id.clone(), tx);

    let stream = UnboundedReceiverStream::new(rx);
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn message_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MessageParams>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let response = process_mcp_request(req, &state.settings).await;

    if let Some(id) = response.0 {
        let mut output = json!({
            "jsonrpc": "2.0",
            "id": id,
        });
        match response.1 {
            Ok(res) => output["result"] = res,
            Err(err) => output["error"] = err,
        }

        // Find session and send response via SSE
        if let Some(tx) = state.sessions.read().await.get(&params.session_id) {
            let _ = tx.send(Ok(Event::default()
                .event("message")
                .data(output.to_string())));
        }
    }

    StatusCode::ACCEPTED
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // 0. Initialize Logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rescue_groups_mcp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(io::stderr))
        .init();

    // 1. Load Settings
    let cli = Cli::parse();
    // Clone command to use after merge_configuration (which consumes cli)
    let command = cli.command.clone();
    let settings = merge_configuration(&cli)?;

    match command {
        Some(Commands::Server) | None => {
            // 2. Setup Stdio
            let stdin = io::stdin();
            let mut reader = stdin.lock();
            let mut line = String::new();

            info!("RescueGroups MCP Server running (Stdio)...");

            // 3. Main Loop
            loop {
                line.clear();
                if reader.read_line(&mut line)? == 0 {
                    break;
                } // EOF

                let req: JsonRpcRequest = match serde_json::from_str::<JsonRpcRequest>(&line) {
                    Ok(r) => {
                        debug!("Received request: method={}", r.method);
                        r
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON-RPC request: {}", e);
                        continue;
                    }
                };

                let response = process_mcp_request(req, &settings).await;

                if let Some(id) = response.0 {
                    let mut output = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                    });
                    match response.1 {
                        Ok(res) => output["result"] = res,
                        Err(err) => output["error"] = err,
                    }
                    println!("{}", output);
                    io::stdout().flush()?;
                }
            }
        }
        Some(Commands::Http(args)) => {
            let app_state = Arc::new(AppState {
                settings: settings.clone(),
                auth_token: args.auth_token,
                sessions: Arc::new(RwLock::new(HashMap::new())),
            });

            let app = Router::new()
                .route("/", post(http_handler))
                .route("/sse", get(sse_handler))
                .route("/message", post(message_handler))
                .with_state(app_state);

            let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
            info!("RescueGroups MCP Server running (HTTP + SSE) on {}", addr);

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }
        Some(Commands::Search(args)) => {
            print_output(fetch_pets(&settings, args).await, cli.json, |v| {
                format_animal_results(v)
            });
        }
        Some(Commands::ListSpecies) => {
            print_output(list_species(&settings).await, cli.json, |v| {
                format_species_results(v)
            });
        }
        Some(Commands::GetAnimal(args)) => {
            print_output(get_animal_details(&settings, args).await, cli.json, |v| {
                let animal_data = v.get("data").ok_or(AppError::NotFound)?;
                let animal = extract_single_item(animal_data).ok_or(AppError::NotFound)?;
                Ok(format_single_animal(animal))
            });
        }
        Some(Commands::GetContact(args)) => {
            print_output(get_contact_info(&settings, args).await, cli.json, |v| {
                format_contact_info(v)
            });
        }
        Some(Commands::Compare(args)) => {
            print_output(compare_animals(&settings, args).await, cli.json, |v| {
                format_comparison_table(v)
            });
        }
        Some(Commands::SearchOrgs(args)) => {
            print_output(search_organizations(&settings, args).await, cli.json, |v| {
                format_org_results(v)
            });
        }
        Some(Commands::GetOrg(args)) => {
            print_output(
                get_organization_details(&settings, args).await,
                cli.json,
                |v| {
                    let org_data = v.get("data").ok_or(AppError::NotFound)?;
                    let org = extract_single_item(org_data).ok_or(AppError::NotFound)?;
                    Ok(format_single_org(org))
                },
            );
        }
        Some(Commands::ListOrgAnimals(args)) => {
            print_output(list_org_animals(&settings, args).await, cli.json, |v| {
                format_animal_results(v)
            });
        }
        Some(Commands::ListAdopted(args)) => {
            print_output(fetch_adopted_pets(&settings, args).await, cli.json, |v| {
                format_animal_results(v)
            });
        }
        Some(Commands::ListBreeds(args)) => {
            let species = args.species.clone();
            print_output(list_breeds(&settings, args).await, cli.json, |v| {
                format_breed_results(v, &species)
            });
        }
        Some(Commands::ListMetadata(args)) => {
            let metadata_type = args.metadata_type.clone();
            print_output(list_metadata(&settings, args).await, cli.json, |v| {
                format_metadata_results(v, &metadata_type)
            });
        }
        Some(Commands::Generate(args)) => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();

            if let Some(shell) = args.shell {
                generate(shell, &mut cmd, bin_name, &mut io::stdout());
            }

            if let Some(ref man_dir) = args.man {
                let out_dir = Path::new(man_dir);
                if !out_dir.exists() {
                    fs::create_dir_all(out_dir)?;
                }
                Man::new(cmd)
                    .render(&mut fs::File::create(out_dir.join("rescue-groups-mcp.1"))?)?;
                info!("Man page generated in {}", man_dir);
            }

            if args.shell.is_none() && args.man.is_none() {
                warn!("Please specify --shell <SHELL> or --man <DIR>");
            }
        }
    }
    Ok(())
}

async fn handle_tool_call(
    name: &str,
    params: Option<Value>,
    settings: &Settings,
) -> Result<Value, AppError> {
    match name {
        "list_animals" => {
            let data = list_animals(settings).await?;
            let content = format_animal_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "list_species" => {
            let data = list_species(settings).await?;
            let content = format_species_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "list_metadata" => {
            let args: MetadataArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(MetadataArgs {
                metadata_type: "colors".to_string(),
            });

            let data = list_metadata(settings, args.clone()).await?;
            let content = format_metadata_results(&data, &args.metadata_type)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "list_breeds" => {
            let args: SpeciesArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(SpeciesArgs {
                species: settings.default_species.clone(),
            });

            let data = list_breeds(settings, args.clone()).await?;
            let content = format_breed_results(&data, &args.species)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "get_animal_details" => {
            let args: AnimalIdArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(AnimalIdArgs {
                animal_id: "0".to_string(),
            });

            let data = get_animal_details(settings, args).await?;
            let animal_data = data.get("data");
            match animal_data.and_then(|d| extract_single_item(d)) {
                Some(a) => {
                    Ok(json!({ "content": [{ "type": "text", "text": format_single_animal(a) }] }))
                }
                None => {
                    Err(AppError::NotFound)
                }
            }
        }
        "get_contact_info" => {
            let args: AnimalIdArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(AnimalIdArgs {
                animal_id: "0".to_string(),
            });

            let data = get_contact_info(settings, args).await?;
            let content = format_contact_info(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "compare_animals" => {
            let args: CompareArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(CompareArgs { animal_ids: vec![] });

            let data = compare_animals(settings, args).await?;
            let content = format_comparison_table(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "search_organizations" => {
            let args: OrgSearchArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(OrgSearchArgs {
                postal_code: None,
                miles: None,
            });

            let data = search_organizations(settings, args).await?;
            let content = format_org_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "get_organization_details" => {
            let args: OrgIdArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(OrgIdArgs {
                org_id: "0".to_string(),
            });

            let data = get_organization_details(settings, args).await?;
            let org_data = data.get("data");
            match org_data.and_then(|d| extract_single_item(d)) {
                Some(o) => {
                    Ok(json!({ "content": [{ "type": "text", "text": format_single_org(o) }] }))
                }
                None => {
                    Err(AppError::NotFound)
                }
            }
        }
        "list_org_animals" => {
            let args: OrgIdArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(OrgIdArgs {
                org_id: "0".to_string(),
            });

            let data = list_org_animals(settings, args).await?;
            let content = format_animal_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "search_adoptable_pets" => {
            let args: ToolArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(ToolArgs {
                postal_code: None,
                miles: None,
                species: None,
                breeds: None,
                sex: None,
                age: None,
                size: None,
                good_with_children: None,
                good_with_dogs: None,
                good_with_cats: None,
                house_trained: None,
                special_needs: None,
                sort_by: None,
            });

            let data = fetch_pets(settings, args).await?;
            let content = format_animal_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "list_adopted_animals" => {
            let args: AdoptedAnimalsArgs = serde_json::from_value(
                params.unwrap_or_default().get("arguments").cloned().unwrap_or_default(),
            )
            .unwrap_or(AdoptedAnimalsArgs {
                postal_code: None,
                miles: None,
                species: None,
            });

            let data = fetch_adopted_pets(settings, args).await?;
            let content = format_animal_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        _ => Err(AppError::NotFound),
    }
}

async fn process_mcp_request(req: JsonRpcRequest, settings: &Settings) -> (Option<Value>, Result<Value, Value>) {
    let response = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "rescue-groups-mcp", "version": env!("PROJECT_VERSION") }
        })),

        "notifications/initialized" => return (None, Ok(json!({}))), // Notification, no response

        "tools/list" => Ok(json!({
            "tools": [
// ... (rest of tools/list content)
                    {
                        "name": "list_animals",
                        "description": "List the most recent adoptable animals available globally.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "list_species",
                        "description": "List all animal species supported by the RescueGroups API.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "list_metadata",
                        "description": "List valid metadata values for animal attributes (colors, patterns, qualities).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "metadata_type": {
                                    "type": "string",
                                    "description": "The type of metadata to list (e.g., colors, patterns, qualities)"
                                }
                            },
                            "required": ["metadata_type"]
                        }
                    },
                    {
                        "name": "list_breeds",
                        "description": "List available breeds for a specific species.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "species": { "type": "string", "description": "Type of animal (e.g., dogs, cats, rabbits)" }
                            },
                            "required": ["species"]
                        }
                    },
                    {
                        "name": "get_animal_details",
                        "description": "Get detailed information about a specific animal by its ID.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "animal_id": { "type": "string", "description": "The unique ID of the animal." }
                            },
                            "required": ["animal_id"]
                        }
                    },
                    {
                        "name": "get_contact_info",
                        "description": "Get the primary contact method (email, phone, organization) for a specific animal.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "animal_id": { "type": "string", "description": "The unique ID of the animal." }
                            },
                            "required": ["animal_id"]
                        }
                    },
                    {
                        "name": "compare_animals",
                        "description": "Compare up to 5 animals side-by-side by their IDs.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "animal_ids": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "List of animal IDs to compare (max 5)."
                                }
                            },
                            "required": ["animal_ids"]
                        }
                    },
                    {
                        "name": "get_organization_details",
                        "description": "Get detailed information about a specific rescue organization by its ID.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "org_id": { "type": "string", "description": "The unique ID of the organization." }
                            },
                            "required": ["org_id"]
                        }
                    },
                    {
                        "name": "list_org_animals",
                        "description": "List all animals available for adoption at a specific organization.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "org_id": { "type": "string", "description": "The unique ID of the organization." }
                            },
                            "required": ["org_id"]
                        }
                    },
                    {
                        "name": "search_organizations",
                        "description": "Search for animal rescue organizations and shelters by location.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "postal_code": { "type": "string", "description": "Zip code (e.g. 90210)" },
                                "miles": { "type": "integer", "description": "Search radius (default 50)" }
                            }
                        }
                    },
                    {
                        "name": "search_adoptable_pets",
                        "description": "Search for adoptable pets (dogs, cats, etc) by location and various traits.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "postal_code": { "type": "string", "description": "Zip code (e.g. 90210)" },
                                "species": { "type": "string", "description": "Type of animal (dogs, cats, rabbits)" },
                                "breeds": { "type": "string", "description": "Specific breed name (e.g. Golden Retriever)" },
                                "miles": { "type": "integer", "description": "Search radius (default 50)" },
                                "sex": { "type": "string", "description": "Sex of the animal (Male, Female)" },
                                "age": { "type": "string", "description": "Age group (Baby, Young, Adult, Senior)" },
                                "size": { "type": "string", "description": "Size group (Small, Medium, Large, X-Large)" },
                                "good_with_children": { "type": "boolean", "description": "Whether the pet is good with children." },
                                "good_with_dogs": { "type": "boolean", "description": "Whether the pet is good with other dogs." },
                                "good_with_cats": { "type": "boolean", "description": "Whether the pet is good with cats." },
                                "house_trained": { "type": "boolean", "description": "Whether the pet is house trained." },
                                "special_needs": { "type": "boolean", "description": "Whether the pet has special needs." },
                                "sort_by": {
                                    "type": "string",
                                    "enum": ["Newest", "Distance", "Random"],
                                    "description": "Sort order for results."
                                }
                            }
                        }
                    },
                    {
                        "name": "list_adopted_animals",
                        "description": "List recently adopted animals (Success Stories) to see happy endings near you.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "postal_code": { "type": "string", "description": "Zip code (e.g. 90210)" },
                                "species": { "type": "string", "description": "Type of animal (dogs, cats, rabbits)" },
                                "miles": { "type": "integer", "description": "Search radius (default 50)" }
                            }
                        }
                    }
            ]
        })),

        "tools/call" => {
            if let Some(params) = req.params {
                let name = params["name"].as_str().unwrap_or("").to_string();
                match handle_tool_call(&name, Some(params), settings).await {
                    Ok(val) => Ok(val),
                    Err(e) => {
                        warn!("Tool call '{}' failed: {}", name, e);
                        Err(e.to_json_rpc_error())
                    }
                }
            } else {
                 Err(json!({ "code": -32602, "message": "Missing parameters" }))
            }
        },

        "ping" => Ok(json!({})),

        _ => Err(json!({ "code": -32601, "message": "Method not found" })),
    };

    (req.id, response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_configuration_defaults() {
        let cli = Cli {
            api_key: Some("test_key".to_string()),
            config: "non_existent.toml".to_string(),
            command: None,
            json: false,
        };
        let settings = merge_configuration(&cli).unwrap();
        assert_eq!(settings.api_key, "test_key");
        assert_eq!(settings.default_postal_code, "90210");
        assert_eq!(settings.default_miles, 50);
        assert_eq!(settings.default_species, "dogs");
    }

    #[test]
    fn test_format_single_animal() {
        let animal = json!({
            "attributes": {
                "name": "Buddy",
                "breedString": "Golden Retriever",
                "descriptionText": "A friendly dog.",
                "sex": "Male",
                "ageGroup": "Young",
                "sizeGroup": "Large",
                "url": "https://example.com/buddy",
                "orgsAnimalsPictures": [
                    { "urlSecureFullsize": "https://example.com/buddy.jpg" }
                ]
            }
        });
        let result = format_single_animal(&animal);
        assert!(result.contains("# Buddy"));
        assert!(result.contains("**Breed:** Golden Retriever"));
        assert!(result.contains("![Buddy](https://example.com/buddy.jpg)"));
        assert!(result.contains("A friendly dog."));
    }

    #[test]
    fn test_format_animal_results() {
        let data = json!({
            "data": [
                {
                    "attributes": {
                        "name": "Buddy",
                        "breedString": "Golden Retriever",
                        "url": "https://example.com/buddy"
                    }
                }
            ]
        });
        let result = format_animal_results(&data).unwrap();
        assert!(result.contains("### [Buddy](https://example.com/buddy)"));
        assert!(result.contains("**Breed:** Golden Retriever"));
    }

    #[test]
    fn test_format_animal_results_empty() {
        let data = json!({ "data": [] });
        let result = format_animal_results(&data).unwrap();
        assert_eq!(result, "No adoptable animals found.");
    }

    #[tokio::test]
    async fn test_list_breeds_mock() {
        let mut server = mockito::Server::new_async().await;

        // Mock species list
        let species_body = json!({
            "data": [
                {
                    "id": "8",
                    "attributes": {
                        "singular": "Dog",
                        "plural": "Dogs"
                    }
                }
            ]
        });
        let _m_species = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&species_body).unwrap())
            .create_async()
            .await;

        let breeds_body = json!({
            "data": [
                { "attributes": { "name": "Labrador" } },
                { "attributes": { "name": "Beagle" } }
            ]
        });

        let _m_breeds = server
            .mock("GET", "/public/animals/species/8/breeds")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&breeds_body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = SpeciesArgs {
            species: "dogs".to_string(),
        };
        let value = list_breeds(&settings, args).await.unwrap();
        let result = format_breed_results(&value, "dogs").unwrap();

        assert!(result.contains("### Breeds for dogs"));
        assert!(result.contains("Labrador"));
        assert!(result.contains("Beagle"));
    }

    #[tokio::test]
    async fn test_list_animals_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                {
                    "attributes": {
                        "name": "Buddy",
                        "breedString": "Golden Retriever",
                        "url": "https://example.com/buddy"
                    }
                }
            ]
        });

        let _m = server
            .mock("GET", "/public/animals")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let value = list_animals(&settings).await.unwrap();
        let result = format_animal_results(&value).unwrap();
        assert!(result.contains("### [Buddy](https://example.com/buddy)"));
    }

    #[tokio::test]
    async fn test_get_animal_details_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": {
                "attributes": {
                    "name": "Buddy",
                    "breedString": "Golden Retriever",
                    "descriptionText": "A friendly dog.",
                    "sex": "Male",
                    "ageGroup": "Young",
                    "sizeGroup": "Large",
                    "url": "https://example.com/buddy",
                    "orgsAnimalsPictures": []
                }
            }
        });

        let _m = server
            .mock("GET", "/public/animals/123")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = AnimalIdArgs {
            animal_id: "123".to_string(),
        };
        let value = get_animal_details(&settings, args).await.unwrap();
        let animal = value.get("data").unwrap();
        let result = format_single_animal(animal);
        assert!(result.contains("# Buddy"));
        assert!(result.contains("A friendly dog."));
    }

    #[tokio::test]
    async fn test_search_organizations_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                {
                    "id": "1",
                    "attributes": {
                        "name": "Local Rescue",
                        "city": "Los Angeles",
                        "state": "CA",
                        "email": "info@localrescue.org",
                        "url": "https://localrescue.org"
                    }
                }
            ]
        });

        let _m = server
            .mock("POST", "/public/orgs/search")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = OrgSearchArgs {
            postal_code: None,
            miles: None,
        };
        let value = search_organizations(&settings, args).await.unwrap();
        let result = format_org_results(&value).unwrap();
        assert!(result.contains("### Local Rescue"));
        assert!(result.contains("**Location:** Los Angeles, CA"));
    }

    #[tokio::test]
    async fn test_get_organization_details_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": {
                "id": "1",
                "attributes": {
                    "name": "Local Rescue",
                    "about": "A great shelter.",
                    "street": "123 Main St",
                    "city": "Los Angeles",
                    "state": "CA",
                    "postalcode": "90210",
                    "email": "info@localrescue.org",
                    "phone": "555-1234",
                    "url": "https://localrescue.org",
                    "facebookUrl": "https://facebook.com/localrescue"
                }
            }
        });

        let _m = server
            .mock("GET", "/public/orgs/1")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = OrgIdArgs {
            org_id: "1".to_string(),
        };
        let value = get_organization_details(&settings, args).await.unwrap();
        let org = value.get("data").unwrap();
        let result = format_single_org(org);
        assert!(result.contains("# Local Rescue"));
        assert!(result.contains("A great shelter."));
        assert!(result.contains("123 Main St"));
    }

    #[tokio::test]
    async fn test_list_org_animals_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                {
                    "attributes": {
                        "name": "OrgPet",
                        "breedString": "Mix",
                        "url": "https://example.com/orgpet"
                    }
                }
            ]
        });

        let _m = server
            .mock("GET", "/public/orgs/1/animals/search/available")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = OrgIdArgs {
            org_id: "1".to_string(),
        };
        let value = list_org_animals(&settings, args).await.unwrap();
        let result = format_animal_results(&value).unwrap();
        assert!(result.contains("### [OrgPet](https://example.com/orgpet)"));
    }

    #[tokio::test]
    async fn test_list_species_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                { "attributes": { "singular": "Dog" } },
                { "attributes": { "singular": "Cat" } }
            ]
        });

        let _m = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let value = list_species(&settings).await.unwrap();
        let result = format_species_results(&value).unwrap();
        assert!(result.contains("### Supported Species"));
        assert!(result.contains("Dog"));
        assert!(result.contains("Cat"));
    }

    #[tokio::test]
    async fn test_list_metadata_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                { "attributes": { "name": "Black" } },
                { "attributes": { "name": "White" } }
            ]
        });

        let _m = server
            .mock("GET", "/public/animals/colors")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = MetadataArgs {
            metadata_type: "colors".to_string(),
        };
        let value = list_metadata(&settings, args).await.unwrap();
        let result = format_metadata_results(&value, "colors").unwrap();
        assert!(result.contains("### Supported colors"));
        assert!(result.contains("Black"));
        assert!(result.contains("White"));
    }

    #[tokio::test]
    async fn test_search_advanced_filters_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                {
                    "attributes": {
                        "name": "FilteredPet",
                        "breedString": "Mix",
                        "url": "https://example.com/filtered"
                    }
                }
            ]
        });

        let m = server
            .mock("POST", "/public/animals/search/available/dogs/haspic")
            .match_body(mockito::Matcher::Json(json!({
                "data": {
                    "filterRadius": {
                        "miles": 50,
                        "postalcode": "90210"
                    },
                    "filters": [
                        {
                            "fieldName": "animals.sex",
                            "operation": "equal",
                            "criteria": "Female"
                        },
                        {
                            "fieldName": "animals.ageGroup",
                            "operation": "equal",
                            "criteria": "Senior"
                        }
                    ]
                }
            })))
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = ToolArgs {
            postal_code: Some("90210".to_string()),
            miles: Some(50),
            species: Some("dogs".to_string()),
            breeds: None,
            sex: Some("Female".to_string()),
            age: Some("Senior".to_string()),
            size: None,
            good_with_children: None,
            good_with_dogs: None,
            good_with_cats: None,
            house_trained: None,
            special_needs: None,
            sort_by: None,
        };

        let value = fetch_pets(&settings, args).await.unwrap();
        let result = format_animal_results(&value).unwrap();
        assert!(result.contains("FilteredPet"));
        m.assert_async().await;
    }

    #[tokio::test]
    async fn test_search_behavior_filters_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                {
                    "attributes": {
                        "name": "GoodBoy",
                        "breedString": "Mix",
                        "url": "https://example.com/goodboy"
                    }
                }
            ]
        });

        let m = server
            .mock("POST", "/public/animals/search/available/dogs/haspic")
            .match_body(mockito::Matcher::Json(json!({
                "data": {
                    "filterRadius": {
                        "miles": 50,
                        "postalcode": "90210"
                    },
                    "filters": [
                        {
                            "fieldName": "animals.isGoodWithChildren",
                            "operation": "equal",
                            "criteria": "Yes"
                        },
                        {
                            "fieldName": "animals.isHouseTrained",
                            "operation": "equal",
                            "criteria": "Yes"
                        },
                        {
                            "fieldName": "animals.isSpecialNeeds",
                            "operation": "equal",
                            "criteria": "No"
                        }
                    ]
                }
            })))
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = ToolArgs {
            postal_code: Some("90210".to_string()),
            miles: Some(50),
            species: Some("dogs".to_string()),
            breeds: None,
            sex: None,
            age: None,
            size: None,
            good_with_children: Some(true),
            good_with_dogs: None,
            good_with_cats: None,
            house_trained: Some(true),
            special_needs: Some(false),
            sort_by: None,
        };

        let value = fetch_pets(&settings, args).await.unwrap();
        let result = format_animal_results(&value).unwrap();
        assert!(result.contains("GoodBoy"));
        m.assert_async().await;
    }

    #[tokio::test]
    async fn test_search_sorting_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                {
                    "attributes": {
                        "name": "NewestPet",
                        "breedString": "Mix",
                        "url": "https://example.com/newest"
                    }
                }
            ]
        });

        // Verify that the query parameter is appended to the URL
        let m = server
            .mock(
                "POST",
                "/public/animals/search/available/dogs/haspic?sort=-animals.createdDate",
            )
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = ToolArgs {
            postal_code: Some("90210".to_string()),
            miles: Some(50),
            species: Some("dogs".to_string()),
            breeds: None,
            sex: None,
            age: None,
            size: None,
            good_with_children: None,
            good_with_dogs: None,
            good_with_cats: None,
            house_trained: None,
            special_needs: None,
            sort_by: Some("Newest".to_string()),
        };

        let value = fetch_pets(&settings, args).await.unwrap();
        let result = format_animal_results(&value).unwrap();
        assert!(result.contains("NewestPet"));
        m.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_adopted_animals_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": [
                {
                    "attributes": {
                        "name": "HappyTail",
                        "breedString": "Mix",
                        "url": "https://example.com/happytail"
                    }
                }
            ]
        });

        let m = server
            .mock("POST", "/public/animals/search/adopted/dogs/haspic")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = AdoptedAnimalsArgs {
            postal_code: Some("90210".to_string()),
            miles: Some(50),
            species: Some("dogs".to_string()),
        };

        let value = fetch_adopted_pets(&settings, args).await.unwrap();
        let result = format_animal_results(&value).unwrap();
        assert!(result.contains("HappyTail"));
        m.assert_async().await;
    }

    #[tokio::test]
    async fn test_caching_behavior() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": {
                "attributes": {
                    "name": "CachedPet",
                    "breedString": "Mix",
                    "descriptionText": "Cached",
                    "sex": "Unknown",
                    "ageGroup": "Unknown",
                    "sizeGroup": "Unknown",
                    "url": "",
                    "orgsAnimalsPictures": []
                }
            }
        });

        // Mock ONLY ONE call
        let m = server
            .mock("GET", "/public/animals/123")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .expect(1) // Expect exactly one call
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = AnimalIdArgs {
            animal_id: "123".to_string(),
        };

        // First call - should hit the mock
        let _ = get_animal_details(&settings, args.clone()).await.unwrap();

        // Second call - should hit the cache, NOT the mock
        let _ = get_animal_details(&settings, args).await.unwrap();

        m.assert_async().await;
    }

    #[tokio::test]
    async fn test_compare_animals_mock() {
        let mut server = mockito::Server::new_async().await;

        // Animal 1
        let body1 = json!({
            "data": {
                "attributes": {
                    "name": "Pet1",
                    "breedString": "Breed1",
                    "sex": "Male",
                    "url": "http://p1"
                }
            }
        });
        let _m1 = server
            .mock("GET", "/public/animals/1")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body1).unwrap())
            .create_async()
            .await;

        // Animal 2
        let body2 = json!({
            "data": {
                "attributes": {
                    "name": "Pet2",
                    "breedString": "Breed2",
                    "sex": "Female",
                    "url": "http://p2"
                }
            }
        });
        let _m2 = server
            .mock("GET", "/public/animals/2")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body2).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = CompareArgs {
            animal_ids: vec!["1".to_string(), "2".to_string()],
        };

        let value = compare_animals(&settings, args).await.unwrap();
        let result = format_comparison_table(&value).unwrap();

        assert!(result.contains("Pet1"));
        assert!(result.contains("Pet2"));
        assert!(result.contains("Breed1"));
        assert!(result.contains("Breed2"));
        assert!(result.contains("Male"));
        assert!(result.contains("Female"));
    }

    #[tokio::test]
    async fn test_get_contact_info_mock() {
        let mut server = mockito::Server::new_async().await;
        let body = json!({
            "data": {
                "attributes": {
                    "name": "Buddy",
                    "url": "https://buddy-link"
                }
            },
            "included": [
                {
                    "type": "orgs",
                    "attributes": {
                        "name": "Rescue Org",
                        "email": "contact@rescue.org",
                        "phone": "555-5555",
                        "city": "Shelter City",
                        "state": "ST",
                        "url": "https://rescue.org"
                    }
                }
            ]
        });

        let _m = server
            .mock("GET", "/public/animals/123?include=orgs")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(serde_json::to_string(&body).unwrap())
            .create_async()
            .await;

        let settings = Settings {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            default_postal_code: "90210".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            cache: Arc::new(moka::future::Cache::builder().build()),
        };

        let args = AnimalIdArgs {
            animal_id: "123".to_string(),
        };

        let value = get_contact_info(&settings, args).await.unwrap();
        let result = format_contact_info(&value).unwrap();

        assert!(result.contains("## Contact Information for Buddy"));
        assert!(result.contains("**Organization:** Rescue Org"));
        assert!(result.contains("**Email:** contact@rescue.org"));
        assert!(result.contains("**Phone:** 555-5555"));
        assert!(result.contains("**Location:** Shelter City, ST"));
        assert!(result.contains(
            "[View adoption application or more info on RescueGroups](https://buddy-link)"
        ));
    }

    #[test]
    fn test_app_error_display() {
        let err = AppError::ApiError("Not Found".to_string());
        assert_eq!(format!("{}", err), "API Error: Not Found");

        let err = AppError::ConfigError("Missing key".to_string());
        assert_eq!(format!("{}", err), "Configuration Error: Missing key");

        let err = AppError::NotFound;
        assert_eq!(format!("{}", err), "Resource Not Found");

        let err = AppError::Internal("test".to_string());
        assert_eq!(format!("{}", err), "Internal Error: test");
    }

    #[test]
    fn test_app_error_conversions() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));

        let s = "test error";
        let app_err: AppError = s.into();
        assert!(matches!(app_err, AppError::Internal(_)));
    }

    #[test]
    fn test_json_rpc_error_mapping() {
        let err = AppError::NotFound;
        let json_err = err.to_json_rpc_error();
        assert_eq!(json_err["code"], -32004);
        assert_eq!(json_err["message"], "Resource Not Found");

        let err = AppError::Internal("server error".to_string());
        let json_err = err.to_json_rpc_error();
        assert_eq!(json_err["code"], -32603);
        assert!(json_err["message"]
            .as_str()
            .unwrap()
            .contains("Internal Error"));

        let err = AppError::ValidationError("invalid".to_string());
        let json_err = err.to_json_rpc_error();
        assert_eq!(json_err["code"], -32602);
        assert!(json_err["message"]
            .as_str()
            .unwrap()
            .contains("Validation Error"));

        let err = AppError::ApiError("upstream".to_string());
        let json_err = err.to_json_rpc_error();
        assert_eq!(json_err["code"], -32005);
        assert!(json_err["message"].as_str().unwrap().contains("API Error"));
    }
}
