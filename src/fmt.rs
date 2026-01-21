use crate::error::AppError;
use serde_json::Value;
use tracing::error;

pub fn extract_single_item(data: &Value) -> Option<&Value> {
    match data {
        Value::Array(arr) => arr.first(),
        Value::Object(_) => Some(data),
        _ => None,
    }
}

pub fn format_single_animal(animal: &Value) -> String {
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

pub fn format_contact_info(data: &Value) -> Result<String, AppError> {
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
            "\n[View adoption application or more info on RescueGroups]({})",
            animal_url
        ));
    }

    Ok(contact_info)
}

pub fn format_animal_results(data: &Value) -> Result<String, AppError> {
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

pub fn format_comparison_table(data: &Value) -> Result<String, AppError> {
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

pub fn format_single_org(org: &Value) -> String {
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
        "# {}\n\n{}\n\n**Address:** {} {} {} {}\n**Phone:** {}\n**Email:** {}\n**Website:** {}\n**Facebook:** {}",
        name, about, address, city, state, postal_code, phone, email, url, facebook
    )
}

pub fn format_species_results(data: &Value) -> Result<String, AppError> {
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

pub fn format_metadata_results(data: &Value, metadata_type: &str) -> Result<String, AppError> {
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

pub fn format_org_results(data: &Value) -> Result<String, AppError> {
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

pub fn format_breed_results(data: &Value, species: &str) -> Result<String, AppError> {
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

pub fn print_output<F>(result: Result<Value, AppError>, json_mode: bool, formatter: F)
where
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_single_animal() {
        let animal = json!({
            "attributes": {
                "name": "Fluffy",
                "breedString": "Poodle",
                "sex": "Female",
                "ageGroup": "Adult",
                "sizeGroup": "Small",
                "descriptionText": "A cute dog.",
                "url": "https://example.com/fluffy",
                "orgsAnimalsPictures": [
                    { "urlSecureFullsize": "https://example.com/fluffy.jpg" }
                ]
            }
        });

        let output = format_single_animal(&animal);
        assert!(output.contains("# Fluffy"));
        assert!(output.contains("**Breed:** Poodle"));
        assert!(output.contains("![Fluffy](https://example.com/fluffy.jpg)"));
    }
}
