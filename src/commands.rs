use crate::cli::{Cli, Commands};
use crate::client::{
    compare_animals, fetch_adopted_pets, fetch_pets, get_animal_details, get_contact_info,
    get_organization_details, get_random_pet, list_breeds, list_metadata, list_metadata_types,
    list_org_animals, list_species, search_organizations,
};
use crate::config::Settings;
use crate::error::AppError;
use crate::fmt::{
    extract_single_item, format_animal_results, format_breed_results, format_comparison_table,
    format_contact_info, format_metadata_results, format_org_results, format_single_animal,
    format_single_org, format_species_results, print_output,
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
