use crate::shared;
use minijinja::context;
use std::collections::HashMap;

fn get_image_map() -> Result<HashMap<i16, String>, Box<dyn std::error::Error>> {
    // Build a list of filenames that might be uses as portrait images. We add
    // the original image first, then the cropped image made for portrait
    // printing, and finally cropped PNG images that we use when editing.
    const PATTERNS: [&str; 3] = [
        "K1b*/K1b_????_????-??-??.jpg",
        "K1b*/matrikel/K1b_????_????-??-??.jpg",
        "K1b*/matrikel/K1b_????_????-??-??.png",
    ];
    let mut filenames = PATTERNS
        .into_iter()
        .flat_map(|pattern| {
            glob::glob(pattern)
                .expect("Failed to read glob pattern")
                .filter_map(|entry| entry.ok().map(|path| path.display().to_string()))
        })
        .collect::<Vec<_>>();
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

fn get_sorted_member_list(filename: String) -> Vec<shared::Person> {
    let mut people = shared::read_csv(filename);
    people.sort_by(|a, b| {
        a.last_name
            .cmp(&b.last_name)
            .then_with(|| a.given_name.cmp(&b.given_name))
    });
    people
}

pub fn run(filename: String, year: Option<i32>) -> Result<(), Box<dyn std::error::Error>> {
    let images = get_image_map()?;
    let people = get_sorted_member_list(filename);
    let people_per_page: Vec<&[shared::Person]> = people.chunks(10).collect();
    let year = year.unwrap_or(shared::current_year());

    let ctx = context!(images => images, people_per_page => people_per_page, year => year);
    let tmpl = shared::ENV.get_template("matrikel.tmpl")?;
    println!("{}", tmpl.render(ctx)?);
    Ok(())
}
