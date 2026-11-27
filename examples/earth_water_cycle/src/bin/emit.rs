//! Emit this model to `.sysml`: `emit [-o <out.sysml>] [--omit-std] [--std-out <std.sysml>]`.
//! `--omit-std` leaves the sysml-std library packages out of the file (`rsysdsl` loads them).

fn main() -> std::process::ExitCode {
    sysml_emit::emit_cli::main(&earth_water_cycle::build_model())
}
