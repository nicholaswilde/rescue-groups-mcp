use crate::cli::{
    AdoptedAnimalsArgs, AnimalIdArgs, BreedIdArgs, CompareArgs, MetadataArgs, OrgIdArgs,
    OrgSearchArgs, SpeciesArgs, ToolArgs,
};
use crate::client::{
    compare_animals, fetch_adopted_pets, fetch_pets, get_animal_details, get_breed_details,
    get_contact_info, get_organization_details, get_random_pet, list_animals, list_breeds,
    list_metadata, list_metadata_types, list_org_animals, list_species, search_organizations,
};
use crate::config::Settings;
use crate::error::AppError;
use crate::fmt::{
    extract_single_item, format_animal_results, format_breed_details, format_breed_results,
    format_comparison_table, format_contact_info, format_metadata_results, format_org_results,
    format_single_animal, format_single_org, format_species_results,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::warn;

#[derive(Deserialize, Debug)]
pub struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    pub _jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

fn get_all_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "list_animals",
            "description": "List the most recent adoptable animals available globally.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "list_species",
            "description": "List all animal species supported by the RescueGroups API.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "list_metadata",
            "description": "List valid metadata values for animal attributes (colors, patterns, qualities).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "metadata_type": {
                        "type": "string",
                        "description": "The type of metadata to list (e.g., colors, patterns, qualities)"
                    },
                    "species": {
                        "type": "string",
                        "description": "Optional: Type of animal (e.g., dogs, cats) to filter results."
                    }
                },
                "required": ["metadata_type"]
            }
        }),
        json!({
            "name": "list_metadata_types",
            "description": "List all valid metadata types that can be used with list_metadata.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "list_breeds",
            "description": "List available breeds for a specific species.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "species": { "type": "string", "description": "Type of animal (e.g., dogs, cats, rabbits)" }
                },
                "required": ["species"]
            }
        }),
        json!({
            "name": "get_breed",
            "description": "Get detailed information about a specific breed by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "breed_id": { "type": "string", "description": "The unique ID of the breed." }
                },
                "required": ["breed_id"]
            }
        }),
        json!({
            "name": "get_animal_details",
            "description": "Get detailed information about a specific animal by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "animal_id": { "type": "string", "description": "The unique ID of the animal." }
                },
                "required": ["animal_id"]
            }
        }),
        json!({
            "name": "get_contact_info",
            "description": "Get the primary contact method (email, phone, organization) for a specific animal.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "animal_id": { "type": "string", "description": "The unique ID of the animal." }
                },
                "required": ["animal_id"]
            }
        }),
        json!({
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
        }),
        json!({
            "name": "get_organization_details",
            "description": "Get detailed information about a specific rescue organization by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_id": { "type": "string", "description": "The unique ID of the organization." }
                },
                "required": ["org_id"]
            }
        }),
        json!({
            "name": "list_org_animals",
            "description": "List all animals available for adoption at a specific organization.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_id": { "type": "string", "description": "The unique ID of the organization." }
                },
                "required": ["org_id"]
            }
        }),
        json!({
            "name": "search_organizations",
            "description": "Search for animal rescue organizations and shelters by location.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "postal_code": { "type": "string", "description": "Zip code (e.g. 90210)" },
                    "miles": { "type": "integer", "description": "Search radius (default 50)" },
                    "query": { "type": "string", "description": "Name of the organization to search for (partial match)" }
                }
            }
        }),
        json!({
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
                    "needs_foster": { "type": "boolean", "description": "Whether the pet needs a foster home." },
                    "color": { "type": "string", "description": "Filter by color (partial match)." },
                    "pattern": { "type": "string", "description": "Filter by pattern (partial match)." },
                    "sort_by": {
                        "type": "string",
                        "enum": ["Newest", "Distance", "Random"],
                        "description": "Sort order for results."
                    }
                }
            }
        }),
        json!({
            "name": "get_random_pet",
            "description": "Get a random adoptable pet (surpise me!).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "species": { "type": "string", "description": "Optional: Type of animal (e.g. dogs, cats)" }
                }
            }
        }),
        json!({
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
        }),
        json!({
            "name": "inspect_tool",
            "description": "Discover available tools or get detailed schema for a specific tool.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tool_name": {
                        "type": "string",
                        "description": "The name of the tool to inspect. If omitted, lists all available tools."
                    }
                }
            }
        }),
    ]
}

fn get_core_tool_definitions() -> Vec<Value> {
    let all = get_all_tool_definitions();
    let core_names = [
        "search_adoptable_pets",
        "get_animal_details",
        "inspect_tool",
    ];

    all.into_iter()
        .filter(|t| core_names.contains(&t["name"].as_str().unwrap_or("")))
        .collect()
}

pub async fn handle_tool_call(
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
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
            )
            .unwrap_or(MetadataArgs {
                metadata_type: "colors".to_string(),
                species: None,
            });

            let data = list_metadata(settings, args.clone()).await?;
            let content = format_metadata_results(&data, &args.metadata_type)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "list_metadata_types" => {
            let data = list_metadata_types().await?;
            let types = data["data"].as_array().unwrap();
            let content = types
                .iter()
                .map(|t| t.as_str().unwrap_or(""))
                .collect::<Vec<&str>>()
                .join("\n");
            Ok(
                json!({ "content": [{ "type": "text", "text": format!("### Supported Metadata Types\n\n{}", content) }] }),
            )
        }
        "list_breeds" => {
            let args: SpeciesArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
            )
            .unwrap_or(SpeciesArgs {
                species: settings.default_species.clone(),
            });

            let data = list_breeds(settings, args.clone()).await?;
            let content = format_breed_results(&data, &args.species)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "get_breed" => {
            let args: BreedIdArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
            )
            .unwrap_or(BreedIdArgs {
                breed_id: "0".to_string(),
            });

            let data = get_breed_details(settings, args).await?;
            let breed_data = data.get("data");
            match breed_data.and_then(|d| extract_single_item(d)) {
                Some(b) => {
                    Ok(json!({ "content": [{ "type": "text", "text": format_breed_details(b) }] }))
                }
                None => Err(AppError::NotFound),
            }
        }
        "get_animal_details" => {
            let args: AnimalIdArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
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
                None => Err(AppError::NotFound),
            }
        }
        "get_contact_info" => {
            let args: AnimalIdArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
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
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
            )
            .unwrap_or(CompareArgs { animal_ids: vec![] });

            let data = compare_animals(settings, args).await?;
            let content = format_comparison_table(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "search_organizations" => {
            let args: OrgSearchArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
            )
            .unwrap_or(OrgSearchArgs {
                postal_code: None,
                miles: None,
                query: None,
            });

            let data = search_organizations(settings, args).await?;
            let content = format_org_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "get_organization_details" => {
            let args: OrgIdArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
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
                None => Err(AppError::NotFound),
            }
        }
        "list_org_animals" => {
            let args: OrgIdArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
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
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
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
                needs_foster: None,
                color: None,
                pattern: None,
                sort_by: None,
            });

            let data = fetch_pets(settings, args).await?;
            let content = format_animal_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "get_random_pet" => {
            let species = params
                .as_ref()
                .and_then(|p| p.get("arguments"))
                .and_then(|a| a.get("species"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());

            let data = get_random_pet(settings, species).await?;
            // Reuse animal formatter but maybe limit to 1 if not already
            let content = format_animal_results(&data)?;
            Ok(json!({ "content": [{ "type": "text", "text": content }] }))
        }
        "list_adopted_animals" => {
            let args: AdoptedAnimalsArgs = serde_json::from_value(
                params
                    .unwrap_or_default()
                    .get("arguments")
                    .cloned()
                    .unwrap_or_default(),
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
        "inspect_tool" => {
            let tool_name = params
                .as_ref()
                .and_then(|p| p.get("arguments"))
                .and_then(|a| a.get("tool_name"))
                .and_then(|n| n.as_str());

            if let Some(name) = tool_name {
                // Find specific tool
                let tools = get_all_tool_definitions();
                if let Some(tool) = tools.iter().find(|t| t["name"].as_str() == Some(name)) {
                    Ok(
                        json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(tool).unwrap() }] }),
                    )
                } else {
                    Err(AppError::NotFound) // Tool not found
                }
            } else {
                // List all tools (name + description)
                let tools = get_all_tool_definitions();
                let summary = tools
                    .iter()
                    .map(|t| {
                        format!(
                            "- {}: {}",
                            t["name"].as_str().unwrap(),
                            t["description"].as_str().unwrap_or("")
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                Ok(json!({ "content": [{ "type": "text", "text": summary }] }))
            }
        }
        _ => Err(AppError::NotFound),
    }
}

pub async fn process_mcp_request(
    req: JsonRpcRequest,
    settings: &Settings,
) -> (Option<Value>, Result<Value, Value>) {
    let response = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "rescue-groups-mcp", "version": env!("PROJECT_VERSION") }
        })),

        "notifications/initialized" => return (None, Ok(json!({}))), // Notification, no response

        "tools/list" => {
            let tools = if settings.lazy {
                get_core_tool_definitions()
            } else {
                get_all_tool_definitions()
            };
            Ok(json!({ "tools": tools }))
        }

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
        }

        "ping" => Ok(json!({})),

        _ => Err(json!({ "code": -32601, "message": "Method not found" })),
    };

    (req.id, response)
}

pub fn format_json_rpc_response(id: Value, result: Result<Value, Value>) -> Value {
    let mut output = json!({
        "jsonrpc": "2.0",
        "id": id,
    });
    match result {
        Ok(res) => output["result"] = res,
        Err(err) => output["error"] = err,
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use governor::{Quota, RateLimiter};
    use moka::future::Cache;
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::Duration;

    fn get_test_settings() -> Settings {
        Settings {
            api_key: "test_key".to_string(),
            base_url: "http://test.url".to_string(),
            default_postal_code: "00000".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            timeout: Duration::from_secs(1),
            lazy: false,
            cache: Arc::new(Cache::new(10)),
            limiter: Arc::new(RateLimiter::direct(Quota::per_second(
                NonZeroU32::new(1).unwrap(),
            ))),
        }
    }

    #[test]
    fn test_format_json_rpc_response_success() {
        let id = json!(1);
        let result = Ok(json!({"foo": "bar"}));
        let output = format_json_rpc_response(id.clone(), result);
        assert_eq!(output["id"], id);
        assert_eq!(output["result"]["foo"], "bar");
        assert!(output.get("error").is_none());
    }

    #[test]
    fn test_format_json_rpc_response_error() {
        let id = json!(1);
        let result = Err(json!({"code": -1, "message": "fail"}));
        let output = format_json_rpc_response(id.clone(), result);
        assert_eq!(output["id"], id);
        assert_eq!(output["error"]["code"], -1);
        assert!(output.get("result").is_none());
    }

    #[tokio::test]
    async fn test_process_mcp_request_initialize() {
        let settings = get_test_settings();
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let (id, result) = process_mcp_request(req, &settings).await;
        assert_eq!(id, Some(json!(1)));
        let res = result.unwrap();
        assert_eq!(res["protocolVersion"], "2024-11-05");
    }

    #[tokio::test]
    async fn test_process_mcp_request_tools_list() {
        let settings = get_test_settings();
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        let (_, result) = process_mcp_request(req, &settings).await;
        let res = result.unwrap();
        let tools = res["tools"].as_array().unwrap();
        assert!(!tools.is_empty());
        let list_animals = tools.iter().find(|t| t["name"] == "list_animals");
        assert!(list_animals.is_some());
        let get_breed = tools.iter().find(|t| t["name"] == "get_breed");
        assert!(get_breed.is_some());
    }

    #[tokio::test]
    async fn test_process_mcp_request_tools_call_get_breed_not_found() {
        let settings = get_test_settings();
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "get_breed",
                "arguments": {
                    "breed_id": "99999"
                }
            })),
        };

        let (_, result) = process_mcp_request(req, &settings).await;
        // Since we don't have a real API or mock here, it will fail network or 404.
        // In our test environment, it will likely be a Network Error because of dummy URL.
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_call_list_species() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_tool_call("list_species", None, &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_list_breeds() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock_species = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_body(r#"{"data": [{"id": "1", "attributes": {"singular": "Dog", "plural": "Dogs"}}]}"#)
            .create_async()
            .await;

        let _mock_breeds = server
            .mock("GET", "/public/animals/species/1/breeds")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "species": "dog"
            }
        });

        let res = handle_tool_call("list_breeds", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_list_metadata() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("GET", "/public/animals/colors")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "metadata_type": "colors"
            }
        });

        let res = handle_tool_call("list_metadata", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_get_animal_details() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("GET", "/public/animals/123")
            .with_status(200)
            .with_body(r#"{"data": {"id": "123", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "animal_id": "123"
            }
        });

        let res = handle_tool_call("get_animal_details", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_get_contact_info() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("GET", "/public/animals/123?include=orgs")
            .with_status(200)
            .with_body(r#"{"data": {"id": "123", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "animal_id": "123"
            }
        });

        let res = handle_tool_call("get_contact_info", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_compare_animals() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body(r#"{"data": {"id": "1", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "animal_ids": ["1", "2"]
            }
        });

        let res = handle_tool_call("compare_animals", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_search_organizations() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("POST", "/public/orgs/search")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "query": "Rescue"
            }
        });

        let res = handle_tool_call("search_organizations", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_get_organization_details() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("GET", "/public/orgs/866")
            .with_status(200)
            .with_body(r#"{"data": {"id": "866", "attributes": {"name": "Test Org"}}}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "org_id": "866"
            }
        });

        let res = handle_tool_call("get_organization_details", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_list_org_animals() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("GET", "/public/orgs/866/animals/search/available")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "org_id": "866"
            }
        });

        let res = handle_tool_call("list_org_animals", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_search_adoptable_pets() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("POST", "/public/animals/search/available/dogs/haspic")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "species": "dogs"
            }
        });

        let res = handle_tool_call("search_adoptable_pets", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_get_random_pet() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("POST", "/public/animals/search/available/dogs/haspic?sort=random")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "species": "dogs"
            }
        });

        let res = handle_tool_call("get_random_pet", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_list_adopted_animals() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings();
        let mut settings = settings.clone();
        settings.base_url = server.url();

        let _mock = server
            .mock("POST", "/public/animals/search/adopted/dogs/haspic")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let params = json!({
            "arguments": {
                "species": "dogs"
            }
        });

        let res = handle_tool_call("list_adopted_animals", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_inspect_tool() {
        let settings = get_test_settings();
        
        let res = handle_tool_call("inspect_tool", None, &settings).await;
        assert!(res.is_ok());

        let params = json!({
            "arguments": {
                "tool_name": "list_animals"
            }
        });
        let res = handle_tool_call("inspect_tool", Some(params), &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_process_mcp_request_notifications() {
        let settings = get_test_settings();
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        };

        let (id, result) = process_mcp_request(req, &settings).await;
        assert!(id.is_none());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_mcp_request_ping() {
        let settings = get_test_settings();
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "ping".to_string(),
            params: None,
        };

        let (_, result) = process_mcp_request(req, &settings).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_mcp_request_tools_list_lazy() {
        let mut settings = get_test_settings();
        settings.lazy = true;
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        let (_, result) = process_mcp_request(req, &settings).await;
        let res = result.unwrap();
        let tools = res["tools"].as_array().unwrap();
        assert!(tools.len() < get_all_tool_definitions().len());
    }

    #[tokio::test]
    async fn test_process_mcp_request_tools_call_missing_params() {
        let settings = get_test_settings();
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/call".to_string(),
            params: None,
        };

        let (_, result) = process_mcp_request(req, &settings).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err()["code"], -32602);
    }

    #[tokio::test]
    async fn test_handle_tool_call_not_found_branches() {
        let mut server = mockito::Server::new_async().await;
        let mut settings = get_test_settings();
        settings.base_url = server.url();

        let _mock = server.mock("GET", mockito::Matcher::Any).with_status(200).with_body(r#"{"data": []}"#).create_async().await;

        // get_breed None
        let res = handle_tool_call("get_breed", None, &settings).await;
        assert!(matches!(res, Err(AppError::NotFound)));

        // get_animal_details None
        let res = handle_tool_call("get_animal_details", None, &settings).await;
        assert!(matches!(res, Err(AppError::NotFound)));

        // get_organization_details None
        let res = handle_tool_call("get_organization_details", None, &settings).await;
        assert!(matches!(res, Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn test_handle_tool_call_unknown_tool() {
        let settings = get_test_settings();
        let res = handle_tool_call("unknown_tool", None, &settings).await;
        assert!(matches!(res, Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn test_handle_tool_call_list_animals() {
        let mut server = mockito::Server::new_async().await;
        let mut settings = get_test_settings();
        settings.base_url = server.url();

        let _mock = server.mock("GET", "/public/animals").with_status(200).with_body(r#"{"data": []}"#).create_async().await;

        let res = handle_tool_call("list_animals", None, &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tool_call_list_metadata_types() {
        let settings = get_test_settings();
        let res = handle_tool_call("list_metadata_types", None, &settings).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_process_mcp_request_method_not_found() {
        let settings = get_test_settings();
        let req = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "unknown".to_string(),
            params: None,
        };

        let (_, result) = process_mcp_request(req, &settings).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err()["code"], -32601);
    }
}
