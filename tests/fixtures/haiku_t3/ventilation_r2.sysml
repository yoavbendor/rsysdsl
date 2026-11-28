package VentilationR2Types {
    doc /* Layer 1: payloads, ports and interfaces. */
    private import ScalarValues::*;
    private import SI::*;

    item def FilterPressureSignal {
        doc /* Differential pressure across the air filter, from the filter's pressure sensor. */
        attribute dpPa : Real;
    }

    port def FilterPressurePort {
        doc /* Directed output port of the differential pressure sensor. */
        out item signal : FilterPressureSignal;
    }

    item def BmsExchange {
        doc /*
            Status and setpoint exchange with the BMS. Content and direction are not in the spec,
            so the port is symmetric and the item carries no fields.
        */
    }

    port def BmsDataPort {
        doc /* Bidirectional BMS data port (status and setpoints). Protocol is TBD. */
        inout item data : BmsExchange;
    }

    item def BoostRequest {
        doc /*
            Boost request event. The spec names a Boost mode but not its trigger; this placeholder
            event exists only so the open transitions can name a trigger.
        */
    }

    item def FrostEvent {
        doc /* Frost condition event. The spec says the FrostProtection entry condition is to be defined. */
    }
}

package VentilationR2Requirements {
    doc /* Layer 2: requirements with document provenance (HRV-REQ draft 0.4). */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'HRV-1'> SupplyAirflowRequirement {
        doc /* HRV-1: Supply airflow shall be at least 0.14 m^3/s. */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 2 (HRV-1)";
        }
        attribute minSupplyAirflowM3s : Real default 0.14 [m^3/s];
        attribute measuredSupplyAirflowM3s : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { measuredSupplyAirflowM3s >= minSupplyAirflowM3s }
    }

    requirement def <'HRV-2'> FilterAlarmRequirement {
        doc /*
            HRV-2: A filter alarm shall be raised when the filter pressure drop exceeds 150 Pa.
            Modeled as: the configured alarm trigger is at or below 150 Pa, so the alarm is raised
            no later than the drop exceeds 150 Pa.
        */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 3 (HRV-2)";
        }
        attribute maxAlarmDpPa : Real default 150.0 [Pa];
        attribute alarmTriggerDpPa : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { alarmTriggerDpPa <= maxAlarmDpPa }
    }

    requirement def <'HRV-3'> TotalPowerRequirement {
        doc /* HRV-3: Total electrical power shall not exceed TBD W (pending energy-label class). */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 4 (HRV-3)";
        }
        attribute maxTotalPowerW : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "HRV-3 limit pending energy-label class (source: TBD W)";
                resolveBy = "detailed";
            }
        }
        attribute measuredTotalPowerW : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { measuredTotalPowerW <= maxTotalPowerW }
    }

    requirement def <'HRV-4'> HeatExchangerEfficiencyRequirement {
        doc /* HRV-4: Heat exchanger efficiency shall be at least the datasheet value. */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 5 (HRV-4)";
        }
        attribute minEfficiency : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "vendor datasheet not received; value not in source";
                resolveBy = "detailed";
            }
        }
        attribute measuredEfficiency : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { measuredEfficiency >= minEfficiency }
    }
}

package VentilationR2Subsystems {
    doc /* Layer 3: subsystem parts, actions, operating modes and calculations. */
    private import VentilationR2Types::*;
    private import ScalarValues::*;
    private import SI::*;
    private import SysmlStdMechanics::*;
    private import SysmlStdPower::*;

    action def ControlAirflow {
        doc /* Air-handling function: command supply and exhaust fans. */
    }

    action def MonitorFilterPressure {
        doc /* Air-handling function: monitor filter differential pressure. */
    }

    action def ExchangeBmsData {
        doc /* Communication function: exchange status and setpoints with the BMS. */
    }

    part def EcFanMotor :> BrushlessDcMotor {
        doc /*
            EC fan motor. Rated 170 W at 2400 rpm; 2400 rpm = 40 rev/s exactly (2400 / 60).
            Motor efficiency is not in the spec, so the sysml-std default is blanked as a TBD.
        */
        attribute :>> ratedPower default 170.0 [W];
        attribute :>> ratedRotationFreq default 40.0 [Hz];
        attribute :>> efficiency {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EC motor efficiency not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> mass {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EC motor mass not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> heatDissipation {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EC motor heat dissipation not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> noiseLevelDb {
            @TbdMetadata::TBD {
                category = "value";
                reason = "fan noise level not given in spec";
                resolveBy = "detailed";
            }
        }
    }

    part def SupplyFan {
        doc /* Supply fan: delivers 0.14 m^3/s, driven by an EC motor. */
        attribute designAirflow : Real default 0.14 [m^3/s];
        part motor : EcFanMotor;
        satisfy requirement : VentilationR2Requirements::SupplyAirflowRequirement;
    }

    part def ExhaustFan {
        doc /* Exhaust fan: exhaust flow equals the supply flow (spec). */
        attribute designAirflow : Real default 0.14 [m^3/s];
        part motor : EcFanMotor;
    }

    part def PlateHeatExchanger {
        doc /* Plate heat exchanger. Efficiency is in the vendor datasheet, which is not received. */
        attribute efficiency : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "vendor datasheet not received";
                resolveBy = "detailed";
            }
        }
        satisfy requirement : VentilationR2Requirements::HeatExchangerEfficiencyRequirement;
    }

    part def DifferentialPressureSensor {
        doc /* Differential pressure sensor across the air filter. */
        port dpOut : FilterPressurePort;
    }

    part def AirFilter {
        doc /* Air filter with its differential pressure sensor. */
        part dpSensor : DifferentialPressureSensor;
    }

    part def MainsSupply :> AcMainsSupply {
        doc /*
            Mains supply: 230 V (spec). Max current and frequency are not in the spec, so the
            AcMainsSupply library defaults are blanked as TBDs.
        */
        attribute :>> nominalVoltage default 230.0 [V];
        attribute :>> maxCurrent {
            @TbdMetadata::TBD {
                category = "value";
                reason = "mains max current not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> mainsFrequency {
            @TbdMetadata::TBD {
                category = "value";
                reason = "mains frequency not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> mass {
            @TbdMetadata::TBD {
                category = "value";
                reason = "mains supply mass not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> ratedPower {
            @TbdMetadata::TBD {
                category = "value";
                reason = "mains supply rated power not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> efficiency {
            @TbdMetadata::TBD {
                category = "value";
                reason = "mains supply efficiency not given in spec";
                resolveBy = "detailed";
            }
        }
        attribute :>> wasteHeat {
            @TbdMetadata::TBD {
                category = "value";
                reason = "mains supply waste heat not given in spec";
                resolveBy = "detailed";
            }
        }
    }

    part def Controller {
        doc /*
            Controller: owns the operating modes, the 150 Pa filter alarm threshold (HRV-2),
            and the BMS data port.
        */
        attribute alarmThreshold : Real default 150.0 [Pa];
        port bmsPort : BmsDataPort;
        perform action control : ControlAirflow;
        perform action monitor : MonitorFilterPressure;
        satisfy requirement : VentilationR2Requirements::FilterAlarmRequirement;
    }

    part def BmsGateway {
        doc /* Gateway to the building management system. */
        port bmsPort : BmsDataPort;
        perform action exchange : ExchangeBmsData;
    }

    state def VentilationMode {
        doc /*
            Operating modes. Normal, Boost, FilterAlarm and FrostProtection (spec).
            Boost and FrostProtection entry/exit and the FilterAlarm clear condition are not in the
            spec: those transitions are TBDs.
        */
        ref part target : Controller;
        entry;
        then normal;
        state normal;
        state boost;
        state filterAlarm;
        state frostProtection;
        transition first normal accept payload : FilterPressureSignal if payload.dpPa > target.alarmThreshold then filterAlarm;
        transition first normal accept BoostRequest then boost {
            @TbdMetadata::TBD {
                category = "element";
                reason = "boost entry condition not given in spec";
                resolveBy = "detailed";
            }
        }
        transition first normal accept FrostEvent then frostProtection {
            @TbdMetadata::TBD {
                category = "element";
                reason = "FrostProtection entry condition to be defined (spec)";
                resolveBy = "detailed";
            }
        }
        transition first boost accept BoostRequest then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "boost exit condition not given in spec";
                resolveBy = "detailed";
            }
        }
        transition first boost accept payload : FilterPressureSignal if payload.dpPa > target.alarmThreshold then filterAlarm;
        transition first filterAlarm accept payload : FilterPressureSignal if payload.dpPa <= target.alarmThreshold then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "filter alarm clear condition not given in spec";
                resolveBy = "detailed";
            }
        }
        transition first frostProtection accept FrostEvent then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "FrostProtection exit condition not given in spec";
                resolveBy = "detailed";
            }
        }
    }

    calc def totalRatedFanPowerW {
        doc /* Total rated electrical power of the two fans (W). */
        in supplyW : Real;
        in exhaustW : Real;
        return result : Real = supplyW + exhaustW;
    }

    calc def filterMarginPa {
        doc /* Margin between the filter alarm threshold and the measured pressure drop (Pa). */
        in limitPa : Real;
        in dpPa : Real;
        return result : Real = limitPa - dpPa;
    }
}

package VentilationR2Context {
    doc /* Layer 4: the single top-level context composing the subsystems. */
    private import VentilationR2Types::*;
    private import VentilationR2Subsystems::*;

    part def HeatRecoveryVentilationUnit {
        doc /*
            Heat-recovery ventilation unit: all subsystems and the controller-to-BMS link.
            HRV-3 (total electrical power) is a system-level requirement and is satisfied here.
        */
        part mains : MainsSupply;
        part supplyFan : SupplyFan;
        part exhaustFan : ExhaustFan;
        part heatExchanger : PlateHeatExchanger;
        part airFilter : AirFilter;
        part controller : Controller;
        part bmsGateway : BmsGateway;
        connection controllerToBms connect controller.bmsPort to bmsGateway.bmsPort {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "BMS vendor decides protocol; status and setpoints exchanged";
                resolveBy = "detailed";
            }
        }
        satisfy requirement : VentilationR2Requirements::TotalPowerRequirement;
    }
}

library package TbdMetadata {
    doc /* To-be-determined markers: every rsysdsl escape hatch carries a reason, an optional owner and the design phase (concept, preliminary, detailed, release) by which it must be resolved. */
    private import ScalarValues::*;

    metadata def TBD {
        doc /* category: unit | value | element | channel | custom_unit; resolveBy: concept | preliminary | detailed | release. */
        attribute category : String;
        attribute reason : String;
        attribute owner : String;
        attribute resolveBy : String;
    }
}
