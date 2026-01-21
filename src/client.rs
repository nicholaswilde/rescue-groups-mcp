use crate::cli::{
    AdoptedAnimalsArgs, AnimalIdArgs, CompareArgs, MetadataArgs, OrgIdArgs, OrgSearchArgs,
    SpeciesArgs, ToolArgs,
};
use crate::config::Settings;
use crate::error::AppError;
use crate::fmt::extract_single_item;
use serde_json::{json, Value};
use tokio::task::JoinSet;

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

    // Check rate limit before making the request
    // Wait until a spot is available
    settings.limiter.until_ready().await;

    let client = reqwest::Client::builder()
        .timeout(settings.timeout)
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build client: {}", e)))?;

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
        return Err(AppError::ApiError(format!(
            "API Error: {}",
            response.status()
        )));
    }

    let data: Value = response.json().await?;
    settings.cache.insert(cache_key, data.clone()).await;
    Ok(data)
}

pub async fn list_breeds(settings: &Settings, args: SpeciesArgs) -> Result<Value, AppError> {
    let species_id = if args.species.chars().all(char::is_numeric) {
        args.species
    } else {
        // Try to resolve name to ID
        let species_list = list_species(settings).await?;
        let data =
            species_list
                .get("data")
                .and_then(|d| d.as_array())
                .ok_or(AppError::Internal(
                    "Failed to fetch species list for resolution".to_string(),
                ))?;

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

pub async fn list_species(settings: &Settings) -> Result<Value, AppError> {
    let url = format!("{}/public/animals/species", settings.base_url);
    fetch_with_cache(settings, &url, "GET", None).await
}

pub async fn list_metadata(settings: &Settings, args: MetadataArgs) -> Result<Value, AppError> {
    let url = format!(
        "{}/public/animals/{}",
        settings.base_url, args.metadata_type
    );
    fetch_with_cache(settings, &url, "GET", None).await
}

pub async fn list_metadata_types() -> Result<Value, AppError> {
    let types = vec![
        "breeds",
        "colors",
        "patterns",
        "species",
        "statuses",
        "qualities",
    ];
    Ok(json!({ "data": types }))
}

pub async fn list_animals(settings: &Settings) -> Result<Value, AppError> {
    let url = format!("{}/public/animals", settings.base_url);
    fetch_with_cache(settings, &url, "GET", None).await
}

pub async fn get_animal_details(
    settings: &Settings,
    args: AnimalIdArgs,
) -> Result<Value, AppError> {
    let url = format!("{}/public/animals/{}", settings.base_url, args.animal_id);
    fetch_with_cache(settings, &url, "GET", None).await
}

pub async fn get_contact_info(settings: &Settings, args: AnimalIdArgs) -> Result<Value, AppError> {
    let url = format!(
        "{}/public/animals/{}?include=orgs",
        settings.base_url, args.animal_id
    );
    fetch_with_cache(settings, &url, "GET", None).await
}

pub async fn compare_animals(settings: &Settings, args: CompareArgs) -> Result<Value, AppError> {
    let mut set = JoinSet::new();
    // Deduplicate and limit
    let mut ids = args.animal_ids.clone();
    ids.sort();
    ids.dedup();

    for id in ids.iter().take(5) {
        let settings = settings.clone();
        let id = id.clone();
        set.spawn(
            async move { get_animal_details(&settings, AnimalIdArgs { animal_id: id }).await },
        );
    }

    let mut valid_animals = Vec::new();
    let mut errors = Vec::new();

    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(val)) => {
                if let Some(data) = val.get("data") {
                    if let Some(animal) = extract_single_item(data) {
                        valid_animals.push(animal.clone());
                    }
                }
            }
            Ok(Err(e)) => errors.push(e.to_string()),
            Err(e) => errors.push(format!("Task join error: {}", e)),
        }
    }

    Ok(json!({ "data": valid_animals, "errors": errors }))
}

pub async fn search_organizations(
    settings: &Settings,
    args: OrgSearchArgs,
) -> Result<Value, AppError> {
    let url = format!("{}/public/orgs/search", settings.base_url);
    let miles = args.miles.unwrap_or(settings.default_miles);
    let postal_code = args
        .postal_code
        .as_deref()
        .unwrap_or(&settings.default_postal_code);

    let body = if let Some(q) = &args.query {
        json!({
            "data": {
                "filterRadius": {
                    "miles": miles,
                    "postalcode": postal_code
                },
                "filters": [
                    {
                        "fieldName": "orgs.name",
                        "operation": "contains",
                        "criteria": q
                    }
                ]
            }
        })
    } else {
        json!({
            "data": {
                "filterRadius": {
                    "miles": miles,
                    "postalcode": postal_code
                }
            }
        })
    };

    fetch_with_cache(settings, &url, "POST", Some(body)).await
}

pub async fn get_organization_details(
    settings: &Settings,
    args: OrgIdArgs,
) -> Result<Value, AppError> {
    let url = format!("{}/public/orgs/{}", settings.base_url, args.org_id);
    fetch_with_cache(settings, &url, "GET", None).await
}

pub async fn list_org_animals(settings: &Settings, args: OrgIdArgs) -> Result<Value, AppError> {
    let url = format!(
        "{}/public/orgs/{}/animals/search/available",
        settings.base_url, args.org_id
    );
    fetch_with_cache(settings, &url, "GET", None).await
}

fn build_search_body(miles: u32, postal_code: &str, filters: Vec<Value>) -> Value {
    let mut data_obj = json!({
        "filterRadius": {
            "miles": miles,
            "postalcode": postal_code
        }
    });

    if !filters.is_empty() {
        data_obj["filters"] = json!(filters);
    }

    json!({ "data": data_obj })
}

fn add_filter(
    filters: &mut Vec<Value>,
    field: &str,
    operation: &str,
    criteria: impl serde::Serialize,
) {
    filters.push(json!({
        "fieldName": field,
        "operation": operation,
        "criteria": criteria
    }));
}

pub async fn fetch_pets(settings: &Settings, args: ToolArgs) -> Result<Value, AppError> {
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
        add_filter(&mut filters, "breeds.name", "contains", breeds);
    }

    if let Some(sex) = &args.sex {
        add_filter(&mut filters, "animals.sex", "equal", sex);
    }

    if let Some(age) = &args.age {
        add_filter(&mut filters, "animals.ageGroup", "equal", age);
    }

    if let Some(size) = &args.size {
        add_filter(&mut filters, "animals.sizeGroup", "equal", size);
    }

    let bool_to_criteria = |v: bool| if v { "Yes" } else { "No" };

    if let Some(val) = args.good_with_children {
        add_filter(
            &mut filters,
            "animals.isGoodWithChildren",
            "equal",
            bool_to_criteria(val),
        );
    }

    if let Some(val) = args.good_with_dogs {
        add_filter(
            &mut filters,
            "animals.isGoodWithDogs",
            "equal",
            bool_to_criteria(val),
        );
    }

    if let Some(val) = args.good_with_cats {
        add_filter(
            &mut filters,
            "animals.isGoodWithCats",
            "equal",
            bool_to_criteria(val),
        );
    }

    if let Some(val) = args.house_trained {
        add_filter(
            &mut filters,
            "animals.isHouseTrained",
            "equal",
            bool_to_criteria(val),
        );
    }

    if let Some(val) = args.special_needs {
        add_filter(
            &mut filters,
            "animals.isSpecialNeeds",
            "equal",
            bool_to_criteria(val),
        );
    }

    if let Some(val) = args.needs_foster {
        add_filter(
            &mut filters,
            "animals.isNeedingFoster",
            "equal",
            bool_to_criteria(val),
        );
    }

    if let Some(color) = &args.color {
        add_filter(&mut filters, "animals.colorDetails", "contains", color);
    }

    if let Some(pattern) = &args.pattern {
        add_filter(&mut filters, "animals.patternDetails", "contains", pattern);
    }

    let body = build_search_body(miles, postal_code, filters);
    fetch_with_cache(settings, &url, "POST", Some(body)).await
}

pub async fn get_random_pet(
    settings: &Settings,
    species: Option<String>,
) -> Result<Value, AppError> {
    let args = ToolArgs {
        postal_code: None,
        miles: None,
        species,
        breeds: None,
        sex: None,
        age: None,
        size: None,
        good_with_children: None,
        good_with_dogs: None,
        good_with_cats: None,
        house_trained: None,
        special_needs: None,
        needs_foster: None,
        color: None,
        pattern: None,
        sort_by: Some("Random".to_string()),
    };
    fetch_pets(settings, args).await
}

pub async fn fetch_adopted_pets(
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

    let body = build_search_body(miles, postal_code, Vec::new());
    fetch_with_cache(settings, &url, "POST", Some(body)).await
}
