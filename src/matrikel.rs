use crate::pdf::{self, HtmlToPdf};
use crate::shared;
use anyhow::{Context, Result};
use headless_chrome::types::PrintToPdfOptions;
use minijinja::context;
use std::collections::HashMap;
use std::path::Path;

fn get_image_map() -> Result<HashMap<i16, String>> {
    // Build a list of filenames that might be uses as portrait images. We add
    // the original image first, then the cropped image made for portrait
    // printing, and finally cropped PNG images that we use when editing.
    const PATTERNS: [&str; 3] = [
        "K1b*/K1b_????_????-??-??.jpg",
        "K1b*/matrikel/K1b_????_????-??-??.jpg",
        "K1b*/matrikel/K1b_????_????-??-??.png",
    ];
    let mut filenames = Vec::new();
    for pattern in PATTERNS {
        let paths =
            glob::glob(pattern).with_context(|| format!("Ogiltigt sökvägsmönster: {pattern}"))?;
        filenames
            .extend(paths.filter_map(|entry| entry.ok().map(|path| path.display().to_string())));
    }
    filenames.sort_unstable();

    let mut map = HashMap::new();
    for filename in filenames {
        match member_number_from_image(&filename) {
            Some(id) => {
                map.insert(id, filename);
            }
            None => eprintln!("Varning: Kunde inte läsa medlemsnummer ur {filename}"),
        }
    }
    Ok(map)
}

fn member_number_from_image(filename: &str) -> Option<i16> {
    let mut parts = filename.split('_');
    let _prefix = parts.next()?;
    let number = parts.next()?.parse().ok()?;
    let rest = parts.next()?;
    rest.contains('-').then_some(number)
}

fn get_sorted_member_list(filename: impl AsRef<Path>) -> Result<Vec<shared::Person>> {
    let mut people = shared::read_csv(filename)?;
    people.sort_unstable_by(|a, b| {
        a.last_name
            .cmp(&b.last_name)
            .then_with(|| a.given_name.cmp(&b.given_name))
    });
    Ok(people)
}

const PEOPLE_PER_PAGE: usize = 10;
const WAIT_FOR_IMAGES_JS: &str = r"
    new Promise((resolve) => {
        const images = Array.from(document.images);
        let pending = images.filter((img) => !img.complete).length;
        if (pending === 0) {
            resolve(true);
            return;
        }
        const done = () => {
            pending -= 1;
            if (pending <= 0) resolve(true);
        };
        for (const img of images) {
            if (img.complete) continue;
            img.addEventListener('load', done, { once: true });
            img.addEventListener('error', done, { once: true });
        }
        setTimeout(() => resolve(true), 15000);
    });
";

pub fn run(filename: impl AsRef<Path>, year: Option<i32>) -> Result<()> {
    let images = get_image_map()?;
    let people = get_sorted_member_list(filename)?;
    let people_per_page: Vec<&[shared::Person]> = people.chunks(PEOPLE_PER_PAGE).collect();
    let year = year.unwrap_or(shared::current_year());

    let ctx = context!(images => images, people_per_page => people_per_page, year => year);
    let html = shared::ENV
        .get_template("matrikel.tmpl")
        .context("Kan inte ladda matrikelmallen")?
        .render(ctx)
        .context("Rendering av matrikelmallen misslyckades")?;

    let pdf = pdf::html_to_pdf(
        &html,
        format!(".matrikel-{year}.html"),
        HtmlToPdf {
            evaluate_js: Some(WAIT_FOR_IMAGES_JS),
            print: PrintToPdfOptions {
                print_background: Some(true),
                ..HtmlToPdf::default().print
            },
            ..Default::default()
        },
    )?;

    let filename = format!("Matrikel {year}.pdf");
    std::fs::write(&filename, pdf).context("Kunde inte skriva PDF-filen")?;
    println!("Skapade filen '{filename}'.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_member_number_from_portrait_filename() {
        assert_eq!(
            member_number_from_image("K1b/K1b_1754_2025-11-22.jpg"),
            Some(1754)
        );
        assert_eq!(
            member_number_from_image("K1b/matrikel/K1b_1810_2025-12-13.png"),
            Some(1810)
        );
        assert_eq!(member_number_from_image("portrait.jpg"), None);
    }
}
