use crate::shared::{self, Person};
use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use std::collections::BTreeMap;
use std::path::Path;

fn print_age_averages(people: &Vec<Person>) {
    let today = Local::now().naive_local().date();
    let mut ages = Vec::new();
    for person in people {
        ages.push(today.years_since(person.born).unwrap());
    }
    ages.sort();

    // Average and median age
    let average = ages.iter().sum::<u32>() as f32 / ages.len() as f32;
    println!("Medelålder: {:.1}", average);
    println!("Medianålder: {}", ages[ages.len() / 2]);

    // Most common age
    let mut occurrences = BTreeMap::new();
    for age in ages {
        *occurrences.entry(age).or_insert(0) += 1;
    }
    let most_common_age = occurrences
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(val, _)| val)
        .unwrap();
    println!("Vanligaste åldern: {} år", most_common_age);
}

fn print_born_each_decade(people: &Vec<Person>) {
    let mut born_at_decade = BTreeMap::new();
    for person in people {
        let entry = born_at_decade.entry(person.born.year() / 10).or_insert(0);
        *entry += 1;
    }

    println!("\nAntal födda per årtionde:");
    for (decade, count) in born_at_decade {
        println!("- {}-talet: {} medlemmar", decade * 10, count);
    }
}

fn print_born_each_month(people: &Vec<Person>) {
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "Maj", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dec",
    ];
    let mut born_at_month = BTreeMap::new();
    for person in people {
        *born_at_month.entry(person.born.month() - 1).or_insert(0) += 1;
    }

    println!("\nAntal födda per månad:");
    for (month, count) in born_at_month {
        println!("- {}: {} medlemmar", month_names[month as usize], count);
    }
}

fn print_city_distribution(people: &Vec<Person>) {
    let mut cities = BTreeMap::new();
    for person in people {
        if person.city.is_empty() {
            continue;
        }
        let city = person.city.to_uppercase();
        *cities.entry(city).or_insert(0) += 1;
    }

    println!("\nMedlemsfördelning per stad/ort (topp 20):");
    let mut sorted_cities: Vec<_> = cities.into_iter().collect();
    sorted_cities.sort_by(|a, b| b.1.cmp(&a.1));

    for (city, count) in sorted_cities.iter().take(20) {
        println!("- {}: {} medlemmar", city, count);
    }
}

fn print_membership_years(people: &Vec<Person>) {
    let today = Local::now().naive_local().date();
    let mut years = Vec::new();
    for person in people {
        if person.level1.is_empty() {
            continue;
        }
        if let Ok(entered) = NaiveDate::parse_from_str(&person.level1, "%Y-%m-%d") {
            years.push(today.years_since(entered).unwrap_or(0));
        }
    }
    years.sort();
    let sum = years.iter().sum::<u32>();
    let average = sum as f32 / years.len() as f32;
    println!("\nMedlemskapstid:");
    println!("- Sammanlagt: {} år", sum);
    println!("- Medel: {:.1} år", average);
    println!("- Median: {} år", years[years.len() / 2]);
}

fn print_most_common_birthdays(people: &Vec<Person>) {
    let mut birthdays = BTreeMap::new();
    for person in people {
        let birthday = (person.born.month(), person.born.day());
        *birthdays.entry(birthday).or_insert(0) += 1;
    }

    if let Some(max_count) = birthdays.values().max()
        && *max_count > 1
    {
        println!("\nVanligaste födelsedag(ar):");
        let month_names = [
            "januari",
            "februari",
            "mars",
            "april",
            "maj",
            "juni",
            "juli",
            "augusti",
            "september",
            "oktober",
            "november",
            "december",
        ];
        for ((month, day), count) in &birthdays {
            if *count == *max_count {
                println!(
                    "- {} {}: {} medlemmar",
                    day,
                    month_names[(*month - 1) as usize],
                    count
                );
            }
        }
    }
}

fn print_new_members_last_year(people: &Vec<Person>) {
    let today = Local::now().naive_local().date();
    let last_year = today.year() - 1;

    let mut entered_last_year = 0;
    for person in people {
        if person.level1.is_empty() {
            continue;
        }
        if let Ok(entered) = NaiveDate::parse_from_str(&person.level1, "%Y-%m-%d")
            && entered.year() == last_year
        {
            entered_last_year += 1;
        }
    }
    println!("Antal nya medlemmar {}: {}", last_year, entered_last_year);
}

fn print_total_at_levels(people: &Vec<Person>) {
    let mut total_at_levels: [u32; 9] = [0; 9];
    for person in people {
        if !person.level9.is_empty() {
            total_at_levels[8] += 1;
            total_at_levels[7] += 1;
        } else if !person.level8.is_empty() {
            total_at_levels[7] += 1;
        } else if !person.level7.is_empty() {
            total_at_levels[6] += 1;
        } else if !person.level6.is_empty() {
            total_at_levels[5] += 1;
        } else if !person.level5.is_empty() {
            total_at_levels[4] += 1;
        } else if !person.level4.is_empty() {
            total_at_levels[3] += 1;
        } else if !person.level3.is_empty() {
            total_at_levels[2] += 1;
        } else if !person.level2.is_empty() {
            total_at_levels[1] += 1;
        } else {
            total_at_levels[0] += 1;
        }
    }

    println!("\nAntal i varje grad:");
    println!("- I: {}", total_at_levels[0]);
    println!("- II: {}", total_at_levels[1]);
    println!("- III: {}", total_at_levels[2]);
    println!("- IV: {}", total_at_levels[3]);
    println!("- V: {}", total_at_levels[4]);
    println!("- VI: {}", total_at_levels[5]);
    println!("- VII: {}", total_at_levels[6]);
    println!(
        "- VIII: {} (varav {} honnörsgrad)",
        total_at_levels[7], total_at_levels[8]
    );
}

pub fn run(filename: impl AsRef<Path>) -> Result<()> {
    let people = shared::read_csv(filename)?;
    println!("Totalt antal medlemmar: {}", people.len());
    print_new_members_last_year(&people);
    print_age_averages(&people);
    print_membership_years(&people);
    print_total_at_levels(&people);
    print_born_each_decade(&people);
    print_born_each_month(&people);
    print_most_common_birthdays(&people);
    print_city_distribution(&people);
    Ok(())
}
