use sysml_dsl::prelude::*;

#[sysml::package(name = "UnmodeledTypePackage")]
pub mod unmodeled_type_package {
    use sysml_dsl::prelude::*;

    #[sysml::part]
    #[derive(Default)]
    pub struct Controller;

    // BUG: Unannotated Rust struct inside #[sysml::package] would be invisible in .sysml!
    pub struct ShadowSafetyState {
        pub tripped: bool,
    }
}

fn main() {}
