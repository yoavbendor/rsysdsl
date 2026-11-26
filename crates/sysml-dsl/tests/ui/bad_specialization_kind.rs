use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct Power;

#[sysml::port]
#[derive(Default)]
pub struct UmbilicalPort {
    #[sysml::out]
    pub power: Power,
}

// BUG: A `part def` cannot specialize (`:>`) a `port def`!
#[sysml::part]
#[sysml::specializes(UmbilicalPort)]
#[derive(Default)]
pub struct BrokenPart;

fn main() {}
