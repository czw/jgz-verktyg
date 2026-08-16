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
        match filename.split('_').collect::<Vec<_>>().as_slice() {
            [_, number, rest] if rest.contains('-') => match number.parse::<i16>() {
                Ok(id) => {
                    map.insert(id, filename);
                }
                Err(err) => eprintln!("Warning: Failed to parse id from {} ({err})", filename),
            },
            _ => eprintln!("Warning: Expected two underscores in filename: {filename}"),
        }
    }
    Ok(map)
}

fn get_sorted_member_list(filename: impl AsRef<Path>) -> Result<Vec<shared::Person>> {
    let mut people = shared::read_csv(filename)?;
    people.sort_by(|a, b| {
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
