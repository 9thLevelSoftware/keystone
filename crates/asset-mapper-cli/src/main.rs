use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use asset_mapper_core::{
    AnalyzeOptions, AssemblyPlan, LlmBundle, ProposeAssemblyOptions, export_connectors_csv,
    export_godot, export_unity, export_unreal, gltf_keystone_extras, propose_assembly_plan,
    resolve_plan, validate_pack, vibe_readiness,
};
use asset_mapper_io::{
    InitPackOptions, PackInputKind, accept_hash_drift, analyze_pack_folder, index_pack_folder,
    init_pack_folder, measure_pack_bounds, migrate_pack_input, read_pack_from_input,
    validate_pack_sources,
};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "asset-mapper")]
#[command(about = "Headless Asset Mapper metadata tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, ValueEnum)]
enum EngineTarget {
    Unreal,
    Unity,
    Godot,
    Csv,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        folder: PathBuf,
        #[arg(long)]
        name: String,
        /// SPDX or human license summary (required; must not be UNSPECIFIED).
        #[arg(long)]
        license: String,
        /// Pack author organization or person (at least one of --author / --source).
        #[arg(long)]
        author: Option<String>,
        /// Pack source URL or origin label (at least one of --author / --source).
        #[arg(long)]
        source: Option<String>,
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
    /// Accept content-hash drift after review (update hashes; keep connectors by default).
    AcceptDrift {
        folder: PathBuf,
        /// Limit to specific asset ids or source paths (repeatable).
        #[arg(long = "asset")]
        assets: Vec<String>,
        /// Clear connectors when accepting drift (default: keep connectors).
        #[arg(long, default_value_t = false)]
        clear_connectors: bool,
    },
    /// Re-measure mesh/image bounds and clear BoundsPlaceholder flags.
    MeasureBounds {
        folder: PathBuf,
    },
    /// Measure bounds and auto-propose connectors, classes, and compatibility rules.
    Analyze {
        folder: PathBuf,
        /// Replace existing connectors instead of skipping assets that already have them.
        #[arg(long, default_value_t = false)]
        replace: bool,
        /// Use AABB face centers only (skip mesh socket detection).
        #[arg(long, default_value_t = false)]
        aabb_only: bool,
        /// Skip assets whose source_path matches this glob (repeatable). Example: Decals/**, *.png
        #[arg(long = "exclude-glob")]
        exclude_globs: Vec<String>,
        /// Propose connectors on image/sprite assets (default: skip images).
        #[arg(long, default_value_t = false)]
        include_images: bool,
    },
    /// Propose a multi-piece assembly plan from pack connectors and rules.
    ProposeAssembly {
        pack: PathBuf,
        /// Maximum pieces including root (default 8).
        #[arg(long, default_value_t = 8)]
        max_pieces: usize,
        /// Root asset id (default: asset with most connectors).
        #[arg(long)]
        root: Option<String>,
        /// Document tile reuse intent (resolve still unique-asset; see notes).
        #[arg(long, default_value_t = false)]
        allow_asset_reuse: bool,
        /// Soft guidance for external tile placement (default 1).
        #[arg(long, default_value_t = 1)]
        max_instances_per_asset: usize,
        /// Write plan JSON to this path (default: stdout).
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Report vibe-builder readiness (coverage, orphans, connectivity, score).
    VibeReady {
        pack: PathBuf,
    },
    /// Migrate pack sidecar to the current schema version.
    Migrate {
        pack: PathBuf,
    },
    /// Export engine-friendly connector/rule tables.
    ExportEngine {
        pack: PathBuf,
        #[arg(long, value_enum)]
        target: EngineTarget,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Write glTF Keystone extras companion JSON (`*.keystone.json`).
    ExportGltfExtras {
        pack: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
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
        Commands::Init {
            folder,
            name,
            license,
            author,
            source,
        } => {
            let report = init_pack_folder(
                folder,
                InitPackOptions {
                    display_name: name,
                    license_summary: license,
                    author,
                    source,
                },
            )?;
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
            let mut report = validate_pack(&loaded.pack);
            if loaded.resolved.kind == PackInputKind::PackFolder {
                if let Some(pack_root) = loaded.resolved.pack_root.as_deref() {
                    let source_report = validate_pack_sources(pack_root, &loaded.pack)?;
                    report.extend(source_report.diagnostics);
                }
            }
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
            match resolve_plan(&loaded.pack, &plan) {
                Ok(scene) => {
                    println!("{}", serde_json::to_string_pretty(&scene)?);
                    Ok(ExitCode::SUCCESS)
                }
                Err(error) => {
                    let report = error.to_report();
                    eprintln!("{}", serde_json::to_string_pretty(&report)?);
                    Ok(ExitCode::from(1))
                }
            }
        }
        Commands::AcceptDrift {
            folder,
            assets,
            clear_connectors,
        } => {
            let asset_filter = if assets.is_empty() {
                None
            } else {
                Some(assets)
            };
            let report = accept_hash_drift(folder, asset_filter, clear_connectors)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::MeasureBounds { folder } => {
            let report = measure_pack_bounds(folder)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Analyze {
            folder,
            replace,
            aabb_only,
            exclude_globs,
            include_images,
        } => {
            let report = analyze_pack_folder(
                folder,
                AnalyzeOptions {
                    replace_existing_connectors: replace,
                    aabb_only,
                    exclude_globs,
                    skip_images: !include_images,
                    ..AnalyzeOptions::default()
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::ProposeAssembly {
            pack,
            max_pieces,
            root,
            allow_asset_reuse,
            max_instances_per_asset,
            output,
        } => {
            let loaded = read_pack_from_input(pack)?;
            let report = propose_assembly_plan(
                &loaded.pack,
                &ProposeAssemblyOptions {
                    max_pieces,
                    root_asset_id: root,
                    allow_asset_reuse,
                    max_instances_per_asset,
                    ..ProposeAssemblyOptions::default()
                },
            );
            let body = serde_json::to_string_pretty(&report)?;
            write_or_print(output, body)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::VibeReady { pack } => {
            let loaded = read_pack_from_input(pack)?;
            let report = vibe_readiness(&loaded.pack);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.ready {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Commands::Migrate { pack } => {
            let report = migrate_pack_input(pack)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::ExportEngine {
            pack,
            target,
            output,
        } => {
            let loaded = read_pack_from_input(pack)?;
            let body = match target {
                EngineTarget::Unreal => serde_json::to_string_pretty(&export_unreal(&loaded.pack))?,
                EngineTarget::Unity => serde_json::to_string_pretty(&export_unity(&loaded.pack))?,
                EngineTarget::Godot => serde_json::to_string_pretty(&export_godot(&loaded.pack))?,
                EngineTarget::Csv => export_connectors_csv(&loaded.pack),
            };
            write_or_print(output, body)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::ExportGltfExtras { pack, output } => {
            let loaded = read_pack_from_input(&pack)?;
            let extras = gltf_keystone_extras(&loaded.pack);
            let body = serde_json::to_string_pretty(&extras)?;
            let output = match output {
                Some(path) => path,
                None => {
                    // Default companion path next to sidecar or pack root.
                    if let Some(root) = loaded.resolved.pack_root {
                        root.join(format!("{}.keystone.json", loaded.pack.pack_id))
                    } else {
                        PathBuf::from(format!("{}.keystone.json", loaded.pack.pack_id))
                    }
                }
            };
            fs::write(&output, format!("{body}\n"))?;
            println!(
                "{}",
                serde_json::json!({
                    "output_path": output.to_string_lossy(),
                    "schema": extras.schema,
                    "pack_id": extras.pack_id,
                })
            );
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn write_or_print(output: Option<PathBuf>, body: String) -> Result<(), Box<dyn std::error::Error>> {
    match output {
        Some(path) => {
            let content = if body.ends_with('\n') {
                body
            } else {
                format!("{body}\n")
            };
            fs::write(path, content)?;
        }
        None => println!("{body}"),
    }
    Ok(())
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
