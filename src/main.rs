use clap::Parser;
use serde::Deserialize;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;

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
#[command(author, version, about)]
struct Cli {
    #[arg(short, long)]
    miles: Option<u32>,
    #[arg(short, long)]
    species: Option<String>,
    #[arg(long, env = "RESCUE_GROUPS_API_KEY", hide_env_values = true)]
    api_key: Option<String>,
    #[arg(long, default_value = "config.toml")]
    config: String,
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

fn merge_configuration(cli: Cli) -> Result<Settings, Box<dyn Error>> {
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
        default_miles: cli
            .miles
            .or(file_config.as_ref().and_then(|c| c.miles))
            .unwrap_or(50),
        default_species: cli
            .species
            .or(file_config.as_ref().and_then(|c| c.species.clone()))
            .unwrap_or_else(|| "dogs".to_string()),
        cache: Arc::new(cache),
    })
}

async fn fetch_with_cache(
    settings: &Settings,
    url: &str,
    method: &str,
    body: Option<Value>,
) -> Result<Value, Box<dyn Error>> {
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
            return Err("Not Found".into());
        }
        return Err(format!("API Error: {}", response.status()).into());
    }

    let data: Value = response.json().await?;
    settings.cache.insert(cache_key, data.clone()).await;
    Ok(data)
}

// =========================================================================
// 2. CORE LOGIC (The Search Function)
// =========================================================================

#[derive(Deserialize, Clone)]
struct ToolArgs {
    postal_code: Option<String>,
    miles: Option<u32>,
    species: Option<String>,
    sex: Option<String>,
    age: Option<String>,
    size: Option<String>,
    good_with_children: Option<bool>,
    good_with_dogs: Option<bool>,
    good_with_cats: Option<bool>,
    house_trained: Option<bool>,
    special_needs: Option<bool>,
    sort_by: Option<String>,
}

#[derive(Deserialize, Clone)]
struct AnimalIdArgs {
    animal_id: String,
}

#[derive(Deserialize, Clone)]
struct SpeciesArgs {
    species: String,
}

#[derive(Deserialize, Clone)]
struct OrgSearchArgs {
    postal_code: Option<String>,
    miles: Option<u32>,
}

#[derive(Deserialize, Clone)]
struct OrgIdArgs {
    org_id: String,
}

#[derive(Deserialize, Clone)]
struct AdoptedAnimalsArgs {
    postal_code: Option<String>,
    miles: Option<u32>,
    species: Option<String>,
}

#[derive(Deserialize, Clone)]
struct MetadataArgs {
    metadata_type: String,
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

fn format_animal_results(data: &Value) -> Result<String, Box<dyn Error>> {
    let animals = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("No data found")?;

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

fn format_species_results(data: &Value) -> Result<String, Box<dyn Error>> {
    let species = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("No species data found")?;

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

fn format_metadata_results(data: &Value, metadata_type: &str) -> Result<String, Box<dyn Error>> {
    let items = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("No metadata found")?;

    if items.is_empty() {
        return Ok(format!("No {} found.", metadata_type));
    }

    let mut names: Vec<String> = items
        .iter()
        .filter_map(|i| i["attributes"]["name"].as_str().map(|n| n.to_string()))
        .collect();

    names.sort();

    Ok(format!("### Supported {}\n\n{}", metadata_type, names.join("\n")))
}

fn format_org_results(data: &Value) -> Result<String, Box<dyn Error>> {
    let orgs = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("No organization data found")?;

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

async fn list_breeds(settings: &Settings, args: SpeciesArgs) -> Result<String, Box<dyn Error>> {
    let url = format!(
        "{}/public/animals/species/{}/breeds",
        settings.base_url, args.species
    );
    let data = fetch_with_cache(settings, &url, "GET", None).await?;
    let breeds = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("No breeds found")?;

    if breeds.is_empty() {
        return Ok(format!("No breeds found for species '{}'.", args.species));
    }

    let mut breed_names: Vec<String> = breeds
        .iter()
        .filter_map(|b| b["attributes"]["name"].as_str().map(|s| s.to_string()))
        .collect();

    breed_names.sort();

    Ok(format!(
        "### Breeds for {}\n\n{}",
        args.species,
        breed_names.join("\n")
    ))
}

async fn list_species(settings: &Settings) -> Result<String, Box<dyn Error>> {

    let url = format!("{}/public/animals/species", settings.base_url);

    let data = fetch_with_cache(settings, &url, "GET", None).await?;

    format_species_results(&data)

}



async fn list_metadata(settings: &Settings, args: MetadataArgs) -> Result<String, Box<dyn Error>> {

    let url = format!("{}/public/animals/{}", settings.base_url, args.metadata_type);

    let data = fetch_with_cache(settings, &url, "GET", None).await?;

    format_metadata_results(&data, &args.metadata_type)

}



async fn list_animals(settings: &Settings) -> Result<String, Box<dyn Error>> {


    let url = format!("{}/public/animals", settings.base_url);
    let data = fetch_with_cache(settings, &url, "GET", None).await?;
    format_animal_results(&data)
}

async fn get_animal_details(
    settings: &Settings,
    args: AnimalIdArgs,
) -> Result<String, Box<dyn Error>> {
    let url = format!("{}/public/animals/{}", settings.base_url, args.animal_id);
    let data = match fetch_with_cache(settings, &url, "GET", None).await {
        Ok(d) => d,
        Err(e) if e.to_string() == "Not Found" => {
            return Ok(format!("Animal with ID {} not found.", args.animal_id));
        }
        Err(e) => return Err(e),
    };
    let animal = data.get("data").ok_or("No animal data found")?;

    Ok(format_single_animal(animal))
}

async fn search_organizations(
    settings: &Settings,
    args: OrgSearchArgs,
) -> Result<String, Box<dyn Error>> {
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

    let data = fetch_with_cache(settings, &url, "POST", Some(body)).await?;
    format_org_results(&data)
}

async fn get_organization_details(
    settings: &Settings,
    args: OrgIdArgs,
) -> Result<String, Box<dyn Error>> {
    let url = format!("{}/public/orgs/{}", settings.base_url, args.org_id);

    let data = match fetch_with_cache(settings, &url, "GET", None).await {
        Ok(d) => d,
        Err(e) if e.to_string() == "Not Found" => {
            return Ok(format!("Organization with ID {} not found.", args.org_id));
        }
        Err(e) => return Err(e),
    };
    let org = data.get("data").ok_or("No organization data found")?;

    Ok(format_single_org(org))
}

async fn list_org_animals(settings: &Settings, args: OrgIdArgs) -> Result<String, Box<dyn Error>> {
    let url = format!(
        "{}/public/orgs/{}/animals/search/available",
        settings.base_url, args.org_id
    );
    let data = match fetch_with_cache(settings, &url, "GET", None).await {
        Ok(d) => d,
        Err(e) if e.to_string() == "Not Found" => {
            return Ok(format!(
                "Organization with ID {} not found or has no available animals.",
                args.org_id
            ));
        }
        Err(e) => return Err(e),
    };
    format_animal_results(&data)
}

async fn fetch_pets(settings: &Settings, args: ToolArgs) -> Result<String, Box<dyn Error>> {
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

    let data = fetch_with_cache(settings, &url, "POST", Some(body)).await?;

    
    let animals = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("No data found")?;

    if animals.is_empty() {
        return Ok(format!(
            "No adoptable {} found within {} miles of {}.",
            species, miles, postal_code
        ));
    }

    format_animal_results(&data)
}

async fn fetch_adopted_pets(
    settings: &Settings,
    args: AdoptedAnimalsArgs,
) -> Result<String, Box<dyn Error>> {
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

    let data = match fetch_with_cache(settings, &url, "POST", Some(body)).await {
        Ok(d) => d,
        Err(e) => return Err(format!("Could not fetch adopted pets (endpoint might not exist): {}", e).into()),
    };

    let animals = data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("No data found")?;

    if animals.is_empty() {
        return Ok(format!(
            "No recently adopted {} found within {} miles of {}.",
            species, miles, postal_code
        ));
    }

    format_animal_results(&data)
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Load Settings
    let cli = Cli::parse();
    let settings = merge_configuration(cli)?;

    // 2. Setup Stdio
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

    eprintln!("RescueGroups MCP Server running...");

    // 3. Main Loop
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        } // EOF

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue, // Ignore malformed lines
        };

        let response = match req.method.as_str() {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "rescue-groups-mcp", "version": "0.1.0" }
            }),

            "notifications/initialized" => continue,

            "tools/list" => json!({
                "tools": [
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
            }),

            "tools/call" => {
                if let Some(params) = req.params {
                    let name = params["name"].as_str().unwrap_or("");
                    match name {
                        "list_animals" => match list_animals(&settings).await {
                            Ok(content) => {
                                json!({ "content": [{ "type": "text", "text": content }] })
                            }
                            Err(e) => {
                                json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                            }
                        },
                        "list_species" => match list_species(&settings).await {
                            Ok(content) => {
                                json!({ "content": [{ "type": "text", "text": content }] })
                            }
                            Err(e) => {
                                json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                            }
                        },
                        "list_metadata" => {
                            let args: MetadataArgs = serde_json::from_value(params["arguments"].clone())
                                .unwrap_or(MetadataArgs {
                                    metadata_type: "colors".to_string(),
                                });

                            match list_metadata(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        },
                        "list_breeds" => {
                            let args: SpeciesArgs = serde_json::from_value(
                                params["arguments"].clone(),
                            )
                            .unwrap_or(SpeciesArgs {
                                species: settings.default_species.clone(),
                            });

                            match list_breeds(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        }
                        "get_animal_details" => {
                            let args: AnimalIdArgs = serde_json::from_value(
                                params["arguments"].clone(),
                            )
                            .unwrap_or(AnimalIdArgs {
                                animal_id: "0".to_string(),
                            });

                            match get_animal_details(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        }
                        "search_organizations" => {
                            let args: OrgSearchArgs = serde_json::from_value(
                                params["arguments"].clone(),
                            )
                            .unwrap_or(OrgSearchArgs {
                                postal_code: None,
                                miles: None,
                            });

                            match search_organizations(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        }
                        "get_organization_details" => {
                            let args: OrgIdArgs = serde_json::from_value(
                                params["arguments"].clone(),
                            )
                            .unwrap_or(OrgIdArgs {
                                org_id: "0".to_string(),
                            });

                            match get_organization_details(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        }
                        "list_org_animals" => {
                            let args: OrgIdArgs = serde_json::from_value(
                                params["arguments"].clone(),
                            )
                            .unwrap_or(OrgIdArgs {
                                org_id: "0".to_string(),
                            });

                            match list_org_animals(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        }
                        "search_adoptable_pets" => {
                            let args: ToolArgs = serde_json::from_value(
                                params["arguments"].clone(),
                            )
                            .unwrap_or(ToolArgs {
                                postal_code: None,
                                miles: None,
                                species: None,
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

                            match fetch_pets(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        }
                        "list_adopted_animals" => {
                            let args: AdoptedAnimalsArgs = serde_json::from_value(
                                params["arguments"].clone(),
                            )
                            .unwrap_or(AdoptedAnimalsArgs {
                                postal_code: None,
                                miles: None,
                                species: None,
                            });

                            match fetch_adopted_pets(&settings, args).await {
                                Ok(content) => {
                                    json!({ "content": [{ "type": "text", "text": content }] })
                                }
                                Err(e) => {
                                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                                }
                            }
                        }
                        _ => {
                            json!({ "content": [{ "type": "text", "text": "Tool not found" }], "isError": true })
                        }
                    }
                } else {
                    json!({ "isError": true })
                }
            }

            "ping" => json!({}),

            _ => json!({ "error": { "code": -32601, "message": "Method not found" } }),
        };

        if let Some(id) = req.id {
            let output = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": response
            });
            println!("{}", output);
            io::stdout().flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_configuration_defaults() {
        let cli = Cli {
            miles: None,
            species: None,
            api_key: Some("test_key".to_string()),
            config: "non_existent.toml".to_string(),
        };
        let settings = merge_configuration(cli).unwrap();
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
        let body = json!({
            "data": [
                { "attributes": { "name": "Labrador" } },
                { "attributes": { "name": "Beagle" } }
            ]
        });

        let _m = server
            .mock("GET", "/public/animals/species/dogs/breeds")
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

        let args = SpeciesArgs {
            species: "dogs".to_string(),
        };
        let result = list_breeds(&settings, args).await.unwrap();

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

        let result = list_animals(&settings).await.unwrap();
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
        let result = get_animal_details(&settings, args).await.unwrap();
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
        let result = search_organizations(&settings, args).await.unwrap();
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
        let result = get_organization_details(&settings, args).await.unwrap();
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
        let result = list_org_animals(&settings, args).await.unwrap();
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

        let result = list_species(&settings).await.unwrap();
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

        let _m = server.mock("GET", "/public/animals/colors")
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

        let args = MetadataArgs { metadata_type: "colors".to_string() };
        let result = list_metadata(&settings, args).await.unwrap();
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

        let m = server.mock("POST", "/public/animals/search/available/dogs/haspic")
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

        let result = fetch_pets(&settings, args).await.unwrap();
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

        let m = server.mock("POST", "/public/animals/search/available/dogs/haspic")
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

        let result = fetch_pets(&settings, args).await.unwrap();
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
        let m = server.mock("POST", "/public/animals/search/available/dogs/haspic?sort=-animals.createdDate")
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

        let result = fetch_pets(&settings, args).await.unwrap();
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

        let m = server.mock("POST", "/public/animals/search/adopted/dogs/haspic")
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

        let result = fetch_adopted_pets(&settings, args).await.unwrap();
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
}
