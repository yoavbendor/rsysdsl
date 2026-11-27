use std::env;
use std::fs;
use std::path::PathBuf;
use sysml_emit::emit_model;

fn main() {
    let mut out: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "-o" || arg == "--output" {
            out = args.next().map(PathBuf::from);
        }
    }
    let model = bicycle::build_model();
    let text = emit_model(&model);
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
        fs::write(&path, &text).expect("failed to write emitted .sysml");
        eprintln!("wrote {} ({} bytes)", path.display(), text.len());
    } else {
        print!("{text}");
    }
}
