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
use std::path::PathBuf;
use std::process::ExitCode;

use sysml_emit::{
    audit_model_with_lib, audit_units_with_lib, build_safety_report, build_trace_matrix,
    build_viewer_snapshot, discover_omg_stdlib, emit_model, iso26262_metadata_package,
    render_safety_report_markdown, render_standalone_html, render_trace_matrix_csv,
    render_trace_matrix_markdown, validate_sysml_units, validate_sysml_units_with_lib,
    ViewerOptions, ViewerTheme, ViewerView,
};
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

    /// Execute closed-loop verification on the Rust DSL Apollo 11 model and emit JSON verdicts.
    Verify {
        /// Scenario to execute.
        #[arg(long, value_enum, default_value = "nominal")]
        scenario: VerifyScenario,
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

        /// Fail with exit code 1 if any Open-MBEE validation warnings are emitted.
        #[arg(long)]
        strict: bool,
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

        /// TCP port for `--serve`.
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum VerifyScenario {
    Nominal,
    Fault,
    All,
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
        Commands::Verify { scenario } => cmd_verify(scenario, cli.pretty),
        Commands::Audit {
            model,
            files,
            verify,
            stdlib,
            lib,
            strict,
        } => cmd_audit(&model, &files, verify, stdlib, lib, strict, cli.pretty),
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
        Commands::View {
            files,
            model,
            output,
            view,
            theme,
            title,
            json_snapshot,
            serve,
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
            port,
            cli.pretty,
        ),
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
    port: u16,
    pretty: bool,
) -> ExitCode {
    let (default_title, default_out, model) = if !files.is_empty() {
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

    let html = render_standalone_html(
        &model,
        &ViewerOptions {
            title: title.clone(),
            initial_view: view,
            theme,
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
        other => Err(format!(
            "unknown model '{other}' (available: apollo11-model, embedded-flight-controller)"
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

fn cmd_audit(
    model_name: &str,
    files: &[PathBuf],
    with_verify: bool,
    stdlib: bool,
    lib: Option<PathBuf>,
    strict: bool,
    pretty: bool,
) -> ExitCode {
    let lib_dir = match resolve_lib_dir(stdlib, lib) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let report = if !files.is_empty() {
        if with_verify {
            eprintln!("error: `--verify` is only supported for executable built-in models, not `--files`");
            return ExitCode::from(2);
        }
        let units = match read_units(files) {
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
            Some(apollo11_sim::run_nominal_verification())
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
    print_json(&report, pretty);
    if report.ok {
        ExitCode::SUCCESS
    } else {
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
            Some(apollo11_sim::run_nominal_verification())
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

fn cmd_verify(scenario: VerifyScenario, pretty: bool) -> ExitCode {
    match scenario {
        VerifyScenario::Nominal => {
            let report = apollo11_sim::run_nominal_verification();
            print_json(&report, pretty);
            if report.all_passed {
                ExitCode::SUCCESS
            } else {
                eprintln!("error: {} requirement(s) failed verification", report.failed);
                ExitCode::from(1)
            }
        }
        VerifyScenario::Fault => {
            let report = apollo11_sim::run_fault_injection_verification();
            print_json(&report, pretty);
            if report.all_passed {
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "info: fault-injection scenario triggered {} expected requirement failure(s)",
                    report.failed
                );
                ExitCode::from(1)
            }
        }
        VerifyScenario::All => {
            let nominal = apollo11_sim::run_nominal_verification();
            let fault = apollo11_sim::run_fault_injection_verification();
            let ok = nominal.all_passed && !fault.all_passed;
            #[derive(Serialize)]
            struct AllVerifyOutput {
                ok: bool,
                nominal: sysml_ir::VerificationReport,
                fault_injection: sysml_ir::VerificationReport,
            }
            print_json(
                &AllVerifyOutput {
                    ok,
                    nominal,
                    fault_injection: fault,
                },
                pretty,
            );
            if ok {
                ExitCode::SUCCESS
            } else {
                eprintln!("error: verification suite expectation mismatch");
                ExitCode::from(1)
            }
        }
    }
}

fn cmd_validate(
    files: &[PathBuf],
    stdlib: bool,
    lib: Option<PathBuf>,
    strict: bool,
    pretty: bool,
) -> ExitCode {
    let units = match read_units(files) {
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
