//! Emit `starter-model` to `.sysml` (stdout or `-o <path>`) for VS Code SysML v2 / `sysml-reactflow`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use sysml_emit::emit_model;

fn main() -> ExitCode {
    let mut out_path: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => match args.next() {
                Some(p) => out_path = Some(PathBuf::from(p)),
                None => {
                    eprintln!("error: missing path after {arg}");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("usage: emit [-o <output.sysml>] (unexpected argument `{other}`)");
                return ExitCode::from(2);
            }
        }
    }

    let model = starter_model::build_model();
    let sysml = emit_model(&model);

    if let Some(path) = out_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = fs::create_dir_all(parent) {
                    eprintln!("error: failed to create directory '{}': {e}", parent.display());
                    return ExitCode::from(2);
                }
            }
        }
        if let Err(e) = fs::write(&path, &sysml) {
            eprintln!("error: failed to write '{}': {e}", path.display());
            return ExitCode::from(2);
        }
        eprintln!("wrote {} ({} bytes)", path.display(), sysml.len());
    } else {
        print!("{sysml}");
    }
    ExitCode::SUCCESS
}
