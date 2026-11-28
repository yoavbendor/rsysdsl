use sysml_dsl::prelude::*;

// BUG: latency is a time; a frequency (`Hz`) is its reciprocal, not a latency.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    #[sysml::attr(budget = "latency", default = 60.0, unit = "Hz")]
    pub frame_latency: Quantity<f64>,
}

fn main() {}
