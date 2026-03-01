use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use visual_novel_engine::{
    compute_script_id, run_repro_case, Engine, ReproCase, ResourceLimiter, SaveData,
    ScriptCompiled, ScriptRaw, SecurityPolicy, UiTrace, SCRIPT_SCHEMA_VERSION,
};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about = "Visual Novel Engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a script JSON file.
    Validate { script: PathBuf },
    /// Compile a script JSON file into binary form.
    Compile {
        script: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Produce an execution trace for a script JSON file.
    Trace {
        script: PathBuf,
        #[arg(long, default_value_t = 100)]
        steps: usize,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Verify a save file against a compiled script.
    VerifySave {
        save: PathBuf,
        #[arg(long)]
        script: PathBuf,
    },
    /// Build an asset manifest with sha256 hashes.
    Manifest {
        assets: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Run a local repro-case JSON and evaluate its oracle/monitors.
    ReproRun {
        repro: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
}

#[derive(Serialize)]
struct TraceEnvelope {
    trace_format_version: u16,
    script_schema_version: String,
    trace: UiTrace,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AssetManifest {
    manifest_version: u16,
    assets: std::collections::BTreeMap<String, AssetEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AssetEntry {
    sha256: String,
    size: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { script } => validate_script(&script),
        Command::Compile { script, output } => compile_script(&script, &output),
        Command::Trace {
            script,
            steps,
            output,
        } => trace_script(&script, steps, &output),
        Command::VerifySave { save, script } => verify_save(&save, &script),
        Command::Manifest { assets, output } => build_manifest(&assets, &output),
        Command::ReproRun {
            repro,
            output,
            strict,
        } => run_repro_bundle(&repro, output.as_deref(), strict),
    }
}

fn validate_script(path: &Path) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let script = ScriptRaw::from_json(&raw).context("parse script")?;
    let policy = SecurityPolicy::default();
    let limits = ResourceLimiter::default();
    policy.validate_raw(&script, limits)?;
    let compiled = script.compile()?;
    policy.validate_compiled(&compiled, limits)?;
    Ok(())
}

fn compile_script(path: &Path, output: &Path) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let script = ScriptRaw::from_json(&raw).context("parse script")?;
    let compiled = script.compile()?;
    let bytes = compiled.to_binary()?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, bytes).with_context(|| format!("write {}", output.display()))?;
    Ok(())
}

fn trace_script(path: &Path, steps: usize, output: &Path) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let script = ScriptRaw::from_json(&raw).context("parse script")?;
    let mut engine = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )?;
    let mut trace = UiTrace::new();
    for step in 0..steps {
        let event = match engine.current_event() {
            Ok(event) => event,
            Err(_) => break,
        };
        let view = visual_novel_engine::TraceUiView::from_event(&event);
        let state = visual_novel_engine::StateDigest::from_state(
            engine.state(),
            engine.script().flag_count as usize,
        );
        trace.push(step as u32, view, state);
        match &event {
            visual_novel_engine::EventCompiled::Choice(_) => {
                let _ = engine.choose(0);
            }
            _ => {
                let _ = engine.step();
            }
        }
    }
    let envelope = TraceEnvelope {
        trace_format_version: 1,
        script_schema_version: SCRIPT_SCHEMA_VERSION.to_string(),
        trace,
    };
    let yaml = serde_yaml::to_string(&envelope)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, yaml).with_context(|| format!("write {}", output.display()))?;
    Ok(())
}

fn verify_save(save_path: &Path, script_path: &Path) -> Result<()> {
    let save_bytes =
        fs::read(save_path).with_context(|| format!("read {}", save_path.display()))?;
    let save = SaveData::from_binary(&save_bytes)?;
    let script_bytes =
        fs::read(script_path).with_context(|| format!("read {}", script_path.display()))?;
    let compiled = ScriptCompiled::from_binary(&script_bytes)?;
    let compiled_bytes = compiled.to_binary()?;
    let script_id = compute_script_id(&compiled_bytes);
    save.validate_script_id(&script_id)?;
    Ok(())
}

fn build_manifest(root: &Path, output: &Path) -> Result<()> {
    let mut assets = std::collections::BTreeMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
        let size = bytes.len() as u64;
        let sha256 = sha256_hex(&bytes);
        assets.insert(rel_str, AssetEntry { sha256, size });
    }
    let manifest = AssetManifest {
        manifest_version: 1,
        assets,
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, json).with_context(|| format!("write {}", output.display()))?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn run_repro_bundle(path: &Path, output: Option<&Path>, strict: bool) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let case = ReproCase::from_json(&raw).context("parse repro case")?;
    let report = run_repro_case(&case);

    println!(
        "repro '{}' => stop_reason={} oracle_triggered={} matched_monitors={}",
        case.title,
        report.stop_reason.label(),
        report.oracle_triggered,
        report.matched_monitors.join(",")
    );
    if let Some(event_ip) = report.failing_event_ip {
        println!("failing_event_ip={event_ip}");
    }
    println!("stop_message={}", report.stop_message);

    if let Some(out) = output {
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = report.to_json().context("serialize repro report")?;
        fs::write(out, payload).with_context(|| format!("write {}", out.display()))?;
    }

    if strict && !report.oracle_triggered {
        anyhow::bail!("repro oracle was not triggered");
    }
    Ok(())
}
