use crate::shared::{self, Person};
use anyhow::Result;
use chrono::Datelike;
use std::collections::BTreeMap;
use std::path::Path;

const MONTH_ABBREVS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Maj", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dec",
];
const MONTH_NAMES: [&str; 12] = [
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

fn average(values: &[u32]) -> f64 {
    f64::from(values.iter().copied().sum::<u32>()) / values.len() as f64
}

fn median(sorted: &[u32]) -> u32 {
    sorted[sorted.len() / 2]
}

fn print_age_averages(people: &[Person]) {
    let today = shared::current_date();
    let mut ages: Vec<u32> = people
        .iter()
        .filter_map(|person| today.years_since(person.born))
        .collect();
    if ages.is_empty() {
        return;
    }
    ages.sort_unstable();

    println!("Medelålder: {:.1}", average(&ages));
    println!("Medianålder: {}", median(&ages));

    let mut occurrences = BTreeMap::new();
    for age in ages {
        *occurrences.entry(age).or_insert(0) += 1;
    }
    if let Some((most_common_age, _)) = occurrences.into_iter().max_by_key(|&(_, count)| count) {
        println!("Vanligaste åldern: {most_common_age} år");
    }
}

fn print_born_each_decade(people: &[Person]) {
    let mut born_at_decade = BTreeMap::new();
    for person in people {
        *born_at_decade.entry(person.born.year() / 10).or_insert(0) += 1;
    }

    println!("\nAntal födda per årtionde:");
    for (decade, count) in born_at_decade {
        println!("- {}-talet: {count} medlemmar", decade * 10);
    }
}

fn print_born_each_month(people: &[Person]) {
    let mut born_at_month = BTreeMap::new();
    for person in people {
        *born_at_month.entry(person.born.month() - 1).or_insert(0) += 1;
    }

    println!("\nAntal födda per månad:");
    for (month, count) in born_at_month {
        println!("- {}: {count} medlemmar", MONTH_ABBREVS[month as usize]);
    }
}

fn print_city_distribution(people: &[Person]) {
    let mut cities = BTreeMap::new();
    for person in people {
        if person.city.is_empty() {
            continue;
        }
        *cities.entry(person.city.to_uppercase()).or_insert(0) += 1;
    }

    println!("\nMedlemsfördelning per stad/ort (topp 20):");
    let mut sorted_cities: Vec<_> = cities.into_iter().collect();
    sorted_cities.sort_unstable_by_key(|(_, count)| std::cmp::Reverse(*count));

    for (city, count) in sorted_cities.into_iter().take(20) {
        println!("- {city}: {count} medlemmar");
    }
}

fn print_membership_years(people: &[Person]) {
    let today = shared::current_date();
    let mut years: Vec<u32> = people
        .iter()
        .filter_map(Person::entered_on)
        .map(|entered| today.years_since(entered).unwrap_or(0))
        .collect();
    if years.is_empty() {
        return;
    }
    years.sort_unstable();

    let sum = years.iter().sum::<u32>();
    println!("\nMedlemskapstid:");
    println!("- Sammanlagt: {sum} år");
    println!("- Medel: {:.1} år", average(&years));
    println!("- Median: {} år", median(&years));
}

fn print_most_common_birthdays(people: &[Person]) {
    let mut birthdays = BTreeMap::new();
    for person in people {
        *birthdays
            .entry((person.born.month(), person.born.day()))
            .or_insert(0) += 1;
    }

    let Some(&max_count) = birthdays.values().max() else {
        return;
    };
    if max_count <= 1 {
        return;
    }

    println!("\nVanligaste födelsedag(ar):");
    for ((month, day), count) in &birthdays {
        if *count == max_count {
            println!(
                "- {day} {}: {count} medlemmar",
                MONTH_NAMES[(*month - 1) as usize]
            );
        }
    }
}

fn print_new_members_last_year(people: &[Person]) {
    let last_year = shared::current_year() - 1;
    let entered_last_year = people
        .iter()
        .filter_map(Person::entered_on)
        .filter(|entered| entered.year() == last_year)
        .count();
    println!("Antal nya medlemmar {last_year}: {entered_last_year}");
}

fn highest_degree_index(dates: &[&str; 9]) -> usize {
    dates.iter().rposition(|date| !date.is_empty()).unwrap_or(0)
}

fn print_total_at_levels(people: &[Person]) {
    let mut total_at_levels = [0u32; 9];
    for person in people {
        match highest_degree_index(&person.degree_dates()) {
            8 => {
                total_at_levels[8] += 1;
                total_at_levels[7] += 1;
            }
            idx => total_at_levels[idx] += 1,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_degree_treats_missing_dates_as_first_degree() {
        assert_eq!(highest_degree_index(&[""; 9]), 0);
    }

    #[test]
    fn highest_degree_finds_honor_degree() {
        let mut dates = [""; 9];
        dates[7] = "2010-01-01";
        dates[8] = "2015-01-01";
        assert_eq!(highest_degree_index(&dates), 8);
    }

    #[test]
    fn empty_member_list_does_not_panic() {
        print_age_averages(&[]);
        print_membership_years(&[]);
        print_most_common_birthdays(&[]);
        print_total_at_levels(&[]);
    }
}
