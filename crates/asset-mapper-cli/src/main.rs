use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use asset_mapper_core::{AssemblyPlan, LlmBundle, PackRecord, resolve_plan, validate_pack};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "asset-mapper")]
#[command(about = "Headless Asset Mapper Phase 0 proof harness")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate { pack: PathBuf },
    Bundle { pack: PathBuf },
    Resolve { pack: PathBuf, plan: PathBuf },
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
        Commands::Validate { pack } => {
            let pack = read_pack(pack)?;
            let report = validate_pack(&pack);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.is_valid() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Commands::Bundle { pack } => {
            let pack = read_pack(pack)?;
            let bundle = LlmBundle::from_pack(&pack);
            println!("{}", serde_json::to_string_pretty(&bundle)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Resolve { pack, plan } => {
            let pack = read_pack(pack)?;
            let plan = read_plan(plan)?;
            let scene = resolve_plan(&pack, &plan)?;
            println!("{}", serde_json::to_string_pretty(&scene)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn read_pack(path: PathBuf) -> Result<PackRecord, Box<dyn std::error::Error>> {
    let input = fs::read_to_string(resolve_input_path(&path).unwrap_or(path))?;
    Ok(serde_json::from_str(&input)?)
}

fn read_plan(path: PathBuf) -> Result<AssemblyPlan, Box<dyn std::error::Error>> {
    let input = fs::read_to_string(resolve_input_path(&path).unwrap_or(path))?;
    Ok(serde_json::from_str(&input)?)
}

fn resolve_input_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() || path.exists() {
        return Some(path.to_path_buf());
    }

    let current_dir = std::env::current_dir().ok()?;
    current_dir
        .ancestors()
        .map(|ancestor| ancestor.join(path))
        .find(|candidate| candidate.exists())
}
