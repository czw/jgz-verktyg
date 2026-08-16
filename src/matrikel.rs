use crate::shared;
use anyhow::{Context, Result};
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

pub fn run(filename: impl AsRef<Path>, year: Option<i32>) -> Result<()> {
    let images = get_image_map()?;
    let people = get_sorted_member_list(filename)?;
    let people_per_page: Vec<&[shared::Person]> = people.chunks(10).collect();
    let year = year.unwrap_or(shared::current_year());

    let ctx = context!(images => images, people_per_page => people_per_page, year => year);
    let tmpl = shared::ENV.get_template("matrikel.tmpl")?;
    println!("{}", tmpl.render(ctx)?);
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
