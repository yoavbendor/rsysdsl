//! Agent-first CLI for `rsysdsl` (inspired by `nomograph-ai/sysml`).
//!
//! Conventions:
//! - Compact JSON on `stdout` by default (`--pretty` for indented JSON).
//! - Human/tool diagnostics strictly on `stderr`.
//! - Exit codes: `0` = ok, `1` = validation/audit/roundtrip/verification failure, `2` = IO/parse error.

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use sysml_emit::{
    audit_model_with_lib, audit_units_with_lib, build_trace_matrix, discover_omg_stdlib,
    emit_model, render_trace_matrix_csv, render_trace_matrix_markdown, validate_sysml_units,
    validate_sysml_units_with_lib,
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
    }
}

fn resolve_builtin_model(model_name: &str) -> Result<sysml_ir::Model, String> {
    match model_name {
        "apollo11-model" => Ok(apollo11_model::build_apollo11_model()),
        other => Err(format!(
            "unknown model '{other}' (available: apollo11-model)"
        )),
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
