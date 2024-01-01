use clap::{Parser, Subcommand};
mod cmd;
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
    /// Gör en lista över bröder med minnesvärda födelsedagar
    #[command(arg_required_else_help = true)]
    Gratulera {
        /// CSV-fil med aktuella medlemsuppgifter
        filename: String,
        /// Vilket år som rapporten ska genereras för om ej nuvarande år
        year: Option<i32>,
    },
}

fn main() {
    match Cli::parse().command {
        Commands::Gratulera { filename, year } => {
            cmd::gratulera(filename, year);
        }
    }
}
