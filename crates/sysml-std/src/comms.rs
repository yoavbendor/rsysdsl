//! Information & communication channel hierarchy: abstract `InfoChannel` specialized into
//! discrete GPIO, analog, UART, I2C, SPI, CAN-FD, Ethernet, and HMI channels.

use sysml_dsl::prelude::*;

/// sysml-std library package. Numeric attribute defaults here are generic LIBRARY DEFAULTS
/// (placeholders), not values from any source document: a model that specializes these
/// definitions must redefine every value its source specifies. `rsysdsl lint` lists the
/// defaults a model still inherits as `inherited_std_default`.
#[sysml::package(library, name = "SysmlStdComms")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
pub mod sysml_std_comms {
    use super::*;
    use crate::flows::sysml_std_flows::{
        CommandPort, DataBusPort, HmiFeedbackPort, HmiInputPort, InfoPort, TelemetryPort,
    };

    /// Abstract root interface for all information transfer channels.
    #[sysml::interface(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct InfoChannel {
        #[sysml::attr(default = 1000.0, unit = "Hz")]
        pub symbol_rate: Quantity<f64>,
        #[sysml::attr(default = 0.01, unit = "s")]
        pub max_latency: Quantity<f64>,
        #[sysml::end]
        pub tx: InfoPort,
        #[sysml::end]
        pub rx: Conjugated<InfoPort>,
    }

    /// Directed command channel carrying actuator commands.
    #[sysml::interface]
    #[sysml::specializes(InfoChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct CommandChannel {
        #[sysml::attr(redefines = "symbolRate", default = 10000.0, unit = "Hz")]
        pub symbol_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.005, unit = "s")]
        pub max_latency: Quantity<f64>,
        #[sysml::end]
        pub tx: CommandPort,
        #[sysml::end]
        pub rx: Conjugated<CommandPort>,
    }

    /// Directed telemetry channel carrying sensor readings or safety status.
    #[sysml::interface]
    #[sysml::specializes(InfoChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct TelemetryChannel {
        #[sysml::attr(redefines = "symbolRate", default = 10000.0, unit = "Hz")]
        pub symbol_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.005, unit = "s")]
        pub max_latency: Quantity<f64>,
        #[sysml::end]
        pub tx: TelemetryPort,
        #[sysml::end]
        pub rx: Conjugated<TelemetryPort>,
    }

    /// Abstract bidirectional digital bus (CAN-FD, I2C, SPI, UART, Ethernet).
    #[sysml::interface(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct DigitalBusChannel {
        #[sysml::attr(default = 100000.0, unit = "Hz")]
        pub bit_rate: Quantity<f64>,
        #[sysml::attr(default = 0.01, unit = "s")]
        pub max_latency: Quantity<f64>,
        #[sysml::end]
        pub endpoint_a: DataBusPort,
        #[sysml::end]
        pub endpoint_b: DataBusPort,
    }

    /// Asynchronous serial UART link (e.g. 115.2 kbaud debug / BMS / HMI).
    #[sysml::interface]
    #[sysml::specializes(DigitalBusChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct UartChannel {
        #[sysml::attr(redefines = "bitRate", default = 115200.0, unit = "Hz")]
        pub bit_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.01, unit = "s")]
        pub max_latency: Quantity<f64>,
    }

    /// Two-wire I2C board-level bus (400 kHz Fast Mode sensors / EEPROM / BMS).
    #[sysml::interface]
    #[sysml::specializes(DigitalBusChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct I2cChannel {
        #[sysml::attr(redefines = "bitRate", default = 400000.0, unit = "Hz")]
        pub bit_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.005, unit = "s")]
        pub max_latency: Quantity<f64>,
    }

    /// High-speed synchronous SPI bus (10 MHz ADC / display / motor driver).
    #[sysml::interface]
    #[sysml::specializes(DigitalBusChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct SpiChannel {
        #[sysml::attr(redefines = "bitRate", default = 10000000.0, unit = "Hz")]
        pub bit_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.0002, unit = "s")]
        pub max_latency: Quantity<f64>,
    }

    /// Deterministic multi-drop CAN-FD field bus (5 Mbps automotive/industrial).
    #[sysml::interface]
    #[sysml::specializes(DigitalBusChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct CanFdChannel {
        #[sysml::attr(redefines = "bitRate", default = 5000000.0, unit = "Hz")]
        pub bit_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.002, unit = "s")]
        pub max_latency: Quantity<f64>,
    }

    /// High-throughput switched Ethernet packet channel (1 Gbps).
    #[sysml::interface]
    #[sysml::specializes(DigitalBusChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct EthernetChannel {
        #[sysml::attr(redefines = "bitRate", default = 1000000000.0, unit = "Hz")]
        pub bit_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.0005, unit = "s")]
        pub max_latency: Quantity<f64>,
    }

    /// Human operator input interaction channel.
    #[sysml::interface]
    #[sysml::specializes(InfoChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct HmiInputChannel {
        #[sysml::attr(redefines = "symbolRate", default = 20.0, unit = "Hz")]
        pub symbol_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.05, unit = "s")]
        pub max_latency: Quantity<f64>,
        #[sysml::end]
        pub operator: HmiInputPort,
        #[sysml::end]
        pub panel: Conjugated<HmiInputPort>,
    }

    /// Visual / display feedback channel to a human operator.
    #[sysml::interface]
    #[sysml::specializes(InfoChannel)]
    #[derive(Debug, Clone, Copy)]
    pub struct HmiFeedbackChannel {
        #[sysml::attr(redefines = "symbolRate", default = 60.0, unit = "Hz")]
        pub symbol_rate: Quantity<f64>,
        #[sysml::attr(redefines = "maxLatency", default = 0.05, unit = "s")]
        pub max_latency: Quantity<f64>,
        #[sysml::end]
        pub panel: HmiFeedbackPort,
        #[sysml::end]
        pub operator: Conjugated<HmiFeedbackPort>,
    }
}
