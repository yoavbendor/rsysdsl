//! Ergonomic Rust DSL facade for authoring strongly typed SysML v2 models.
//!
//! ```rust,ignore
//! use sysml_dsl::prelude::*;
//!
//! #[sysml::package(name = "Demo")]
//! pub mod demo {
//!     use sysml_dsl::prelude::*;
//!
//!     #[sysml::item]
//!     pub struct Power;
//!
//!     #[sysml::port]
//!     pub struct PowerPort {
//!         #[sysml::out]
//!         pub power: Power,
//!     }
//! }
//! ```

pub use sysml_ir as ir;
pub use sysml_ir::*;
pub use sysml_macros as sysml;

pub mod prelude {
    pub use sysml_ir::{
        Conjugated, EvaluableRequirement, Model, Package, Performs, PortConnectable, Quantity,
        Satisfies, Specializes, SysmlAction, SysmlCalc, SysmlEnum, SysmlInterface, SysmlItem,
        SysmlPart, SysmlPort, SysmlRequirement, SysmlState, ToScalar, TransitionError,
        VerificationReport, VerificationStatus, VerificationVerdict, Verifies,
    };
    pub use sysml_macros as sysml;
}
