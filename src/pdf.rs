use anyhow::{Context, Result};
use headless_chrome::{Browser, LaunchOptions, types::PrintToPdfOptions};
use std::{
    fs::{remove_file, write},
    path::Path,
    time::Duration,
};

pub const A4_PAPER_WIDTH_INCHES: f64 = 8.27;
pub const A4_PAPER_HEIGHT_INCHES: f64 = 11.69;

pub struct HtmlToPdf {
    pub wait_for_selector: Option<&'static str>,
    pub evaluate_js: Option<&'static str>,
    pub print: PrintToPdfOptions,
}

impl Default for HtmlToPdf {
    fn default() -> Self {
        Self {
            wait_for_selector: None,
            evaluate_js: None,
            print: PrintToPdfOptions {
                paper_width: Some(A4_PAPER_WIDTH_INCHES),
                paper_height: Some(A4_PAPER_HEIGHT_INCHES),
                prefer_css_page_size: Some(true),
                ..Default::default()
            },
        }
    }
}

pub fn html_to_pdf(html: &str, html_path: impl AsRef<Path>, options: HtmlToPdf) -> Result<Vec<u8>> {
    let html_path = html_path.as_ref();
    write(html_path, html)
        .with_context(|| format!("Kunde inte skriva HTML-filen {}", html_path.display()))?;

    let result = render_written_html(html_path, options);
    remove_file(html_path).ok();
    result
}

fn render_written_html(html_path: &Path, options: HtmlToPdf) -> Result<Vec<u8>> {
    let launch = LaunchOptions::default_builder()
        .idle_browser_timeout(Duration::from_secs(300))
        .build()
        .context("Kunde inte konfigurera webbläsaren")?;
    let browser = Browser::new(launch).context("Kan inte starta webbläsaren")?;
    let tab = browser.new_tab().context("Kan inte skapa en ny flik")?;

    let abs_path = html_path
        .canonicalize()
        .with_context(|| format!("Kunde inte hitta {}", html_path.display()))?;
    tab.navigate_to(&format!("file://{}", abs_path.display()))
        .context("Kunde inte öppna HTML-filen")?;
    tab.wait_until_navigated()?;

    if let Some(selector) = options.wait_for_selector {
        tab.wait_for_element(selector)
            .with_context(|| format!("{selector} renderades inte"))?;
    }
    if let Some(js) = options.evaluate_js {
        tab.evaluate(js, true)
            .context("Kunde inte vänta på att sidan skulle bli klar")?;
    }

    let pdf = tab
        .print_to_pdf(Some(options.print))
        .context("Kunde inte skapa PDF")?;
    tab.close_with_unload()?;
    Ok(pdf)
}
