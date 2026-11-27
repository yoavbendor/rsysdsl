use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct EnergyPayload;

#[sysml::port(abstract)]
#[derive(Default)]
pub struct AbstractEnergyPort {
    #[sysml::out]
    pub energy: EnergyPayload,
}

#[sysml::port]
#[sysml::specializes(AbstractEnergyPort)]
#[derive(Default)]
pub struct DcBusPort {
    #[sysml::out]
    pub energy: EnergyPayload,
}

#[sysml::interface(abstract)]
pub struct EnergyChannel {
    #[sysml::end]
    pub source: DcBusPort,
    #[sysml::end]
    pub sink: Conjugated<DcBusPort>,
    #[sysml::attr(default = 28.0, unit = "V")]
    pub nominal_voltage: Quantity<f64>,
}

#[sysml::interface]
#[sysml::specializes(EnergyChannel)]
pub struct RegulatedDcBus {
    #[sysml::attr(redefines = "nominalVoltage", default = 12.0, unit = "V")]
    pub nominal_voltage: Quantity<f64>,
}

#[sysml::part(abstract)]
pub struct BatteryBase {
    #[sysml::attr(default = 100.0, unit = "W")]
    pub max_power: Quantity<f64>,
    #[sysml::port]
    pub power_out: DcBusPort,
}

#[sysml::part]
#[sysml::specializes(BatteryBase)]
pub struct CompactBattery {
    #[sysml::attr(redefines = "maxPower", default = 150.0, unit = "W")]
    pub max_power: Quantity<f64>,
    #[sysml::port]
    pub power_out: DcBusPort,
}

#[sysml::part]
#[sysml::specializes(BatteryBase)]
pub struct ExtendedBattery {
    #[sysml::attr(redefines = "maxPower", default = 300.0, unit = "W")]
    pub max_power: Quantity<f64>,
    #[sysml::port]
    pub power_out: DcBusPort,
}

#[sysml::variation(of = BatteryBase)]
pub enum BatteryPackOption {
    Compact(CompactBattery),
    Extended(ExtendedBattery),
}

#[sysml::part]
pub struct LogicalPowerManager {
    #[sysml::attr(default = 5.0, unit = "W")]
    pub budget: Quantity<f64>,
}

#[sysml::part]
pub struct PhysicalMcuBoard {
    #[sysml::port]
    pub power_in: Conjugated<DcBusPort>,
}

#[sysml::part]
#[sysml::connect(interface = RegulatedDcBus, name = "dcFeed", from = battery.power_out, to = mcu.power_in)]
#[sysml::allocate(name = "powerMgmtToMcu", from = logical_mgr, to = mcu)]
pub struct PowerSystem {
    #[sysml::part]
    pub battery: CompactBattery,
    #[sysml::part]
    pub logical_mgr: LogicalPowerManager,
    #[sysml::part]
    pub mcu: PhysicalMcuBoard,
}

fn main() {
    let sys = PowerSystem::default();
    assert_eq!(sys.battery.max_power.value, 150.0);
    let bus = RegulatedDcBus::default();
    assert_eq!(bus.nominal_voltage.value, 12.0);
}
