//! Agent-first CLI for `rsysdsl` (inspired by `nomograph-ai/sysml`).
//!
//! Conventions:
//! - Compact JSON on `stdout` by default (`--pretty` for indented JSON).
//! - Human/tool diagnostics strictly on `stderr`.
//! - Exit codes: `0` = ok, `1` = validation/audit/roundtrip/verification failure, `2` = IO/parse error.

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sysml_emit::{
    audit_model_with_lib, audit_units_with_lib, build_safety_report, build_trace_matrix,
    build_viewer_snapshot, discover_omg_stdlib, emit_model, iso26262_metadata_package,
    lint_model, reconcile_model, render_lint_markdown, render_reconcile_markdown,
    render_safety_report_markdown, render_standalone_html, render_trace_matrix_csv,
    render_trace_matrix_markdown, validate_sysml_units, validate_sysml_units_with_lib, KgExport,
    ReconcileSeverity, ViewerOptions, ViewerTheme, ViewerView,
};
mod scaffold;

use sysml_lift::{
    compare_sysml_roundtrip, generate_rust_dsl, lift_sysml_units, CodegenOptions,
};

#[derive(Parser)]
#[command(
    name = "rsysdsl",
    version,
    about = "Bidirectional Rust DSL <-> SysML v2 (System as Code) CLI"
)]
struct Cli {
    /// Pretty-print JSON output instead of compact single-line JSON.
    #[arg(long, global = true)]
    pretty: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deterministically emit OMG-valid SysML v2 (.sysml) from a Rust DSL model.
    Emit {
        /// Built-in model name to emit.
        #[arg(long, default_value = "apollo11-model")]
        model: String,

        /// Write emitted `.sysml` text to a file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Print raw `.sysml` text to stdout instead of a JSON envelope.
        #[arg(long)]
        raw: bool,
    },

    /// Lift existing `.sysml` files into compilable Rust DSL (`#[sysml::*]`) source code.
    Lift {
        /// Input `.sysml` file paths.
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Write generated `.rs` source to a file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include a `main()` that re-emits `.sysml` to stdout.
        #[arg(long)]
        emit_main: bool,
    },

    /// Check bidirectional SysML v2 -> IR/Rust DSL -> SysML v2 round-trip fidelity.
    RoundtripCheck {
        /// Input `.sysml` file paths.
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Optional path to write the re-emitted `.sysml` output.
        #[arg(long)]
        reemitted_out: Option<PathBuf>,
    },

    /// Execute registered closed-loop verification scenarios (sysml-sim) and emit JSON verdicts.
    Verify {
        /// Scenario name or alias (see `--list`); `all` runs every registered scenario.
        #[arg(long, default_value = "nominal")]
        scenario: String,
        /// List registered scenarios as JSON and exit.
        #[arg(long)]
        list: bool,
        /// Write the sysml-sim run (JSON time-series trace + per-requirement verdicts) here.
        #[arg(long)]
        trace: Option<PathBuf>,
    },

    /// Audit semantic validity, orphan requirements, and verification traceability completeness.
    Audit {
        /// Built-in model name to audit (ignored when `--files` is provided).
        #[arg(long, default_value = "apollo11-model")]
        model: String,

        /// Optional `.sysml` input files to validate and audit directly instead of a built-in model.
        #[arg(long, num_args = 1..)]
        files: Vec<PathBuf>,

        /// Also run executable verification and require PASS verdicts for all constrained requirements.
        #[arg(long)]
        verify: bool,

        /// Auto-discover and load the official OMG Standard Library (`sysml.library`).
        #[arg(long)]
        stdlib: bool,

        /// Explicit path to the OMG Standard Library (`sysml.library`) directory.
        #[arg(long)]
        lib: Option<PathBuf>,

        /// Fail with exit code 1 if any Open-MBEE validation warnings are emitted (and if any
        /// TBD is overdue at `--maturity`).
        #[arg(long)]
        strict: bool,

        /// Design maturity: every TBD whose `resolve_by` is at or before this phase is overdue
        /// (an error in `--strict`). `release` therefore allows no TBDs.
        #[arg(long, value_enum, default_value = "concept")]
        maturity: CliPhase,
    },

    /// Print the TBD ledger (every escape hatch with its reason, owner and resolve-by phase).
    Tbd {
        /// Built-in model name (ignored when `--files` is provided).
        #[arg(long, default_value = "apollo11-model")]
        model: String,

        /// `.sysml` input files to lift (emitted by a model's `emit` binary).
        #[arg(long, num_args = 1..)]
        files: Vec<PathBuf>,

        /// Output format: json or markdown.
        #[arg(long, value_enum, default_value = "json")]
        format: LintFormat,

        /// Optional file path to write the rendered ledger.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// TBD baseline JSON (a ledger written by `--write-baseline`), used by `--check`.
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Ratchet: exit 1 if the TBD count (total or any kind) rose above `--baseline`, printing
        /// the new entries. Raising the baseline must be a deliberate change in the same commit.
        #[arg(long, requires = "baseline")]
        check: bool,

        /// Write the current ledger to `--baseline` (creates or overwrites it).
        #[arg(long, requires = "baseline", conflicts_with = "check")]
        write_baseline: bool,
    },

    /// Generate an ASPICE bidirectional traceability matrix (JSON, Markdown, or CSV).
    TraceMatrix {
        /// Built-in model name to trace (ignored when `--files` is provided).
        #[arg(long, default_value = "apollo11-model")]
        model: String,

        /// Optional `.sysml` input files to lift and trace directly instead of a built-in model.
        #[arg(long, num_args = 1..)]
        files: Vec<PathBuf>,

        /// Output format: json, markdown, or csv.
        #[arg(long, value_enum, default_value = "json")]
        format: TraceFormat,

        /// Run executable verification on the built-in model and populate pass/fail verdicts & evidence.
        #[arg(long)]
        verify: bool,

        /// Optional file path to write the rendered traceability matrix.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate `.sysml` files using the embedded Open-MBEE `sysml-toolkit` oracle.
    Validate {
        /// Input `.sysml` file paths.
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Auto-discover and load the official OMG Standard Library (`sysml.library`).
        #[arg(long)]
        stdlib: bool,

        /// Explicit path to the OMG Standard Library (`sysml.library`) directory.
        #[arg(long)]
        lib: Option<PathBuf>,

        /// Fail with exit code 1 if any Open-MBEE validation warnings are emitted.
        #[arg(long)]
        strict: bool,
    },

    /// Generate an ISO 26262 Safety, FMEA Ranking & Minimal Fault Cut-Set report (JSON or Markdown).
    SafetyReport {
        /// Built-in model name (ignored when `--files` is provided).
        #[arg(long, default_value = "embedded-flight-controller")]
        model: String,

        /// Optional `.sysml` input files to lift and analyze directly.
        #[arg(long, num_args = 1..)]
        files: Vec<PathBuf>,

        /// Output format: json or markdown.
        #[arg(long, value_enum, default_value = "json")]
        format: SafetyFormat,

        /// Optional file path to write the rendered report.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Lint a model for architecture, hierarchy, reuse, and physical-modeling smells.
    Lint {
        /// Built-in model name (ignored when `--files` is provided).
        #[arg(long, default_value = "apollo11-model")]
        model: String,

        /// Optional `.sysml` input files to lift and lint directly.
        #[arg(long, num_args = 1..)]
        files: Vec<PathBuf>,

        /// Output format: json or markdown.
        #[arg(long, value_enum, default_value = "json")]
        format: LintFormat,

        /// Fail with exit code 1 if any lint warnings are detected (`!report.clean`).
        #[arg(long)]
        strict: bool,

        /// Optional file path to write the rendered lint report.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate a standalone offline interactive HTML diagram viewer (or serve locally).
    View {
        /// Optional `.sysml` files to lift and view (uses `--model` when empty).
        files: Vec<PathBuf>,

        /// Built-in model name when no positional files are passed (`apollo11-model` or `embedded-flight-controller`).
        #[arg(long, default_value = "apollo11-model")]
        model: String,

        /// Output `.html` path (defaults to `<stem>.html` alongside the first file or `<model>.html`).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Initial diagram view.
        #[arg(long, value_enum, default_value = "general")]
        view: CliViewerView,

        /// Default color theme (`dark` or `light`).
        #[arg(long, value_enum, default_value = "dark")]
        theme: CliViewerTheme,

        /// Custom HTML page title (defaults to filename or model name).
        #[arg(long)]
        title: Option<String>,

        /// Print the `ViewerSnapshot` JSON to stdout instead of writing HTML.
        #[arg(long)]
        json_snapshot: bool,

        /// Serve the generated HTML on `http://127.0.0.1:<port>` instead of exiting after writing.
        #[arg(long)]
        serve: bool,

        /// Optional simulation trace JSON (`rsysdsl verify --trace <path>`) to embed for interactive playback.
        #[arg(long)]
        trace: Option<PathBuf>,

        /// TCP port for `--serve`.
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },

    /// Scaffold a new compiling model crate with layered Types -> Requirements -> Subsystems -> Context packages.
    New {
        /// System name in any case (`my_system`, `my-system`, `MySystem`); package name becomes kebab-case.
        name: String,

        /// Target directory (default: `<workspace>/examples/<snake_name>`). Must not exist.
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Do not add the crate to the root `Cargo.toml` `members`; generate a standalone crate instead.
        #[arg(long)]
        no_workspace: bool,
    },

    /// Reconcile a SysML v2 model against a cppognee/docmem Knowledge Graph export (`kg_export.json`).
    Reconcile {
        /// Path to `kg_export.json` produced by `tools/corpus_kg/`.
        #[arg(long)]
        kg: PathBuf,

        /// `.sysml` files to lift and reconcile (uses `--model` when empty).
        #[arg(long)]
        files: Vec<PathBuf>,

        /// Built-in model name when `--files` is empty.
        #[arg(long, default_value = "apollo11-model")]
        model: String,

        /// Optional corpus directory containing `manifest.json` for stale-document detection.
        #[arg(long)]
        corpus_dir: Option<PathBuf>,

        /// Output format (`json` or `markdown`).
        #[arg(long, value_enum, default_value = "json")]
        format: LintFormat,

        /// Exit with non-zero code on any error OR warning (alias drift, missing entity, unresolved conflict).
        #[arg(long)]
        strict: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum TraceFormat {
    Json,
    Markdown,
    Csv,
}

#[derive(Clone, Copy, ValueEnum)]
enum SafetyFormat {
    Json,
    Markdown,
}

#[derive(Clone, Copy, ValueEnum)]
enum CliPhase {
    Concept,
    Preliminary,
    Detailed,
    Release,
}

impl From<CliPhase> for sysml_ir::TbdPhase {
    fn from(p: CliPhase) -> Self {
        match p {
            CliPhase::Concept => sysml_ir::TbdPhase::Concept,
            CliPhase::Preliminary => sysml_ir::TbdPhase::Preliminary,
            CliPhase::Detailed => sysml_ir::TbdPhase::Detailed,
            CliPhase::Release => sysml_ir::TbdPhase::Release,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum LintFormat {
    Json,
    Markdown,
}

#[derive(Clone, Copy, ValueEnum)]
enum CliViewerView {
    General,
    Interconnection,
    State,
    Activity,
    Sequence,
    Usecase,
    Tree,
    Package,
    Graph,
    Hierarchy,
}

impl From<CliViewerView> for ViewerView {
    fn from(v: CliViewerView) -> Self {
        match v {
            CliViewerView::General => ViewerView::General,
            CliViewerView::Interconnection => ViewerView::Interconnection,
            CliViewerView::State => ViewerView::State,
            CliViewerView::Activity => ViewerView::Activity,
            CliViewerView::Sequence => ViewerView::Sequence,
            CliViewerView::Usecase => ViewerView::UseCase,
            CliViewerView::Tree => ViewerView::Tree,
            CliViewerView::Package => ViewerView::Package,
            CliViewerView::Graph => ViewerView::Graph,
            CliViewerView::Hierarchy => ViewerView::Hierarchy,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum CliViewerTheme {
    Dark,
    Light,
}

impl From<CliViewerTheme> for ViewerTheme {
    fn from(t: CliViewerTheme) -> Self {
        match t {
            CliViewerTheme::Dark => ViewerTheme::Dark,
            CliViewerTheme::Light => ViewerTheme::Light,
        }
    }
}

fn print_json<T: Serialize>(val: &T, pretty: bool) {
    let text = if pretty {
        serde_json::to_string_pretty(val)
    } else {
        serde_json::to_string(val)
    };
    match text {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("error: failed to serialize JSON output: {e}"),
    }
}

fn read_units(files: &[PathBuf]) -> Result<Vec<(String, String)>, String> {
    let mut units = Vec::with_capacity(files.len());
    for path in files {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("failed to read '{}': {e}", path.display()))?;
        units.push((path.display().to_string(), content));
    }
    Ok(units)
}

/// Name of the synthetic unit holding the sysml-std library when it is auto-loaded.
const SYSML_STD_UNIT: &str = "<sysml-std>";

/// Like [`read_units`], but when the files reference `SysmlStd*` packages without defining
/// them (emitted with `emit --omit-std`), append the bundled sysml-std library as an extra
/// unit so validation, lint and trace resolve it.
fn read_units_with_std(files: &[PathBuf]) -> Result<Vec<(String, String)>, String> {
    let mut units = read_units(files)?;
    let references_std = units.iter().any(|(_, t)| t.contains("SysmlStd"));
    let defines_std = units
        .iter()
        .any(|(_, t)| t.contains("library package SysmlStd") || t.contains("package SysmlStd"));
    if references_std && !defines_std {
        let std_model = sysml_ir::Model::new(sysml_std::std_packages());
        units.push((SYSML_STD_UNIT.to_string(), sysml_emit::emit_model(&std_model)));
    }
    Ok(units)
}

fn resolve_lib_dir(stdlib: bool, lib: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
    if let Some(dir) = lib {
        if !dir.is_dir() {
            return Err(format!(
                "standard library path '{}' does not exist or is not a directory",
                dir.display()
            ));
        }
        return Ok(Some(dir));
    }
    if stdlib {
        return discover_omg_stdlib().map(Some).ok_or_else(|| {
            "OMG standard library (`sysml.library`) could not be auto-discovered; pass `--lib <PATH>` or set `SYSML_LIBRARY_DIR`".to_string()
        });
    }
    Ok(None)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Emit { model, output, raw } => cmd_emit(&model, output, raw, cli.pretty),
        Commands::Lift {
            files,
            output,
            emit_main,
        } => cmd_lift(&files, output, emit_main, cli.pretty),
        Commands::RoundtripCheck {
            files,
            reemitted_out,
        } => cmd_roundtrip_check(&files, reemitted_out, cli.pretty),
        Commands::Verify {
            scenario,
            list,
            trace,
        } => cmd_verify(&scenario, list, trace, cli.pretty),
        Commands::Audit {
            model,
            files,
            verify,
            stdlib,
            lib,
            strict,
            maturity,
        } => cmd_audit(&model, &files, verify, stdlib, lib, strict, maturity.into(), cli.pretty),
        Commands::Tbd {
            model,
            files,
            format,
            output,
            baseline,
            check,
            write_baseline,
        } => {
            if let Some(path) = baseline.filter(|_| check || write_baseline) {
                cmd_tbd_baseline(&model, &files, &path, write_baseline, cli.pretty)
            } else {
                cmd_tbd(&model, &files, format, output, cli.pretty)
            }
        }
        Commands::TraceMatrix {
            model,
            files,
            format,
            verify,
            output,
        } => cmd_trace_matrix(&model, &files, format, verify, output, cli.pretty),
        Commands::Validate {
            files,
            stdlib,
            lib,
            strict,
        } => cmd_validate(&files, stdlib, lib, strict, cli.pretty),
        Commands::SafetyReport {
            model,
            files,
            format,
            output,
        } => cmd_safety_report(&model, &files, format, output, cli.pretty),
        Commands::Lint {
            model,
            files,
            format,
            strict,
            output,
        } => cmd_lint(&model, &files, format, strict, output, cli.pretty),
        Commands::View {
            files,
            model,
            output,
            view,
            theme,
            title,
            json_snapshot,
            serve,
            trace,
            port,
        } => cmd_view(
            &files,
            &model,
            output,
            view.into(),
            theme.into(),
            title,
            json_snapshot,
            serve,
            trace,
            port,
            cli.pretty,
        ),
        Commands::New {
            name,
            dir,
            no_workspace,
        } => cmd_new(&name, dir, no_workspace, cli.pretty),
        Commands::Reconcile {
            kg,
            files,
            model,
            corpus_dir,
            format,
            strict,
        } => cmd_reconcile(
            &kg,
            &files,
            &model,
            corpus_dir.as_deref(),
            format,
            strict,
            cli.pretty,
        ),
    }
}

fn cmd_reconcile(
    kg_path: &Path,
    files: &[PathBuf],
    model_name: &str,
    corpus_dir: Option<&Path>,
    format: LintFormat,
    strict: bool,
    pretty: bool,
) -> ExitCode {
    let kg_raw = match fs::read_to_string(kg_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read KG export '{}': {e}", kg_path.display());
            return ExitCode::FAILURE;
        }
    };
    let kg: KgExport = match serde_json::from_str(&kg_raw) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("error: invalid KG JSON '{}': {e}", kg_path.display());
            return ExitCode::FAILURE;
        }
    };
    let model = if !files.is_empty() {
        let units = match read_units_with_std(files) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        };
        lift_sysml_units(&units).model
    } else {
        match resolve_builtin_model(model_name) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        }
    };

    let report = reconcile_model(&model, &kg, corpus_dir);
    match format {
        LintFormat::Markdown => print!("{}", render_reconcile_markdown(&report)),
        LintFormat::Json => print_json(&report, pretty),
    }
    let has_warn = report
        .findings
        .iter()
        .any(|f| f.severity == ReconcileSeverity::Warning);
    if !report.ok || (strict && has_warn) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_view(
    files: &[PathBuf],
    model_name: &str,
    output: Option<PathBuf>,
    view: ViewerView,
    theme: ViewerTheme,
    title_override: Option<String>,
    json_snapshot: bool,
    serve: bool,
    trace_path: Option<PathBuf>,
    port: u16,
    pretty: bool,
) -> ExitCode {
    let (default_title, default_out, model) = if !files.is_empty() {
        let units = match read_units_with_std(files) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let lifted = lift_sysml_units(&units);
        if !lifted.parse_errors.is_empty() {
            for err in &lifted.parse_errors {
                eprintln!("parse error: {err}");
            }
            return ExitCode::from(2);
        }
        let first = &files[0];
        let stem = first
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("sysml_model");
        let default_title = if files.len() == 1 {
            first
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(stem)
                .to_string()
        } else {
            format!("{stem} (+{} files)", files.len() - 1)
        };
        let default_out = match first.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.join(format!("{stem}.html")),
            _ => PathBuf::from(format!("{stem}.html")),
        };
        (default_title, default_out, lifted.model)
    } else {
        let model = match resolve_builtin_model(model_name) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let default_out = PathBuf::from(format!("{model_name}.html"));
        (model_name.to_string(), default_out, model)
    };

    let title = title_override.unwrap_or(default_title);
    let snapshot = build_viewer_snapshot(&model, view);

    if json_snapshot {
        print_json(&snapshot, pretty);
        return ExitCode::SUCCESS;
    }

    let trace_json = match trace_path {
        Some(p) => match fs::read_to_string(&p) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("error: failed to read trace '{}': {e}", p.display());
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    let html = render_standalone_html(
        &model,
        &ViewerOptions {
            title: title.clone(),
            initial_view: view,
            theme,
            trace_json,
        },
    );

    if serve {
        if let Some(out) = &output {
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            if let Err(e) = fs::write(out, &html) {
                eprintln!("error: failed to write '{}': {e}", out.display());
                return ExitCode::from(2);
            }
        }
        let addr = format!("127.0.0.1:{port}");
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: failed to bind HTTP server on {addr}: {e}");
                return ExitCode::from(2);
            }
        };
        let actual_addr = listener.local_addr().map_or(addr, |a| a.to_string());
        eprintln!("Serving SysML v2 viewer at http://{actual_addr}/ (Ctrl+C to stop)");
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("warning: accept error: {e}");
                    continue;
                }
            };
            let mut reader = BufReader::new(&stream);
            let mut request_line = String::new();
            let _ = reader.read_line(&mut request_line);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                if line.trim().is_empty() {
                    break;
                }
                line.clear();
            }

            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                html.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(html.as_bytes());
            let _ = stream.flush();
        }
        return ExitCode::SUCCESS;
    }

    let out = output.unwrap_or(default_out);
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Err(e) = fs::write(&out, &html) {
        eprintln!("error: failed to write '{}': {e}", out.display());
        return ExitCode::from(2);
    }

    #[derive(Serialize)]
    struct ViewOutput {
        ok: bool,
        output: String,
        bytes: usize,
        title: String,
        view: &'static str,
        theme: &'static str,
        packages: usize,
        relationships: usize,
    }
    print_json(
        &ViewOutput {
            ok: true,
            output: out.display().to_string(),
            bytes: html.len(),
            title,
            view: view.runtime_id(),
            theme: theme.as_str(),
            packages: snapshot.elements.len(),
            relationships: snapshot.relationships.len(),
        },
        pretty,
    );
    ExitCode::SUCCESS
}

fn resolve_builtin_model(model_name: &str) -> Result<sysml_ir::Model, String> {
    match model_name {
        "apollo11-model" => Ok(apollo11_model::build_apollo11_model()),
        "embedded-flight-controller" => Ok(sysml_ir::Model::new(vec![
            iso26262_metadata_package(),
            embedded_flight_controller::flight_controller::sysml_package(),
        ])),
        "microwave" => Ok(microwave::build_model()),
        "dishwasher" => Ok(dishwasher::build_model()),
        "power-tools-family" => Ok(power_tools_family::build_model()),
        other => Err(format!(
            "unknown model '{other}' (available: apollo11-model, embedded-flight-controller, microwave, dishwasher, power-tools-family)"
        )),
    }
}

/// Executable verification evidence for a built-in model (`--verify`).
fn builtin_verification(model_name: &str) -> Result<sysml_ir::VerificationReport, String> {
    match model_name {
        "apollo11-model" => Ok(apollo11_sim::run_nominal_verification()),
        "microwave" => Ok(microwave_sim::verification_report()),
        "dishwasher" => Ok(dishwasher_sim::verification_report()),
        "power-tools-family" => Ok(power_tools_family_sim::verification_report()),
        other => Err(format!(
            "`--verify` has no simulation runner for model '{other}' (available: apollo11-model, microwave, dishwasher, power-tools-family)"
        )),
    }
}

fn cmd_safety_report(
    model_name: &str,
    files: &[PathBuf],
    format: SafetyFormat,
    output: Option<PathBuf>,
    pretty: bool,
) -> ExitCode {
    let (label, model) = if !files.is_empty() {
        let units = match read_units_with_std(files) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let lifted = lift_sysml_units(&units);
        if !lifted.parse_errors.is_empty() {
            for err in &lifted.parse_errors {
                eprintln!("parse error: {err}");
            }
            return ExitCode::from(2);
        }
        let label = if files.len() == 1 {
            files[0].display().to_string()
        } else {
            format!("{} files", files.len())
        };
        (label, lifted.model)
    } else {
        let model = match resolve_builtin_model(model_name) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        (model_name.to_string(), model)
    };

    let report = build_safety_report(label, &model);
    let rendered = match format {
        SafetyFormat::Json => {
            let text = if pretty {
                serde_json::to_string_pretty(&report)
            } else {
                serde_json::to_string(&report)
            };
            match text {
                Ok(s) => format!("{s}\n"),
                Err(e) => {
                    eprintln!("error: failed to serialize safety report: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        SafetyFormat::Markdown => render_safety_report_markdown(&report),
    };

    if let Some(path) = &output {
        if let Err(e) = fs::write(path, &rendered) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
    }
    print!("{rendered}");
    ExitCode::SUCCESS
}

/// Load a model from `--files` (lifted, sysml-std appended when referenced) or a built-in name.
fn load_model(model_name: &str, files: &[PathBuf]) -> Result<(String, sysml_ir::Model), ExitCode> {
    if files.is_empty() {
        return resolve_builtin_model(model_name)
            .map(|m| (model_name.to_string(), m))
            .map_err(|e| {
                eprintln!("error: {e}");
                ExitCode::from(2)
            });
    }
    let units = read_units_with_std(files).map_err(|e| {
        eprintln!("error: {e}");
        ExitCode::from(2)
    })?;
    let lifted = lift_sysml_units(&units);
    if !lifted.parse_errors.is_empty() {
        for err in &lifted.parse_errors {
            eprintln!("parse error: {err}");
        }
        return Err(ExitCode::from(2));
    }
    let label = if files.len() == 1 {
        files[0].display().to_string()
    } else {
        format!("{} files", files.len())
    };
    Ok((label, lifted.model))
}

fn cmd_tbd(
    model_name: &str,
    files: &[PathBuf],
    format: LintFormat,
    output: Option<PathBuf>,
    pretty: bool,
) -> ExitCode {
    let (label, model) = match load_model(model_name, files) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ledger = sysml_emit::tbd_ledger_report(label, &model);
    let rendered = match format {
        LintFormat::Json => {
            let text = if pretty {
                serde_json::to_string_pretty(&ledger)
            } else {
                serde_json::to_string(&ledger)
            };
            match text {
                Ok(s) => format!("{s}\n"),
                Err(e) => {
                    eprintln!("error: failed to serialize TBD ledger: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        LintFormat::Markdown => sysml_emit::render_tbd_markdown(&ledger),
    };
    if let Some(path) = &output {
        if let Err(e) = fs::write(path, &rendered) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
    }
    print!("{rendered}");
    ExitCode::SUCCESS
}

/// `rsysdsl tbd --baseline <file> --check | --write-baseline`.
fn cmd_tbd_baseline(
    model_name: &str,
    files: &[PathBuf],
    baseline_path: &std::path::Path,
    write: bool,
    pretty: bool,
) -> ExitCode {
    let (label, model) = match load_model(model_name, files) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let current = sysml_emit::tbd_ledger_report(label, &model);
    if write {
        let text = match serde_json::to_string_pretty(&current) {
            Ok(t) => format!("{t}\n"),
            Err(e) => {
                eprintln!("error: failed to serialize TBD baseline: {e}");
                return ExitCode::from(2);
            }
        };
        if let Err(e) = fs::write(baseline_path, text) {
            eprintln!("error: failed to write '{}': {e}", baseline_path.display());
            return ExitCode::from(2);
        }
        eprintln!(
            "wrote TBD baseline {} ({} TBD(s))",
            baseline_path.display(),
            current.tbd_count
        );
        print_json(&current, pretty);
        return ExitCode::SUCCESS;
    }
    let baseline: sysml_emit::TbdLedger = match fs::read_to_string(baseline_path)
        .map_err(|e| e.to_string())
        .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()))
    {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "error: cannot read TBD baseline '{}': {e} (create it with `rsysdsl tbd --files ... --baseline {} --write-baseline`)",
                baseline_path.display(),
                baseline_path.display()
            );
            return ExitCode::from(2);
        }
    };
    let ratchet = sysml_emit::check_tbd_ratchet(&baseline, &current);
    print_json(&ratchet, pretty);
    if ratchet.ok {
        if !ratchet.resolved_entries.is_empty() {
            eprintln!(
                "note: {} baseline TBD(s) resolved; lower the baseline with --write-baseline to lock that in",
                ratchet.resolved_entries.len()
            );
        }
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "error: TBD count rose above the baseline ({}); new TBDs:",
            ratchet.rose.join(", ")
        );
        for e in &ratchet.new_entries {
            eprintln!(
                "  + {} [{}, resolve by {}]: {}",
                e.element,
                e.kind.as_str(),
                e.resolve_by.as_str(),
                e.reason
            );
        }
        eprintln!(
            "help: resolve them, or raise the baseline deliberately in the same commit: rsysdsl tbd --files ... --baseline {} --write-baseline",
            baseline_path.display()
        );
        ExitCode::from(1)
    }
}

fn cmd_lint(
    model_name: &str,
    files: &[PathBuf],
    format: LintFormat,
    strict: bool,
    output: Option<PathBuf>,
    pretty: bool,
) -> ExitCode {
    let (label, model) = if !files.is_empty() {
        let units = match read_units_with_std(files) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let lifted = lift_sysml_units(&units);
        if !lifted.parse_errors.is_empty() {
            for err in &lifted.parse_errors {
                eprintln!("parse error: {err}");
            }
            return ExitCode::from(2);
        }
        let label = if files.len() == 1 {
            files[0].display().to_string()
        } else {
            format!("{} files", files.len())
        };
        (label, lifted.model)
    } else {
        let model = match resolve_builtin_model(model_name) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        (model_name.to_string(), model)
    };

    let report = lint_model(label, &model);
    let rendered = match format {
        LintFormat::Json => {
            let text = if pretty {
                serde_json::to_string_pretty(&report)
            } else {
                serde_json::to_string(&report)
            };
            match text {
                Ok(s) => format!("{s}\n"),
                Err(e) => {
                    eprintln!("error: failed to serialize lint report: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        LintFormat::Markdown => render_lint_markdown(&report),
    };

    if let Some(path) = &output {
        if let Err(e) = fs::write(path, &rendered) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
    }
    print!("{rendered}");
    if strict && !report.clean {
        eprintln!(
            "error: lint failed in strict mode with {} warning(s)",
            report.summary.warning_count
        );
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_emit(model_name: &str, output: Option<PathBuf>, raw: bool, pretty: bool) -> ExitCode {
    let model = match resolve_builtin_model(model_name) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let sysml = emit_model(&model);
    let validation = validate_sysml_units(&[(format!("{model_name}.sysml"), sysml.clone())]);

    if let Some(path) = &output {
        if let Err(e) = fs::write(path, &sysml) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
    }

    if raw {
        print!("{sysml}");
    } else {
        #[derive(Serialize)]
        struct EmitOutput {
            model: String,
            packages: usize,
            bytes: usize,
            output_path: Option<String>,
            validation_ok: bool,
            validation_errors: usize,
            sysml: String,
        }
        print_json(
            &EmitOutput {
                model: model_name.to_string(),
                packages: model.packages.len(),
                bytes: sysml.len(),
                output_path: output.map(|p| p.display().to_string()),
                validation_ok: validation.valid,
                validation_errors: validation.error_count,
                sysml,
            },
            pretty,
        );
    }

    if validation.valid {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "error: emitted SysML v2 failed validation with {} error(s)",
            validation.error_count
        );
        ExitCode::from(1)
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_audit(
    model_name: &str,
    files: &[PathBuf],
    with_verify: bool,
    stdlib: bool,
    lib: Option<PathBuf>,
    strict: bool,
    maturity: sysml_ir::TbdPhase,
    pretty: bool,
) -> ExitCode {
    let lib_dir = match resolve_lib_dir(stdlib, lib) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let mut report = if !files.is_empty() {
        if with_verify {
            eprintln!("error: `--verify` is only supported for executable built-in models, not `--files`");
            return ExitCode::from(2);
        }
        let units = match read_units_with_std(files) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let lifted = lift_sysml_units(&units);
        audit_units_with_lib(&units, &lifted.model, None, lib_dir.as_deref(), strict)
    } else {
        let model = match resolve_builtin_model(model_name) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let verification = if with_verify {
            match builtin_verification(model_name) {
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            }
        } else {
            None
        };
        audit_model_with_lib(
            model_name,
            &model,
            verification.as_ref(),
            lib_dir.as_deref(),
            strict,
        )
    };
    sysml_emit::apply_maturity(&mut report, maturity, strict);
    print_json(&report, pretty);
    if report.ok {
        ExitCode::SUCCESS
    } else {
        if !report.overdue_tbds.is_empty() && strict {
            eprintln!(
                "error: {} TBD(s) due by maturity `{}` are still open (see `overdue_tbds`)",
                report.overdue_tbds.len(),
                report.maturity.as_str()
            );
        }
        eprintln!(
            "error: audit failed with {} finding(s)",
            report.findings.len()
        );
        ExitCode::from(1)
    }
}

fn cmd_trace_matrix(
    model_name: &str,
    files: &[PathBuf],
    format: TraceFormat,
    with_verify: bool,
    output: Option<PathBuf>,
    pretty: bool,
) -> ExitCode {
    let (label, model, verification) = if !files.is_empty() {
        if with_verify {
            eprintln!("error: `--verify` is only supported for executable built-in models, not `--files`");
            return ExitCode::from(2);
        }
        let units = match read_units_with_std(files) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let lifted = lift_sysml_units(&units);
        if !lifted.parse_errors.is_empty() {
            for err in &lifted.parse_errors {
                eprintln!("parse error: {err}");
            }
            return ExitCode::from(2);
        }
        let label = if files.len() == 1 {
            files[0].display().to_string()
        } else {
            format!("{} files", files.len())
        };
        (label, lifted.model, None)
    } else {
        let model = match resolve_builtin_model(model_name) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };
        let verification = if with_verify {
            match builtin_verification(model_name) {
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            }
        } else {
            None
        };
        (model_name.to_string(), model, verification)
    };

    let matrix = build_trace_matrix(label, &model, verification.as_ref());
    let rendered = match format {
        TraceFormat::Json => {
            let text = if pretty {
                serde_json::to_string_pretty(&matrix)
            } else {
                serde_json::to_string(&matrix)
            };
            match text {
                Ok(s) => format!("{s}\n"),
                Err(e) => {
                    eprintln!("error: failed to serialize trace matrix: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        TraceFormat::Markdown => render_trace_matrix_markdown(&matrix),
        TraceFormat::Csv => render_trace_matrix_csv(&matrix),
    };

    if let Some(path) = &output {
        if let Err(e) = fs::write(path, &rendered) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
    }
    print!("{rendered}");
    ExitCode::SUCCESS
}

fn cmd_lift(
    files: &[PathBuf],
    output: Option<PathBuf>,
    emit_main: bool,
    pretty: bool,
) -> ExitCode {
    let units = match read_units(files) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let report = lift_sysml_units(&units);
    if !report.parse_errors.is_empty() {
        for err in &report.parse_errors {
            eprintln!("parse error: {err}");
        }
        return ExitCode::from(2);
    }

    let rust_src = generate_rust_dsl(
        &report.model,
        &CodegenOptions {
            emit_model_fn: true,
            emit_main_emitter: emit_main,
        },
    );

    if let Some(path) = &output {
        if let Err(e) = fs::write(path, &rust_src) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
    }

    if !report.unsupported.is_empty() {
        eprintln!(
            "info: lifted {} package(s) with {} explicitly logged unsupported construct(s)",
            report.stats.packages,
            report.unsupported.len()
        );
    }

    #[derive(Serialize)]
    struct LiftOutput {
        ok: bool,
        output_path: Option<String>,
        stats: sysml_lift::LiftStats,
        unsupported: Vec<sysml_lift::UnsupportedConstruct>,
        rust_bytes: usize,
        rust_source: String,
    }
    print_json(
        &LiftOutput {
            ok: true,
            output_path: output.map(|p| p.display().to_string()),
            stats: report.stats,
            unsupported: report.unsupported,
            rust_bytes: rust_src.len(),
            rust_source: rust_src,
        },
        pretty,
    );
    ExitCode::SUCCESS
}

fn cmd_roundtrip_check(
    files: &[PathBuf],
    reemitted_out: Option<PathBuf>,
    pretty: bool,
) -> ExitCode {
    let units = match read_units(files) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let lifted = lift_sysml_units(&units);
    if !lifted.parse_errors.is_empty() {
        for err in &lifted.parse_errors {
            eprintln!("parse error: {err}");
        }
        return ExitCode::from(2);
    }

    let reemitted = emit_model(&lifted.model);
    if let Some(path) = &reemitted_out {
        if let Err(e) = fs::write(path, &reemitted) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
    }

    let validation = validate_sysml_units(&[("reemitted.sysml".to_string(), reemitted.clone())]);
    let diff = compare_sysml_roundtrip(&units, &[("reemitted.sysml".to_string(), reemitted)]);

    let ok = validation.valid && diff.fidelity_ok;
    #[derive(Serialize)]
    struct RoundtripOutput {
        ok: bool,
        validation_ok: bool,
        validation_errors: usize,
        roundtrip: sysml_lift::RoundTripDiff,
        unsupported_logged: usize,
    }
    print_json(
        &RoundtripOutput {
            ok,
            validation_ok: validation.valid,
            validation_errors: validation.error_count,
            roundtrip: diff,
            unsupported_logged: lifted.unsupported.len(),
        },
        pretty,
    );

    if ok {
        ExitCode::SUCCESS
    } else {
        eprintln!("error: roundtrip-check detected validation or semantic fidelity mismatch");
        ExitCode::from(1)
    }
}

/// Every scenario `rsysdsl verify` can run. Runner crates register here.
fn verify_registry() -> sysml_sim::Registry {
    let mut registry = sysml_sim::Registry::new();
    registry.extend(apollo11_sim::scenarios());
    registry.extend(microwave_sim::scenarios());
    registry.extend(dishwasher_sim::scenarios());
    registry.extend(power_tools_family_sim::scenarios());
    registry
}

/// `--scenario all` output: `{"ok": .., "<key>": report, ...}` in registration order.
struct AllVerifyOutput {
    ok: bool,
    reports: Vec<(&'static str, sysml_ir::VerificationReport)>,
}

impl Serialize for AllVerifyOutput {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.reports.len() + 1))?;
        map.serialize_entry("ok", &self.ok)?;
        for (key, report) in &self.reports {
            map.serialize_entry(key, report)?;
        }
        map.end()
    }
}

fn cmd_verify(scenario: &str, list: bool, trace: Option<PathBuf>, pretty: bool) -> ExitCode {
    let registry = verify_registry();
    if list {
        print_json(&registry.iter().collect::<Vec<_>>(), pretty);
        return ExitCode::SUCCESS;
    }

    let selected: Vec<&sysml_sim::RegisteredScenario> = if scenario == "all" {
        registry.iter().collect()
    } else {
        match registry.find(scenario) {
            Some(s) => vec![s],
            None => {
                let names: Vec<_> = registry.iter().map(|s| s.name).collect();
                eprintln!(
                    "error: unknown scenario '{scenario}' (available: all, {}; see `rsysdsl verify --list`)",
                    names.join(", ")
                );
                return ExitCode::from(2);
            }
        }
    };

    let outcomes: Vec<_> = selected.iter().map(|s| (s, (s.run)())).collect();

    if let Some(path) = trace {
        let runs: Vec<&sysml_sim::RunResult> = outcomes.iter().map(|(_, o)| &o.run).collect();
        let text = if runs.len() == 1 {
            serde_json::to_string(runs[0])
        } else {
            serde_json::to_string(&runs)
        }
        .expect("run results serialize");
        if let Err(e) = fs::write(&path, text) {
            eprintln!("error: cannot write trace to {}: {e}", path.display());
            return ExitCode::from(2);
        }
    }

    if scenario == "all" {
        let ok = outcomes.iter().all(|(s, o)| s.meets_expectation(o));
        let reports = outcomes
            .into_iter()
            .map(|(s, o)| (s.key, o.report))
            .collect();
        print_json(&AllVerifyOutput { ok, reports }, pretty);
        return if ok {
            ExitCode::SUCCESS
        } else {
            eprintln!("error: verification suite expectation mismatch");
            ExitCode::from(1)
        };
    }

    let (entry, outcome) = &outcomes[0];
    let report = &outcome.report;
    print_json(report, pretty);
    if report.all_passed {
        ExitCode::SUCCESS
    } else if !entry.expect_pass {
        eprintln!(
            "info: fault-injection scenario triggered {} expected requirement failure(s)",
            report.failed
        );
        ExitCode::from(1)
    } else {
        eprintln!("error: {} requirement(s) failed verification", report.failed);
        ExitCode::from(1)
    }
}

fn cmd_validate(
    files: &[PathBuf],
    stdlib: bool,
    lib: Option<PathBuf>,
    strict: bool,
    pretty: bool,
) -> ExitCode {
    let units = match read_units_with_std(files) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let lib_dir = match resolve_lib_dir(stdlib, lib) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let report = validate_sysml_units_with_lib(&units, lib_dir.as_deref());
    print_json(&report, pretty);
    let ok = report.valid && (!strict || report.warning_count == 0);
    if ok {
        ExitCode::SUCCESS
    } else if !report.valid {
        eprintln!(
            "error: validation failed with {} error(s)",
            report.error_count
        );
        ExitCode::from(1)
    } else {
        eprintln!(
            "error: strict validation failed with {} warning(s)",
            report.warning_count
        );
        ExitCode::from(1)
    }
}

fn cmd_new(name: &str, dir: Option<PathBuf>, no_workspace: bool, pretty: bool) -> ExitCode {
    match scaffold::scaffold(name, dir, no_workspace) {
        Ok(report) => {
            eprintln!("created {} at {}", report.package, report.dir);
            print_json(&report, pretty);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
