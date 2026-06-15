use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use asset_mapper_core::{AssemblyPlan, LlmBundle, resolve_plan, validate_pack};
use asset_mapper_io::{index_pack_folder, init_pack_folder, read_pack_from_input};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "asset-mapper")]
#[command(about = "Headless Asset Mapper metadata tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        folder: PathBuf,
        #[arg(long)]
        name: String,
    },
    Index {
        folder: PathBuf,
    },
    Validate {
        pack: PathBuf,
    },
    Bundle {
        pack: PathBuf,
    },
    Resolve {
        pack: PathBuf,
        plan: PathBuf,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { folder, name } => {
            let report = init_pack_folder(folder, name)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Index { folder } => {
            let report = index_pack_folder(folder)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Validate { pack } => {
            let loaded = read_pack_from_input(pack)?;
            let report = validate_pack(&loaded.pack);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.is_valid() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Commands::Bundle { pack } => {
            let loaded = read_pack_from_input(pack)?;
            let bundle = LlmBundle::from_pack(&loaded.pack);
            println!("{}", serde_json::to_string_pretty(&bundle)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Resolve { pack, plan } => {
            let loaded = read_pack_from_input(pack)?;
            let plan = read_plan(plan)?;
            let scene = resolve_plan(&loaded.pack, &plan)?;
            println!("{}", serde_json::to_string_pretty(&scene)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn read_plan(path: PathBuf) -> Result<AssemblyPlan, Box<dyn std::error::Error>> {
    let input = fs::read_to_string(&path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("failed to read plan {}: {error}", path.display()),
        )
    })?;
    Ok(serde_json::from_str(&input)?)
}
