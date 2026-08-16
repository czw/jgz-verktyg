use crate::shared::{Person, current_date, read_csv};
use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::thread;
use std::time::Duration;

fn build_ratsit_json(who: &str, age: u32) -> serde_json::Value {
    ureq::json!({
        "who": who,
        "age": [(age - 1).to_string(), age.to_string()],
        "phoneticSearch": true,
        "companyName": "",
        "orgNr": "",
        "firstName": "",
        "lastName": "",
        "personNumber": "",
        "phone": "",
        "address": "",
        "postnr": "",
        "postort": "",
        "kommun": "",
        "page": 1,
    })
}

/// Expected format: `SE-123 45`. Returns the five digits Ratsit searches for.
fn swedish_postal_digits(zip: &str) -> Option<String> {
    let digits = zip.strip_prefix("SE-")?;
    let (area, suffix) = digits.split_once(' ')?;
    if area.len() == 3
        && suffix.len() == 2
        && area.bytes().all(|b| b.is_ascii_digit())
        && suffix.bytes().all(|b| b.is_ascii_digit())
    {
        Some(format!("{area}{suffix}"))
    } else {
        None
    }
}

fn suggested_address(person: &Person, data: &serde_json::Value) -> Option<String> {
    let hits = data["person"]["hits"].as_array()?;
    if hits.len() != 1 {
        return None;
    }
    let info = &hits[0];
    Some(format!(
        "{}: Eventuellt ny adress - {}, {}",
        person.number, info["streetAddress"], info["city"]
    ))
}

fn is_deceased(url: &str) -> Result<bool> {
    let html = ureq::get(url)
        .call()
        .with_context(|| format!("Problem med att hämta livsstatus från {url}"))?
        .into_string()
        .with_context(|| format!("Kunde inte läsa svaret från {url}"))?;
    Ok(html.contains("registrerades som avliden"))
}

fn query_ratsit(who: &str, age: u32) -> Result<serde_json::Value> {
    ureq::post("https://www.ratsit.se/api/search/combined")
        .send_json(build_ratsit_json(who, age))
        .context("Kunde inte hämta data om person")?
        .into_json()
        .context("Kunde inte göra om data till JSON")
}

fn search_name(person: &Person) -> String {
    format!("{} {}", person.given_name, person.last_name)
        .replace("d.y.", "")
        .replace("MACHNOW", "MACKNOW")
}

fn search_address(person: &Person, postal_digits: &str) -> String {
    format!("{} {postal_digits} {}", person.address, person.city).replace('\n', " ")
}

fn verify_using_ratsit(person: &Person, postal_digits: &str) -> Result<()> {
    let age = current_date()
        .years_since(person.born)
        .with_context(|| format!("{}: Ogiltigt födelsedatum", person.number))?;

    let name = search_name(person);
    let address = search_address(person, postal_digits);

    let data = query_ratsit(&format!("{name} {address}"), age)?;
    let hits = data["person"]["hits"]
        .as_array()
        .context("Ratsit-svaret saknar personträffar")?;
    if hits.len() == 1 {
        let url = hits[0]["personUrl"]
            .as_str()
            .context("Ratsit-svaret saknar personUrl")?;
        if url.is_empty() {
            bail!("{}: Kan inte kolla livsstatus", person.number);
        }
        if is_deceased(url)? {
            bail!("{}: Avliden", person.number);
        }
        return Ok(());
    }

    // No exact match. Check if someone of the same age lives in the same town.
    let data = query_ratsit(&format!("{name} {}", person.city), age)?;
    if let Some(message) = suggested_address(person, &data) {
        bail!("{message}");
    }

    // Still no match. Widen the search: name and age only.
    let data = query_ratsit(&name, age)?;
    if let Some(message) = suggested_address(person, &data) {
        bail!("{message}");
    }

    bail!(
        "{}: Hittar inte {} {} i Ratsit",
        person.number,
        person.given_name,
        person.last_name
    );
}

fn verify(person: &Person) -> Result<()> {
    let Some(postal_digits) = swedish_postal_digits(&person.zip_code) else {
        if person.zip_code.starts_with("SE-") {
            bail!(
                "{}: Jag litar inte på postnumret {}",
                person.number,
                person.zip_code
            );
        }
        bail!(
            "{}: Inte bosatt i Sverige ({})",
            person.number,
            person.zip_code
        );
    };

    verify_using_ratsit(person, &postal_digits)
}

pub fn run(filename: impl AsRef<Path>) -> Result<()> {
    // This could easily have been made to run in parallel, but we do not
    // want to strain the services we use.
    let mut people = read_csv(filename)?;
    people.sort_unstable_by_key(|p| p.number);
    let pb = ProgressBar::new(people.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("{msg} {bar:56.cyan/blue} ({eta})")
            .context("Ogiltig progressbar-mall")?,
    );
    pb.set_message("Verifierar");

    let mut messages = Vec::new();
    for person in people {
        pb.inc(1);
        if let Err(err) = verify(&person) {
            messages.push(err);
        }
        thread::sleep(Duration::from_secs(6));
    }
    pb.finish();

    if !messages.is_empty() {
        println!("Fick följande meddelanden:");
        for message in messages {
            println!(" - {message}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_swedish_postal_codes() {
        assert_eq!(swedish_postal_digits("SE-123 45").as_deref(), Some("12345"));
        assert_eq!(swedish_postal_digits("SE-12345"), None);
        assert_eq!(swedish_postal_digits("SE-12 345"), None);
        assert_eq!(swedish_postal_digits("123 45"), None);
        assert_eq!(swedish_postal_digits("NO-123 45"), None);
    }
}
