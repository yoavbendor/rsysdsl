use sysml_dsl::prelude::*;

#[sysml::package(name = "LeakyImplPackage")]
pub mod leaky_impl_package {
    use sysml_dsl::prelude::*;

    #[sysml::part]
    #[derive(Default)]
    pub struct Valve {
        pub pressure: f64,
    }

    // BUG: Raw `impl` block with hidden `if` logic inside #[sysml::package]!
    impl Valve {
        pub fn secret_interlock(&self) -> bool {
            if self.pressure > 100.0 {
                false
            } else {
                true
            }
        }
    }
}

fn main() {}
