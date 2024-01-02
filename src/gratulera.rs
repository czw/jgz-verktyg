use crate::shared;
use chrono::Datelike;
use rust_xlsxwriter::{Format, Workbook, Worksheet};

fn write_column_names(sheet: &mut Worksheet) {
    // Write the top row containing column names
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
            .write_with_format(0, u16::try_from(i).unwrap(), name, &header_format)
            .expect("Kunde inte skriva kolumnnamn");
    }
    sheet
        .set_freeze_panes(1, 0)
        .expect("Kunde inte frysa översta raden");
}

fn write_person(sheet: &mut Worksheet, person: shared::Person, row: u32, year: i32) {
    let msg = "Kunde inte skriva till Excel-kolumn";
    let age = year - person.born.year();
    let born_string = person.born.to_string();
    let born_day_and_month = born_string.get(5..10).unwrap();
    let name = person.given_name + " " + &person.last_name;

    sheet.write(row, 0, person.number).expect(msg);
    sheet.write(row, 1, born_day_and_month).expect(msg);
    sheet.write(row, 2, age).expect(msg);
    sheet.write(row, 3, born_string).expect(msg);
    sheet.write(row, 4, name).expect(msg);
    sheet.write(row, 5, person.address).expect(msg);
    sheet.write(row, 6, person.zip_code).expect(msg);
    sheet.write(row, 7, person.city).expect(msg);
}

pub fn run(filename: String, year: Option<i32>) {
    let year = year.unwrap_or(shared::current_year());

    // Eliminate everyone below 50 and with an age not divisible by 5
    let mut eligible = Vec::new();
    for person in shared::read_csv(filename) {
        let age = year - person.born.year();
        if age >= 50 && (age % 5) == 0 {
            eligible.push(person);
        }
    }

    // Sort the list by month and day born
    eligible.sort_unstable_by(|a, b| {
        let a = a.born;
        let b = b.born;
        let order = a.month0().partial_cmp(&b.month0()).unwrap();
        if order != std::cmp::Ordering::Equal {
            return order;
        }
        a.day0().partial_cmp(&b.day0()).unwrap()
    });

    // Create the Excel file
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    write_column_names(sheet);
    let mut row = 1;
    for person in eligible {
        write_person(sheet, person, row, year);
        row += 1;
    }

    // Adjust the column sizes and save the file
    sheet.autofit();
    let filename = format!("Gratulera {year}.xlsx");
    workbook
        .save(&filename)
        .expect("Kunde inte skriva Excel-filen");
    println!("Skapade filen '{filename}'.");
}
