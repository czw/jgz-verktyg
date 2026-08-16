use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod adresskoll;
mod gratulera;
mod matrikel;
mod matrikelstatistik;
mod medlemsavgift;
mod pdf;
mod shared;

#[derive(Debug, Parser)]
#[command(name = "jgz-verktyg")]
#[command(about = "Utför diverse uppgifter för Jämtlands Gille", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Kontrollera att brödernas adresser är korrekt
    #[command(arg_required_else_help = true)]
    Adresskoll {
        /// CSV-fil med aktuella medlemsuppgifter
        filename: PathBuf,
    },
    /// Gör en lista över bröder med minnesvärda födelsedagar
    #[command(arg_required_else_help = true)]
    Gratulera {
        /// CSV-fil med aktuella medlemsuppgifter
        filename: PathBuf,
        /// Vilket år som rapporten ska genereras för om ej nuvarande år
        year: Option<i32>,
    },
    /// Generera en medlemsmatrikel som PDF
    #[command(arg_required_else_help = true)]
    Matrikel {
        /// CSV-fil med aktuella medlemsuppgifter
        filename: PathBuf,
        /// Vilket år som matrikeln ska genereras för om ej nuvarande år
        year: Option<i32>,
    },
    /// Generera statistik om medlemmar för tryck i matrikeln
    #[command(arg_required_else_help = true)]
    Matrikelstatistik {
        /// CSV-fil med aktuella medlemsuppgifter
        filename: PathBuf,
    },
    /// Generera fakturor för medlemsavgift
    #[command(arg_required_else_help = true)]
    Medlemsavgift {
        /// CSV-fil med aktuella medlemsuppgifter
        filename: PathBuf,
    },
}

fn load_dotenv() -> Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(error) if error.not_found() => Ok(()),
        Err(error) => Err(error).context("Fel vid läsning av .env-fil"),
    }
}

fn main() -> Result<()> {
    load_dotenv()?;
    match Cli::parse().command {
        Commands::Adresskoll { filename } => adresskoll::run(filename),
        Commands::Gratulera { filename, year } => gratulera::run(filename, year),
        Commands::Matrikel { filename, year } => matrikel::run(filename, year),
        Commands::Matrikelstatistik { filename } => matrikelstatistik::run(filename),
        Commands::Medlemsavgift { filename } => medlemsavgift::run(filename),
    }
}
