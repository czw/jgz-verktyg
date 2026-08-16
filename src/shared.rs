use chrono::{Datelike, Local, NaiveDate};
use minijinja::{Environment, path_loader};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub static ENV: LazyLock<Environment<'static>> = LazyLock::new(|| {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    env
});

#[derive(Debug, Serialize, Clone, Copy, PartialEq)]
pub enum MemberFeeLevel {
    Free,
    Normal,
    Reduced,
}

impl<'de> Deserialize<'de> for MemberFeeLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            Ok(MemberFeeLevel::Normal)
        } else if s.starts_with("Fri") {
            Ok(MemberFeeLevel::Free)
        } else if s.starts_with("Reducerad") {
            Ok(MemberFeeLevel::Reduced)
        } else {
            Ok(MemberFeeLevel::Normal)
        }
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct PaymentHistory {
    pub last_paid_year: u16,
}
const LAST_PAYMENT_IF_EMPTY: u16 = 2023;

impl<'de> Deserialize<'de> for PaymentHistory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut last_year = LAST_PAYMENT_IF_EMPTY;
        for (key, value) in map {
            if key.starts_with("Medlavg ") {
                let is_filled = match value {
                    serde_json::Value::String(s) => !s.is_empty(),
                    serde_json::Value::Null => false,
                    _ => true, // Numbers, etc.
                };
                if is_filled
                    && let Ok(year) = key["Medlavg ".len()..].parse::<u16>()
                    && year > last_year
                {
                    last_year = year;
                }
            }
        }
        Ok(PaymentHistory {
            last_paid_year: last_year,
        })
    }
}

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
    #[serde(rename(deserialize = "Epost"), deserialize_with = "trim_whitespace")]
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
    #[serde(rename(deserialize = "Fri"))]
    pub member_fee_level: MemberFeeLevel,
    #[serde(flatten)]
    pub last_member_fee_paid: PaymentHistory,
}

pub fn current_date() -> NaiveDate {
    Local::now().naive_local().date()
}

pub fn current_year() -> i32 {
    current_date().year()
}

fn trim_whitespace<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.trim().to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_deserialization() {
        let csv_data = "\
N:r,Efternamn,Förnamn,Adress,Postadress,Postnr,Född,Epost,Tfn,Mobil,I. graden,II. graden,III. graden,IV. graden,V. graden,VI. graden,VII. graden,VIII. graden,Hgr,Fri,Medlavg 2024,Medlavg 2025,Medlavg 2026
1,Svensson,Sven,Gatan 1,Staden,123 45,1980-01-01, sven@example.com ,0123,070,2000-01-01,,,,,,,,,Fri 2024,100,200,
2,Karlsson,Karl,Vägen 2,Byn,543 21,1990-05-10,karl@example.com,0321,071,2010-02-02,,,,,,,,,Reducerad 2025,100,,300
3,Nilsson,Nils,Stigen 3,Hult,987 65,1975-12-12,nils@example.com,0987,072,1995-03-03,,,,,,,,,,,,";
        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let people: Vec<Person> = reader.deserialize().collect::<Result<_, _>>().unwrap();

        assert_eq!(people.len(), 3);

        // Person 1: Fri 2024, paid 2024 and 2025. Highest year should be 2025. Email trimmed.
        assert_eq!(people[0].member_fee_level, MemberFeeLevel::Free);
        assert_eq!(people[0].last_member_fee_paid.last_paid_year, 2025);
        assert_eq!(people[0].email, "sven@example.com");

        // Person 2: Reducerad 2025, paid 2024 and 2026. Highest year should be 2026.
        assert_eq!(people[1].member_fee_level, MemberFeeLevel::Reduced);
        assert_eq!(people[1].last_member_fee_paid.last_paid_year, 2026);

        // Person 3: Blank, no payments.
        assert_eq!(people[2].member_fee_level, MemberFeeLevel::Normal);
        assert_eq!(
            people[2].last_member_fee_paid.last_paid_year,
            LAST_PAYMENT_IF_EMPTY
        );
    }
}
