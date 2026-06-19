#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "shape-cli")]
#[command(about = "Headless Shape Lab tooling")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Placeholder demo generation command.
    Demo,
    /// Placeholder project validation command.
    Validate,
    /// Placeholder OBJ export command.
    Export,
}

fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Demo) => {
            println!("demo command is not implemented in Wave 0");
        }
        Some(Command::Validate) => {
            println!("validate command is not implemented in Wave 0");
        }
        Some(Command::Export) => {
            println!("export command is not implemented in Wave 0");
        }
        None => {
            println!("Run with --help to see placeholder commands.");
        }
    }
    Ok(())
}
