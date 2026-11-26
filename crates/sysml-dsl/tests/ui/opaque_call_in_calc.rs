use sysml_dsl::prelude::*;

// An opaque unmodeled Rust function living outside #[sysml::package]
fn secret_unmodeled_helper(x: f64) -> f64 {
    if x > 10.0 { 0.0 } else { x }
}

#[sysml::package(name = "CalcLeakPackage")]
pub mod calc_leak_package {
    use super::secret_unmodeled_helper;
    use sysml_dsl::prelude::*;

    // BUG: Calling an opaque non-#[sysml::calc] Rust function from inside #[sysml::calc]!
    #[sysml::calc]
    pub fn leaky_calc(x: f64) -> f64 {
        secret_unmodeled_helper(x) * 2.0
    }
}

fn main() {}
