use crate::shared::{Person, current_date, read_csv};
use anyhow::{Context, Result, anyhow, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

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

fn check_for_new_address(person: &Person, data: &serde_json::Value) -> Option<Result<()>> {
    if data["person"]["hits"].as_array()?.len() == 1 {
        let info = &data["person"]["hits"][0];
        return Some(Err(anyhow!(
            "{}: Eventuellt ny adress - {}, {}",
            person.number,
            info["streetAddress"],
            info["city"]
        )));
    }
    None
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

fn verify_using_ratsit(person: &Person) -> Result<()> {
    // Find out how old this person is
    let today = current_date();
    let age = today.years_since(person.born).unwrap();

    // Fixup the name and address to form solid search strings
    let mut name = person.given_name.clone() + " " + &person.last_name;
    name = name.replace("d.y.", "");
    name = name.replace("MACHNOW", "MACKNOW");
    let mut address = person.address.clone()
        + " "
        + person.zip_code.get(3..6).unwrap()
        + person.zip_code.get(7..9).unwrap()
        + " "
        + &person.city;
    address = address.replace('\n', " ");

    // Do a search with all the known information. If we only get a single
    // hit, we have found our man.
    let name_and_address = name.clone() + " " + &address;
    let data = query_ratsit(&name_and_address, age)?;
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

    // So no exact match. Alright, check if there's someone of the same age
    // living in the same town.
    let name_and_city = name.clone() + " " + &person.city;
    let data = query_ratsit(&name_and_city, age)?;
    if let Some(value) = check_for_new_address(person, &data) {
        return value;
    }

    // Still no match. Let's widen the search: only use the age and no address.
    let data = query_ratsit(&name, age)?;
    if let Some(value) = check_for_new_address(person, &data) {
        return value;
    }

    // Well, I can't find this person.
    bail!(
        "{}: Hittar inte {} {} i Ratsit",
        person.number,
        person.given_name,
        person.last_name
    );
}

fn verify(person: &Person) -> Result<()> {
    // Skip everyone not living in Sweden
    if !person.zip_code.starts_with("SE-") {
        bail!(
            "{}: Inte bosatt i Sverige ({})",
            person.number,
            person.zip_code
        );
    }

    // Verify that the zip code format looks somewhat sane (SE-123 45)
    if person.zip_code.chars().nth(6).unwrap() != ' ' || person.zip_code.len() != 9 {
        bail!(
            "{}: Jag litar inte på postnumret {}",
            person.number,
            person.zip_code
        );
    }

    verify_using_ratsit(person)
}

pub fn run(filename: impl AsRef<Path>) -> Result<()> {
    // Just a quick design note here: this could easily have been made to run
    // in parallel and async, but... We don't really want to strain the
    // services we use.
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
        std::thread::sleep(std::time::Duration::from_secs(6));
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
