use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(author, version = env!("PROJECT_VERSION"), about)]
pub struct Cli {
    #[arg(long, env = "RESCUE_GROUPS_API_KEY", hide_env_values = true)]
    pub api_key: Option<String>,
    #[arg(long, default_value = "config.toml")]
    pub config: String,

    /// Output raw JSON instead of formatted text
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
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
    /// Get a random adoptable pet
    RandomPet {
        #[arg(long)]
        species: Option<String>,
    },
    /// List recently adopted animals (Success Stories)
    ListAdopted(AdoptedAnimalsArgs),
    /// List available breeds for a species
    ListBreeds(SpeciesArgs),
    /// Get details for a specific breed
    GetBreed(BreedIdArgs),
    /// List metadata values (colors, patterns, etc.)
    ListMetadata(MetadataArgs),
    /// List available metadata types
    ListMetadataTypes,
    /// Generate shell completions or man pages
    Generate(GenerateArgs),
}

#[derive(Args, Clone, Debug)]
pub struct HttpArgs {
    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Port to bind to
    #[arg(long, default_value = "3000")]
    pub port: u16,

    /// Optional authentication token (Bearer token)
    #[arg(long, env = "MCP_AUTH_TOKEN")]
    pub auth_token: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct GenerateArgs {
    /// Type of shell completion to generate
    #[arg(short, long)]
    pub shell: Option<Shell>,

    /// Generate man pages to the specified directory
    #[arg(short, long)]
    pub man: Option<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct ToolArgs {
    #[arg(long)]
    pub postal_code: Option<String>,
    #[arg(long)]
    pub miles: Option<u32>,
    #[arg(long)]
    pub species: Option<String>,
    #[arg(long)]
    pub breeds: Option<String>,
    #[arg(long)]
    pub sex: Option<String>,
    #[arg(long)]
    pub age: Option<String>,
    #[arg(long)]
    pub size: Option<String>,
    #[arg(long)]
    pub good_with_children: Option<bool>,
    #[arg(long)]
    pub good_with_dogs: Option<bool>,
    #[arg(long)]
    pub good_with_cats: Option<bool>,
    #[arg(long)]
    pub house_trained: Option<bool>,
    #[arg(long)]
    pub special_needs: Option<bool>,
    #[arg(long)]
    pub needs_foster: Option<bool>,
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub pattern: Option<String>,
    #[arg(long)]
    pub sort_by: Option<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct AnimalIdArgs {
    #[arg(long)]
    pub animal_id: String,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct BreedIdArgs {
    #[arg(long)]
    pub breed_id: String,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct CompareArgs {
    /// Comma-separated list of animal IDs to compare (max 5)
    #[arg(long, value_delimiter = ',')]
    pub animal_ids: Vec<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct SpeciesArgs {
    #[arg(long)]
    pub species: String,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct OrgSearchArgs {
    #[arg(long)]
    pub postal_code: Option<String>,
    #[arg(long)]
    pub miles: Option<u32>,
    #[arg(long)]
    pub query: Option<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct OrgIdArgs {
    #[arg(long)]
    pub org_id: String,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct AdoptedAnimalsArgs {
    #[arg(long)]
    pub postal_code: Option<String>,
    #[arg(long)]
    pub miles: Option<u32>,
    #[arg(long)]
    pub species: Option<String>,
}

#[derive(Args, Deserialize, Clone, Debug)]
pub struct MetadataArgs {
    #[arg(long)]
    pub metadata_type: String,
    #[arg(long)]
    pub species: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_args() {
        let args = vec!["prog", "search", "--species", "cats", "--color", "Black"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Some(Commands::Search(search_args)) => {
                assert_eq!(search_args.species, Some("cats".to_string()));
                assert_eq!(search_args.color, Some("Black".to_string()));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_events_removed() {
        let args = vec!["prog", "search-events"];
        let result = Cli::try_parse_from(args);
        assert!(result.is_err()); // Should fail as command was removed
    }
}
