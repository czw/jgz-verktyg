use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Person {
    #[serde(rename(deserialize = "N:r"))]
    pub number: i16,
    #[serde(rename(deserialize = "Efternamn"))]
    pub last_name: String,
    #[serde(rename(deserialize = "Förnamn"))]
    pub given_name: String,
    #[serde(rename(deserialize = "Adress"))]
    pub address: String,
    #[serde(rename(deserialize = "Postadress"))]
    pub city: String,
    #[serde(rename(deserialize = "Postnr"))]
    pub zip_code: String,
    #[serde(rename(deserialize = "Född"))]
    pub born: NaiveDate,
    #[serde(rename(deserialize = "Epost"))]
    pub email: String,
    #[serde(rename(deserialize = "Tfn"))]
    pub phone1: String,
    #[serde(rename(deserialize = "Mobil"))]
    pub phone2: String,
    #[serde(rename(deserialize = "I. graden"))]
    pub level1: String,
    #[serde(rename(deserialize = "II. graden"))]
    pub level2: String,
    #[serde(rename(deserialize = "III. graden"))]
    pub level3: String,
    #[serde(rename(deserialize = "IV. graden"))]
    pub level4: String,
    #[serde(rename(deserialize = "V. graden"))]
    pub level5: String,
    #[serde(rename(deserialize = "VI. graden"))]
    pub level6: String,
    #[serde(rename(deserialize = "VII. graden"))]
    pub level7: String,
    #[serde(rename(deserialize = "VIII. graden"))]
    pub level8: String,
    #[serde(rename(deserialize = "Hgr"))]
    pub level9: String,
}

fn current_date() -> NaiveDate {
    Local::now().naive_local().date()
}

pub fn current_year() -> i32 {
    current_date().year()
}

pub fn read_csv(filename: String) -> Vec<Person> {
    let mut people = Vec::new();
    let mut reader = csv::Reader::from_path(filename).expect("Kunde inte öppna medlemsregistret");
    for result in reader.deserialize() {
        let person: Person = result.expect("Något gick fel vid läsning av medlemsregistret");
        people.push(person);
    }
    people
}
