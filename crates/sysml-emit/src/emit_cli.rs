//! Shared command-line driver for model crates' `src/bin/emit.rs`.
//!
//! ```text
//! emit [-o <model.sysml>] [--omit-std] [--std-out <sysml_std.sysml>]
//! ```
//! - `-o` / `--output`: write the emitted SysML to a file (default: stdout).
//! - `--omit-std`: leave the `sysml-std` library packages (`SysmlStd*`) out of the output so
//!   the file holds only the model's own packages. `rsysdsl audit|lint|trace-matrix|view
//!   --files ...` load the sysml-std library automatically when a file references it.
//! - `--std-out <path>`: also write the `SysmlStd*` packages the model uses to their own file
//!   (implies `--omit-std`), e.g. for tools other than `rsysdsl`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use sysml_ir::{Model, Package};

use crate::emit::emit_model;

/// `true` for packages of the `sysml-std` engineering library (`SysmlStdFlows`, ...).
pub fn is_sysml_std_package(pkg: &Package) -> bool {
    pkg.name.starts_with("SysmlStd")
}

/// Split a model into (model packages, sysml-std library packages) and emit each.
/// Requirement references are qualified against the full model before splitting.
pub fn emit_model_split_std(model: &Model) -> (String, String) {
    let full = crate::resolve::qualify_requirement_refs(model);
    let (std_pkgs, own): (Vec<Package>, Vec<Package>) =
        full.packages.into_iter().partition(is_sysml_std_package);
    (emit_model(&Model::new(own)), emit_model(&Model::new(std_pkgs)))
}

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create directory '{}': {e}", parent.display()))?;
        }
    }
    fs::write(path, text).map_err(|e| format!("failed to write '{}': {e}", path.display()))?;
    eprintln!("wrote {} ({} bytes)", path.display(), text.len());
    Ok(())
}

/// Parse `std::env::args()` and emit `model` (see the module docs for flags).
pub fn main(model: &Model) -> ExitCode {
    run(std::env::args().skip(1), model)
}

/// Testable core of [`main`].
pub fn run(args: impl IntoIterator<Item = String>, model: &Model) -> ExitCode {
    let mut out_path: Option<PathBuf> = None;
    let mut std_out: Option<PathBuf> = None;
    let mut omit_std = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" | "--std-out" => match args.next() {
                Some(p) if arg == "--std-out" => std_out = Some(PathBuf::from(p)),
                Some(p) => out_path = Some(PathBuf::from(p)),
                None => {
                    eprintln!("error: missing path after {arg}");
                    return ExitCode::from(2);
                }
            },
            "--omit-std" => omit_std = true,
            other => {
                eprintln!(
                    "usage: emit [-o <output.sysml>] [--omit-std] [--std-out <std.sysml>] (unexpected argument `{other}`)"
                );
                return ExitCode::from(2);
            }
        }
    }

    let (sysml, std_text) = if omit_std || std_out.is_some() {
        emit_model_split_std(model)
    } else {
        (emit_model(model), String::new())
    };

    if let Some(path) = &std_out {
        if let Err(e) = write_file(path, &std_text) {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    }
    if let Some(path) = out_path {
        if let Err(e) = write_file(&path, &sysml) {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    } else {
        print!("{sysml}");
    }
    ExitCode::SUCCESS
}
