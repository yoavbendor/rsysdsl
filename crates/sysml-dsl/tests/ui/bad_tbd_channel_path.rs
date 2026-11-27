use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Debug, Clone, Copy, Default)]
pub struct Frame {}

#[sysml::port]
#[derive(Debug, Clone, Copy, Default)]
pub struct DataPort {
    #[sysml::out_item]
    pub frame: Frame,
}

#[sysml::part]
#[derive(Debug, Clone, Copy, Default)]
pub struct Node {
    #[sysml::port]
    pub data: DataPort,
}

// BUG: a TBD channel skips port-compatibility checks, but field paths are still checked
// (`node_b.dat` does not exist).
#[sysml::part]
#[sysml::connect(tbd = "bus protocol not chosen", name = "link", from = node_a.data, to = node_b.dat)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Net {
    #[sysml::part]
    pub node_a: Node,
    #[sysml::part]
    pub node_b: Node,
}

fn main() {}
