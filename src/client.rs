use crate::cli::{
    AdoptedAnimalsArgs, AnimalIdArgs, BreedIdArgs, CompareArgs, MetadataArgs, OrgIdArgs,
    OrgSearchArgs, SpeciesArgs, ToolArgs,
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

async fn resolve_species_id(settings: &Settings, species: &str) -> Result<String, AppError> {
    if species.chars().all(char::is_numeric) {
        return Ok(species.to_string());
    }

    let species_list = list_species(settings).await?;
    let data = species_list
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or(AppError::Internal(
            "Failed to fetch species list for resolution".to_string(),
        ))?;

    let target = species.to_lowercase();
    let found = data.iter().find(|s| {
        let attrs = &s["attributes"];
        let singular = attrs["singular"].as_str().unwrap_or("").to_lowercase();
        let plural = attrs["plural"].as_str().unwrap_or("").to_lowercase();
        singular == target || plural == target
    });

    if let Some(s) = found {
        Ok(s["id"].as_str().unwrap_or("").to_string())
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn list_breeds(settings: &Settings, args: SpeciesArgs) -> Result<Value, AppError> {
    let species_id = resolve_species_id(settings, &args.species).await?;

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
    let url = if let Some(species) = &args.species {
        let species_id = resolve_species_id(settings, species).await?;
        format!(
            "{}/public/animals/species/{}/{}",
            settings.base_url, species_id, args.metadata_type
        )
    } else {
        format!(
            "{}/public/animals/{}",
            settings.base_url, args.metadata_type
        )
    };
    fetch_with_cache(settings, &url, "GET", None).await
}

pub async fn get_breed_details(settings: &Settings, args: BreedIdArgs) -> Result<Value, AppError> {
    let url = format!(
        "{}/public/animals/breeds/{}",
        settings.base_url, args.breed_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::SpeciesArgs;
    use crate::config::Settings;
    use governor::{Quota, RateLimiter};
    use moka::future::Cache;
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::Duration;

    fn get_test_settings(url: String) -> Settings {
        Settings {
            api_key: "test_key".to_string(),
            base_url: url,
            default_postal_code: "00000".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            timeout: Duration::from_secs(1),
            lazy: false,
            cache: Arc::new(Cache::new(10)),
            limiter: Arc::new(RateLimiter::direct(Quota::per_second(
                NonZeroU32::new(100).unwrap(),
            ))),
        }
    }

    #[tokio::test]
    async fn test_list_species() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let settings = get_test_settings(url);

        let mock = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(r#"{"data": [{"id": "1", "type": "species", "attributes": {"singular": "Dog", "plural": "Dogs"}}]}"#)
            .create_async()
            .await;

        let result = list_species(&settings).await.unwrap();
        mock.assert_async().await;
        assert_eq!(result["data"][0]["attributes"]["singular"], "Dog");
    }

    #[tokio::test]
    async fn test_resolve_species_id_numeric() {
        let settings = get_test_settings("http://localhost".to_string());
        let id = resolve_species_id(&settings, "1").await.unwrap();
        assert_eq!(id, "1");
    }

    #[tokio::test]
    async fn test_resolve_species_id_name() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let settings = get_test_settings(url);

        let _mock = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(r#"{"data": [{"id": "1", "type": "species", "attributes": {"singular": "Dog", "plural": "Dogs"}}]}"#)
            .create_async()
            .await;

        let id = resolve_species_id(&settings, "dog").await.unwrap();
        assert_eq!(id, "1");

        let id = resolve_species_id(&settings, "Dogs").await.unwrap();
        assert_eq!(id, "1");
    }

    #[tokio::test]
    async fn test_resolve_species_id_not_found() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let settings = get_test_settings(url);

        let _mock = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_header("content-type", "application/vnd.api+json")
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let result = resolve_species_id(&settings, "cat").await;
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn test_list_breeds() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let settings = get_test_settings(url);

        let _mock_species = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"singular": "Dog", "plural": "Dogs"}}]}"#)
            .create_async()
            .await;

        let _mock_breeds = server
            .mock("GET", "/public/animals/species/1/breeds")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"name": "Labrador"}}]}"#)
            .create_async()
            .await;

        let result = list_breeds(&settings, SpeciesArgs { species: "dog".to_string() }).await.unwrap();
        assert_eq!(result["data"][0]["attributes"]["name"], "Labrador");
    }

    #[tokio::test]
    async fn test_get_animal_details() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/123")
            .with_status(200)
            .with_body(r#"{"data": {"id": "123", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let result = get_animal_details(&settings, AnimalIdArgs { animal_id: "123".to_string() }).await.unwrap();
        assert_eq!(result["data"]["attributes"]["name"], "Buddy");
    }

    #[tokio::test]
    async fn test_fetch_pets() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/animals/search/available/dogs/haspic?sort=-animals.createdDate")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"name": "Buddy"}}]}"#)
            .create_async()
            .await;

        let args = ToolArgs {
            postal_code: Some("12345".to_string()),
            miles: Some(10),
            species: Some("dogs".to_string()),
            breeds: Some("Labrador".to_string()),
            sex: Some("Male".to_string()),
            age: Some("Adult".to_string()),
            size: Some("Large".to_string()),
            good_with_children: Some(true),
            good_with_dogs: Some(true),
            good_with_cats: Some(false),
            house_trained: Some(true),
            special_needs: Some(false),
            needs_foster: Some(false),
            color: Some("Black".to_string()),
            pattern: Some("Solid".to_string()),
            sort_by: Some("Newest".to_string()),
        };

        let result = fetch_pets(&settings, args).await.unwrap();
        assert_eq!(result["data"][0]["attributes"]["name"], "Buddy");
    }

    #[tokio::test]
    async fn test_search_organizations() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/orgs/search")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"name": "Rescue Group"}}]}"#)
            .create_async()
            .await;

        let args = OrgSearchArgs {
            postal_code: None,
            miles: None,
            query: Some("Rescue".to_string()),
        };

        let result = search_organizations(&settings, args).await.unwrap();
        assert_eq!(result["data"][0]["attributes"]["name"], "Rescue Group");
    }

    #[tokio::test]
    async fn test_get_random_pet() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/animals/search/available/dogs/haspic?sort=random")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"name": "Buddy"}}]}"#)
            .create_async()
            .await;

        let result = get_random_pet(&settings, Some("dogs".to_string())).await.unwrap();
        assert_eq!(result["data"][0]["attributes"]["name"], "Buddy");
    }

    #[tokio::test]
    async fn test_fetch_adopted_pets() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/animals/search/adopted/dogs/haspic")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"name": "Happy"}}]}"#)
            .create_async()
            .await;

        let args = AdoptedAnimalsArgs {
            postal_code: None,
            miles: None,
            species: None,
        };

        let result = fetch_adopted_pets(&settings, args).await.unwrap();
        assert_eq!(result["data"][0]["attributes"]["name"], "Happy");
    }

    #[tokio::test]
    async fn test_list_metadata() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/colors")
            .with_status(200)
            .with_body(r#"{"data": ["Black", "White"]}"#)
            .create_async()
            .await;

        let args = MetadataArgs {
            metadata_type: "colors".to_string(),
            species: None,
        };

        let result = list_metadata(&settings, args).await.unwrap();
        assert_eq!(result["data"][0], "Black");
    }

    #[tokio::test]
    async fn test_get_contact_info() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/123?include=orgs")
            .with_status(200)
            .with_body(r#"{"data": {"id": "123"}, "included": [{"type": "orgs", "attributes": {"email": "test@example.com"}}]}"#)
            .create_async()
            .await;

        let result = get_contact_info(&settings, AnimalIdArgs { animal_id: "123".to_string() }).await.unwrap();
        assert!(result.get("included").is_some());
    }

    #[tokio::test]
    async fn test_compare_animals() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock1 = server
            .mock("GET", "/public/animals/1")
            .with_status(200)
            .with_body(r#"{"data": {"id": "1", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let _mock2 = server
            .mock("GET", "/public/animals/2")
            .with_status(200)
            .with_body(r#"{"data": {"id": "2", "attributes": {"name": "Lucy"}}}"#)
            .create_async()
            .await;

        let args = CompareArgs {
            animal_ids: vec!["1".to_string(), "2".to_string()],
        };

        let result = compare_animals(&settings, args).await.unwrap();
        assert_eq!(result["data"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_list_metadata_types() {
        let result = list_metadata_types().await.unwrap();
        assert!(result["data"].as_array().unwrap().contains(&json!("breeds")));
    }

    #[tokio::test]
    async fn test_api_error_404() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/999")
            .with_status(404)
            .create_async()
            .await;

        let result = get_animal_details(&settings, AnimalIdArgs { animal_id: "999".to_string() }).await;
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn test_api_error_500() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/error")
            .with_status(500)
            .create_async()
            .await;

        let result = get_animal_details(&settings, AnimalIdArgs { animal_id: "error".to_string() }).await;
        assert!(matches!(result, Err(AppError::ApiError(_))));
    }

    #[tokio::test]
    async fn test_list_animals() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let result = list_animals(&settings).await.unwrap();
        assert!(result["data"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_get_organization_details() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/orgs/866")
            .with_status(200)
            .with_body(r#"{"data": {"id": "866", "attributes": {"name": "Test Org"}}}"#)
            .create_async()
            .await;

        let result = get_organization_details(&settings, OrgIdArgs { org_id: "866".to_string() }).await.unwrap();
        assert_eq!(result["data"]["attributes"]["name"], "Test Org");
    }

    #[tokio::test]
    async fn test_list_org_animals() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/orgs/866/animals/search/available")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let result = list_org_animals(&settings, OrgIdArgs { org_id: "866".to_string() }).await.unwrap();
        assert!(result["data"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_list_metadata_with_species() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let settings = get_test_settings(url);

        let _mock_species = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"singular": "Dog", "plural": "Dogs"}}]}"#)
            .create_async()
            .await;

        let _mock_metadata = server
            .mock("GET", "/public/animals/species/1/colors")
            .with_status(200)
            .with_body(r#"{"data": ["Brown"]}"#)
            .create_async()
            .await;

        let args = MetadataArgs {
            metadata_type: "colors".to_string(),
            species: Some("dog".to_string()),
        };

        let result = list_metadata(&settings, args).await.unwrap();
        assert_eq!(result["data"][0], "Brown");
    }

    #[tokio::test]
    async fn test_compare_animals_with_errors() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock1 = server
            .mock("GET", "/public/animals/1")
            .with_status(200)
            .with_body(r#"{"data": {"id": "1", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let _mock2 = server
            .mock("GET", "/public/animals/error")
            .with_status(500)
            .create_async()
            .await;

        let args = CompareArgs {
            animal_ids: vec!["1".to_string(), "error".to_string()],
        };

        let result = compare_animals(&settings, args).await.unwrap();
        assert_eq!(result["data"].as_array().unwrap().len(), 1);
        assert_eq!(result["errors"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_rate_limiting_call() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let mut settings = get_test_settings(url);
        // High rate to avoid slow tests
        settings.limiter = Arc::new(RateLimiter::direct(Quota::per_second(
            NonZeroU32::new(100).unwrap(),
        )));

        let _mock = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        list_species(&settings).await.unwrap();
    }
}
