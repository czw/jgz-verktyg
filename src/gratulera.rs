use crate::shared;
use anyhow::{Context, Result};
use chrono::Datelike;
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use std::path::Path;

fn write_column_names(sheet: &mut Worksheet) -> Result<()> {
    let column_names = [
        "Nr",
        "Dag-Månad",
        "Ålder",
        "Födelsedag",
        "Namn",
        "Adress",
        "Postnummer",
        "Ort",
    ];
    let header_format = Format::new().set_bold();

    for (i, name) in column_names.into_iter().enumerate() {
        sheet
            .write_with_format(0, u16::try_from(i)?, name, &header_format)
            .context("Kunde inte skriva kolumnnamn")?;
    }
    sheet
        .set_freeze_panes(1, 0)
        .context("Kunde inte frysa översta raden")?;
    Ok(())
}

fn is_milestone_age(age: i32) -> bool {
    age >= 50 && age % 5 == 0
}

fn write_person(sheet: &mut Worksheet, person: &shared::Person, row: u32, year: i32) -> Result<()> {
    let age = year - person.born.year();
    let name = format!("{} {}", person.given_name, person.last_name);

    sheet.write(row, 0, person.number)?;
    sheet.write(row, 1, person.born.format("%m-%d").to_string())?;
    sheet.write(row, 2, age)?;
    sheet.write(row, 3, person.born.to_string())?;
    sheet.write(row, 4, name)?;
    sheet.write(row, 5, person.address.as_str())?;
    sheet.write(row, 6, person.zip_code.as_str())?;
    sheet.write(row, 7, person.city.as_str())?;
    Ok(())
}

pub fn run(filename: impl AsRef<Path>, year: Option<i32>) -> Result<()> {
    let year = year.unwrap_or(shared::current_year());

    let mut eligible: Vec<_> = shared::read_csv(filename)?
        .into_iter()
        .filter(|person| is_milestone_age(year - person.born.year()))
        .collect();

    eligible.sort_unstable_by_key(|person| (person.born.month(), person.born.day()));

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    write_column_names(sheet)?;
    for (offset, person) in eligible.iter().enumerate() {
        write_person(sheet, person, u32::try_from(offset + 1)?, year)?;
    }

    sheet.autofit();
    let filename = format!("Gratulera {year}.xlsx");
    workbook
        .save(&filename)
        .context("Kunde inte skriva Excel-filen")?;
    println!("Skapade filen '{filename}'.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_ages_start_at_fifty_and_step_by_five() {
        assert!(is_milestone_age(50));
        assert!(is_milestone_age(75));
        assert!(!is_milestone_age(49));
        assert!(!is_milestone_age(51));
    }
}
