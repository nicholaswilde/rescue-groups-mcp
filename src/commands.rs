use crate::cli::{Cli, Commands};
use crate::client::{
    compare_animals, fetch_adopted_pets, fetch_pets, get_animal_details, get_breed_details,
    get_contact_info, get_organization_details, get_random_pet, list_breeds, list_metadata,
    list_metadata_types, list_org_animals, list_species, search_organizations,
};
use crate::config::Settings;
use crate::error::AppError;
use crate::fmt::{
    extract_single_item, format_animal_results, format_breed_details, format_breed_results,
    format_comparison_table, format_contact_info, format_metadata_results, format_org_results,
    format_single_animal, format_single_org, format_species_results, print_output,
};
use clap::CommandFactory;
use clap_complete::generate;
use clap_mangen::Man;
use std::fs;
use std::io;
use std::path::Path;
use tracing::{info, warn};

pub async fn handle_command(
    command: Commands,
    settings: &Settings,
    json_mode: bool,
) -> Result<(), AppError> {
    match command {
        Commands::Server | Commands::Http(_) => {
            // These should be handled by the caller (main.rs)
            Ok(())
        }
        Commands::Search(args) => {
            print_output(fetch_pets(settings, args).await, json_mode, |v| {
                format_animal_results(v)
            });
            Ok(())
        }
        Commands::ListSpecies => {
            print_output(list_species(settings).await, json_mode, |v| {
                format_species_results(v)
            });
            Ok(())
        }
        Commands::GetAnimal(args) => {
            print_output(get_animal_details(settings, args).await, json_mode, |v| {
                let animal_data = v.get("data").ok_or(AppError::NotFound)?;
                let animal = extract_single_item(animal_data).ok_or(AppError::NotFound)?;
                Ok(format_single_animal(animal))
            });
            Ok(())
        }
        Commands::GetContact(args) => {
            print_output(get_contact_info(settings, args).await, json_mode, |v| {
                format_contact_info(v)
            });
            Ok(())
        }
        Commands::Compare(args) => {
            print_output(compare_animals(settings, args).await, json_mode, |v| {
                format_comparison_table(v)
            });
            Ok(())
        }
        Commands::SearchOrgs(args) => {
            print_output(search_organizations(settings, args).await, json_mode, |v| {
                format_org_results(v)
            });
            Ok(())
        }
        Commands::GetOrg(args) => {
            print_output(
                get_organization_details(settings, args).await,
                json_mode,
                |v| {
                    let org_data = v.get("data").ok_or(AppError::NotFound)?;
                    let org = extract_single_item(org_data).ok_or(AppError::NotFound)?;
                    Ok(format_single_org(org))
                },
            );
            Ok(())
        }
        Commands::ListOrgAnimals(args) => {
            print_output(list_org_animals(settings, args).await, json_mode, |v| {
                format_animal_results(v)
            });
            Ok(())
        }
        Commands::RandomPet { species } => {
            print_output(get_random_pet(settings, species).await, json_mode, |v| {
                format_animal_results(v)
            });
            Ok(())
        }
        Commands::ListAdopted(args) => {
            print_output(fetch_adopted_pets(settings, args).await, json_mode, |v| {
                format_animal_results(v)
            });
            Ok(())
        }
        Commands::ListBreeds(args) => {
            let species = args.species.clone();
            print_output(list_breeds(settings, args).await, json_mode, |v| {
                format_breed_results(v, &species)
            });
            Ok(())
        }
        Commands::GetBreed(args) => {
            print_output(get_breed_details(settings, args).await, json_mode, |v| {
                let breed_data = v.get("data").ok_or(AppError::NotFound)?;
                let breed = extract_single_item(breed_data).ok_or(AppError::NotFound)?;
                Ok(format_breed_details(breed))
            });
            Ok(())
        }
        Commands::ListMetadata(args) => {
            let metadata_type = args.metadata_type.clone();
            print_output(list_metadata(settings, args).await, json_mode, |v| {
                format_metadata_results(v, &metadata_type)
            });
            Ok(())
        }
        Commands::ListMetadataTypes => {
            print_output(list_metadata_types().await, json_mode, |v| {
                let types = v["data"].as_array().unwrap();
                let content = types
                    .iter()
                    .map(|t| t.as_str().unwrap_or(""))
                    .collect::<Vec<&str>>()
                    .join("\n");
                Ok(format!("### Supported Metadata Types\n\n{}", content))
            });
            Ok(())
        }
        Commands::Generate(args) => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();

            if let Some(shell) = args.shell {
                generate(shell, &mut cmd, bin_name, &mut io::stdout());
            }

            if let Some(ref man_dir) = args.man {
                let out_dir = Path::new(man_dir);
                if !out_dir.exists() {
                    fs::create_dir_all(out_dir).map_err(AppError::Io)?;
                }
                Man::new(cmd)
                    .render(
                        &mut fs::File::create(out_dir.join("rescue-groups-mcp.1"))
                            .map_err(AppError::Io)?,
                    )
                    .map_err(AppError::Io)?;
                info!("Man page generated in {}", man_dir);
            }

            if args.shell.is_none() && args.man.is_none() {
                warn!("Please specify --shell <SHELL> or --man <DIR>");
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{AnimalIdArgs, SpeciesArgs};
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
    async fn test_handle_command_list_species() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/species")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_command(Commands::ListSpecies, &settings, false).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_get_animal() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/123")
            .with_status(200)
            .with_body(r#"{"data": {"id": "123", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::GetAnimal(AnimalIdArgs {
                animal_id: "123".to_string(),
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_list_breeds() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

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

        let res = handle_command(
            Commands::ListBreeds(SpeciesArgs {
                species: "dog".to_string(),
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_search_orgs() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/orgs/search")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::SearchOrgs(crate::cli::OrgSearchArgs {
                postal_code: None,
                miles: None,
                query: None,
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_list_metadata_types() {
        let settings = get_test_settings("http://localhost".to_string());
        let res = handle_command(Commands::ListMetadataTypes, &settings, false).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_random_pet() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/animals/search/available/dogs/haspic?sort=random")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::RandomPet {
                species: Some("dogs".to_string()),
            },
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_search() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/animals/search/available/dogs/haspic")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::Search(crate::cli::ToolArgs {
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
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_get_contact() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/123?include=orgs")
            .with_status(200)
            .with_body(r#"{"data": {"id": "123", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::GetContact(crate::cli::AnimalIdArgs {
                animal_id: "123".to_string(),
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_compare() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body(r#"{"data": {"id": "1", "attributes": {"name": "Buddy"}}}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::Compare(crate::cli::CompareArgs {
                animal_ids: vec!["1".to_string(), "2".to_string()],
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_get_org() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/orgs/866")
            .with_status(200)
            .with_body(r#"{"data": {"id": "866", "attributes": {"name": "Test Org"}}}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::GetOrg(crate::cli::OrgIdArgs {
                org_id: "866".to_string(),
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_list_org_animals() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/orgs/866/animals/search/available")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::ListOrgAnimals(crate::cli::OrgIdArgs {
                org_id: "866".to_string(),
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_list_adopted() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("POST", "/public/animals/search/adopted/dogs/haspic")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::ListAdopted(crate::cli::AdoptedAnimalsArgs {
                postal_code: None,
                miles: None,
                species: None,
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_list_metadata() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/colors")
            .with_status(200)
            .with_body(r#"{"data": []}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::ListMetadata(crate::cli::MetadataArgs {
                metadata_type: "colors".to_string(),
                species: None,
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_get_breed() {
        let mut server = mockito::Server::new_async().await;
        let settings = get_test_settings(server.url());

        let _mock = server
            .mock("GET", "/public/animals/breeds/1")
            .with_status(200)
            .with_body(r#"{"data": {"id": "1", "attributes": {"name": "Labrador"}}}"#)
            .create_async()
            .await;

        let res = handle_command(
            Commands::GetBreed(crate::cli::BreedIdArgs {
                breed_id: "1".to_string(),
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_handle_command_generate() {
        let settings = get_test_settings("http://localhost".to_string());
        let res = handle_command(
            Commands::Generate(crate::cli::GenerateArgs {
                shell: Some(clap_complete::Shell::Bash),
                man: None,
            }),
            &settings,
            false,
        )
        .await;
        assert!(res.is_ok());
    }
}
