use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Debug, Clone, Copy, Default)]
pub struct Reading {}

#[sysml::port]
#[derive(Debug, Clone, Copy, Default)]
pub struct BasePort {
    #[sysml::out_item]
    pub reading: Reading,
}

// BUG: BasePort has no item `sample` to redefine.
#[sysml::port]
#[sysml::specializes(BasePort)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ChildPort {
    #[sysml::out_item(redefines)]
    pub sample: Reading,
}

fn main() {}
