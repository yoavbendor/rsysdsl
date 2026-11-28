# Haiku trial-3 fixtures

- Authored by Haiku agents in trial 3 from short written specs (see `docs/trials/T3_PROTOCOL.md`): greenhouse, ebike, water heater and heat-recovery ventilation, 3 runs each.
- Emitted with `rsysdsl emit --omit-std`; the files are unmodified copies of the trial outputs.
- Used as regression inputs for the `budget`, `refute` and `trade-study` operators (`crates/sysml-cli/tests/operators_regression.rs`).
- Some models contain deliberate gaps and unrecognised `[W]` attributes (e.g. `ratedPowerW`); they are inputs, not reference designs.
