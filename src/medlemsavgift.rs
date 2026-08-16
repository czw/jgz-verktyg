use crate::shared::{self, MemberFeeLevel, Person};
use anyhow::{Context, Error, Result};
use headless_chrome::{Browser, LaunchOptions, types::PrintToPdfOptions};
use indicatif::{ProgressBar, ProgressStyle};
use lettre::{
    Address, Message, SmtpTransport, Transport,
    message::{Attachment, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::{File, OpenOptions, read_to_string, remove_file, write},
    io::{self, Write},
    path::Path,
    time::{Duration, Instant},
};

const SENT_FILENAME: &str = "invoices_sent_to.txt";
const NORMAL_YEARLY_FEE: i32 = 350;
const REDUCED_YEARLY_FEE: i32 = 100;
const A4_PAPER_WIDTH_INCHES: f64 = 8.27;
const A4_PAPER_HEIGHT_INCHES: f64 = 11.69;
const MAIL_BODY: &str = r#"Bäste broder!

Det är dags att betala din medlemsavgift i Jämtlands Gille. Vi hoppas att du
betalar skyndsamt och att vi ses under våra ordinarie eller extra möten.

Nästa extramöte är den 21:a augusti där vi har en pubkväll där respektive är
inbjudna, och med möjlighet att ballotera in nya bröder. Nästa ordinarie möte
är den 19:e september. Mer information finns på vår hemsida
https://www.jgz.se där exakta klockslag också finns."#;
const WAIT_FOR_LOGO_JS: &str = r#"
    new Promise((resolve) => {
        const img = document.querySelector('.logo-container img');
        if (!img || img.complete) {
            resolve(true);
            return;
        }
        img.addEventListener('load', () => resolve(true), { once: true });
        img.addEventListener('error', () => resolve(true), { once: true });
    });
"#;

struct Mailer {
    from: Mailbox,
    transport: SmtpTransport,
}

impl Mailer {
    fn send(&self, message: &Message) -> Result<()> {
        self.transport
            .send(message)
            .context("Kunde inte skicka e-post")?;
        Ok(())
    }
}

fn get_from_env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("Hittade inte miljövariabeln {name}"))
}

fn build_mailer() -> Result<Mailer> {
    let from_name = get_from_env("SMTP_FROM_NAME")?;
    let from_email = get_from_env("SMTP_FROM_EMAIL")?;
    let from_address = from_email
        .parse::<Address>()
        .with_context(|| format!("Ogiltig SMTP_FROM_EMAIL: {from_email}"))?;

    let server = get_from_env("SMTP_SERVER")?;
    let credentials = Credentials::new(
        get_from_env("SMTP_USERNAME")?,
        get_from_env("SMTP_PASSWORD")?,
    );
    let transport = SmtpTransport::starttls_relay(&server)
        .with_context(|| format!("Kunde inte konfigurera SMTP-reläet {server}"))?
        .port(587)
        .credentials(credentials)
        .build();

    Ok(Mailer {
        from: Mailbox::new(Some(from_name), from_address),
        transport,
    })
}

#[derive(Serialize)]
struct MailingInfo {
    // -- Things we know about this person
    address: String,
    city: String,
    email: String,
    #[serde(serialize_with = "serialize_mailboxes")]
    email_addresses: Vec<Mailbox>,
    name: String,
    number: i16,
    phone1: String,
    phone2: String,
    zip_code: String,

    // -- Things related to this payment
    /// Does this person have previous outstanding payments?
    payment_outstanding: bool,
    /// Long and loyal membership leads to reduced payments
    payment_reduced: bool,
    /// The total payment that we want
    payment_sum: i32,
}

fn serialize_mailboxes<S>(mailboxes: &[Mailbox], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    mailboxes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .serialize(serializer)
}

impl MailingInfo {
    fn try_new(person: Person) -> Result<Self> {
        let name = format_name(&person.given_name, &person.last_name);
        let email_addresses = parse_email_addresses(&name, &person.email)?;

        let payment_reduced = person.member_fee_level == MemberFeeLevel::Reduced;
        let years_to_pay =
            shared::current_year() - person.last_member_fee_paid.last_paid_year as i32;
        if years_to_pay == 0 {
            return Err(Error::msg("Redan betalt"));
        }

        let yearly_fee = if payment_reduced {
            REDUCED_YEARLY_FEE
        } else {
            NORMAL_YEARLY_FEE
        };

        Ok(Self {
            address: person.address,
            city: person.city.to_uppercase(),
            email: person.email,
            email_addresses,
            name,
            number: person.number,
            payment_outstanding: years_to_pay > 1,
            payment_reduced,
            payment_sum: yearly_fee * years_to_pay,
            phone1: person.phone1,
            phone2: person.phone2,
            zip_code: person.zip_code,
        })
    }
}

fn format_name(given_name: &str, last_name: &str) -> String {
    let mut result = String::with_capacity(given_name.len() + last_name.len() + 1);
    let mut capitalize_next = true;

    for c in given_name.chars().chain([' ']).chain(last_name.chars()) {
        if c.is_alphabetic() {
            let mapped = if capitalize_next {
                c.to_uppercase().collect::<String>()
            } else {
                c.to_lowercase().collect::<String>()
            };
            result.push_str(&mapped);
            capitalize_next = false;
        } else {
            result.push(c);
            capitalize_next = c == '-' || c.is_whitespace();
        }
    }

    result
}

fn parse_email_addresses(name: &str, email: &str) -> Result<Vec<Mailbox>> {
    email
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|address| {
            let address = address
                .parse::<Address>()
                .with_context(|| format!("Ogiltig e-postadress: {address:?}"))?;
            Ok(Mailbox::new(Some(name.to_string()), address))
        })
        .collect()
}

fn build_mailing_info(filename: impl AsRef<Path>) -> Result<Vec<MailingInfo>> {
    shared::read_csv(filename)?
        .into_iter()
        .filter(|p| !p.email.is_empty() && p.member_fee_level != MemberFeeLevel::Free)
        .map(MailingInfo::try_new)
        .collect()
}

fn read_sent_numbers() -> Result<HashSet<i16>> {
    let contents = match read_to_string(SENT_FILENAME) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };

    Ok(contents
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter_map(|number| number.parse::<i16>().ok())
        .collect())
}

fn generate_pdf(info: &MailingInfo) -> Result<Vec<u8>> {
    let today = shared::current_date().format("%Y-%m-%d").to_string();
    let ctx = minijinja::context!(info => info, today => today);
    let template = shared::ENV
        .get_template("faktura.html")
        .context("Kan inte ladda mallen")?
        .render(ctx)
        .context("Rendering av mallen misslyckades")?;

    let html_path = std::env::temp_dir().join(format!("faktura_{}.html", info.number));
    write(&html_path, &template).context("Kunde inte skriva HTML-filen")?;

    let browser = Browser::new(LaunchOptions::default()).context("Kan inte starta webbläsaren")?;
    let tab = browser.new_tab().context("Kan inte skapa en ny flik")?;
    tab.navigate_to(&format!("file://{}", html_path.display()))?;
    tab.wait_until_navigated()?;

    tab.wait_for_element("#qr-code canvas")
        .context("QR-koden renderades inte")?;
    tab.evaluate(WAIT_FOR_LOGO_JS, true)
        .context("Kunde inte vänta på logotypen")?;

    let pdf = tab.print_to_pdf(Some(PrintToPdfOptions {
        paper_width: Some(A4_PAPER_WIDTH_INCHES),
        paper_height: Some(A4_PAPER_HEIGHT_INCHES),
        prefer_css_page_size: Some(true),
        ..Default::default()
    }))?;

    tab.close_with_unload()?;
    remove_file(&html_path).ok();

    Ok(pdf)
}

fn send_invoice(
    mailer: &Mailer,
    subject: &str,
    mail_body: &SinglePart,
    info: &MailingInfo,
    sent_file: &mut File,
    mail_interval: Duration,
    progress: &ProgressBar,
) -> Result<()> {
    let started = Instant::now();

    progress.set_message(info.name.clone());
    progress.force_draw();

    let pdf = generate_pdf(info)?;
    let attachment =
        Attachment::new(String::from("faktura.pdf")).body(pdf, "application/pdf".parse()?);

    for address in &info.email_addresses {
        let email = Message::builder()
            .from(mailer.from.clone())
            .to(address.clone())
            .subject(subject)
            .multipart(
                MultiPart::mixed()
                    .singlepart(mail_body.clone())
                    .singlepart(attachment.clone()),
            )
            .context("Kunde inte skapa e-postmeddelandet")?;

        mailer.send(&email)?;
        writeln!(sent_file, "{} {address}", info.number)?;
        sent_file.flush()?;
    }

    let elapsed = started.elapsed();
    if elapsed < mail_interval {
        std::thread::sleep(mail_interval - elapsed);
    }

    progress.inc(1);
    Ok(())
}

pub fn run(filename: impl AsRef<Path>) -> Result<()> {
    let sent_numbers = read_sent_numbers()?;
    let mailer = build_mailer()?;
    let mail_interval = Duration::from_secs(get_from_env("SMTP_SEND_INTERVAL")?.parse()?);

    let subject = format!("JG: Medlemsavgift {}", shared::current_year());
    let mail_body = SinglePart::plain(MAIL_BODY.to_string());

    let mailing_info = build_mailing_info(&filename)?
        .into_iter()
        .filter(|info| !sent_numbers.contains(&info.number))
        .collect::<Vec<_>>();

    let mut sent_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SENT_FILENAME)?;

    let progress = ProgressBar::new(mailing_info.len().try_into()?);
    progress.set_style(
        ProgressStyle::with_template("{bar:56.cyan/blue} {eta} {msg}")
            .expect("Ogiltig progressbar-mall"),
    );

    for info in mailing_info {
        send_invoice(
            &mailer,
            &subject,
            &mail_body,
            &info,
            &mut sent_file,
            mail_interval,
            &progress,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_names_in_title_case() {
        assert_eq!(format_name("erik", "lundberg"), "Erik Lundberg");
        assert_eq!(
            format_name("ERIK-GUSTAF", "LUNDBERG"),
            "Erik-Gustaf Lundberg"
        );
        assert_eq!(format_name("öRJAN", "von-BERGEN"), "Örjan Von-Bergen");
    }

    #[test]
    fn parses_comma_separated_email_addresses() {
        let mailboxes =
            parse_email_addresses("Erik Lundberg", "erik@example.com,  erik2@example.com ,")
                .unwrap();

        assert_eq!(mailboxes.len(), 2);
        assert_eq!(mailboxes[0].email.to_string(), "erik@example.com");
        assert_eq!(mailboxes[1].email.to_string(), "erik2@example.com");
    }

    #[test]
    fn rejects_invalid_email_address() {
        assert!(parse_email_addresses("Erik Lundberg", "inte en epost").is_err());
    }

    #[test]
    fn ignores_blank_email_addresses() {
        assert!(
            parse_email_addresses("Erik Lundberg", "  , ,")
                .unwrap()
                .is_empty()
        );
    }
}
