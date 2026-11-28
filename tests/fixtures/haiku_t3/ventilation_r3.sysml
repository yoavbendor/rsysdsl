package VentilationR3Types {
    doc /* Layer 1: payloads, ports and interfaces. */
    private import ScalarValues::*;
    private import SI::*;

    item def BmsExchange {
        doc /*
            Status and setpoints exchanged with the BMS. Content and protocol are open: the BMS
            vendor decides the protocol.
        */
    }

    item def FilterDpReading {
        doc /* Filter differential pressure reading. */
        attribute dp : Real default 0.0 [Pa];
    }

    port def BmsPort {
        doc /* Bidirectional BMS-facing port (status and setpoints). */
        inout item exchange : BmsExchange;
    }

    port def DpPort {
        doc /* Directed differential pressure output port (connect to `Conjugated<DpPort>`). */
        out item reading : FilterDpReading;
    }

    abstract interface def HrvChannel {
        doc /* Abstract classification base for HRV arrows. */
    }

    interface def DpLink :> HrvChannel {
        doc /* Filter differential pressure signal, filter sensor to controller. */
        end source : DpPort;
        end sink : ~DpPort;
    }
}

package VentilationR3Requirements {
    doc /* Layer 2: requirements with document provenance (Source: HRV-REQ draft 0.4). */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'HRV-1'> SupplyAirflowMinimum {
        doc /* HRV-1: Supply airflow shall be at least 0.14 m^3/s. */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, §2";
        }
        attribute minSupplyAirflow : Real default 0.14 [m^3/s];
        attribute measuredSupplyAirflow : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { measuredSupplyAirflow >= minSupplyAirflow }
    }

    requirement def <'HRV-2'> FilterAlarmLimit {
        doc /*
            HRV-2: A filter alarm shall be raised when the filter pressure drop exceeds 150 Pa.
            Checked as: the measured pressure drop at alarm trip must not exceed the 150 Pa limit.
        */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, §3";
        }
        attribute alarmDpLimit : Real default 150.0 [Pa];
        attribute measuredAlarmDp : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { measuredAlarmDp <= alarmDpLimit }
    }

    requirement def <'HRV-3'> TotalPowerLimit {
        doc /* HRV-3: Total electrical power shall not exceed TBD W (pending energy-label class). */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, §4";
        }
        attribute maxTotalPower : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "limit pending energy-label class (HRV-3 gives TBD W)";
                resolveBy = "detailed";
            }
        }
        attribute measuredTotalPower : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { measuredTotalPower <= maxTotalPower }
    }

    requirement def <'HRV-4'> HeatExchangerEfficiencyMinimum {
        doc /* HRV-4: Heat exchanger efficiency shall be at least the vendor datasheet value. */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, §5";
        }
        attribute minEfficiency : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "vendor datasheet not yet received";
                resolveBy = "detailed";
            }
        }
        attribute measuredEfficiency : Real {
            doc /* [evidence] measured value supplied by a verification case; any model value is a placeholder, not evidence */
        }
        require constraint { measuredEfficiency >= minEfficiency }
    }
}

package VentilationR3Subsystems {
    doc /* Layer 3: subsystem blueprints and the controller operating-mode state machine. */
    private import VentilationR3Types::*;
    private import ScalarValues::*;
    private import SI::*;

    part def EcMotor {
        doc /* Electronically commutated (EC) motor, rated 170 W at 2400 rpm (source). */
        attribute ratedPower : Real default 170.0 [W];
        attribute ratedSpeed : Real default 40.0 [Hz] {
            doc /* Rated speed: 2400 rpm in the source, stated here as 40 Hz (2400 / 60, exact). */
        }
    }

    part def SupplyFan {
        doc /* Supply fan: EC motor driven, delivers 0.14 m^3/s. */
        attribute ratedAirflow : Real default 0.14 [m^3/s];
        part motor : EcMotor;
        satisfy requirement : VentilationR3Requirements::SupplyAirflowMinimum;
    }

    part def ExhaustFan {
        doc /* Exhaust fan: EC motor driven; exhaust flow equals supply flow (source). */
        attribute ratedAirflow : Real default 0.14 [m^3/s];
        part motor : EcMotor;
    }

    part def PlateHeatExchanger {
        doc /* Plate heat exchanger. Efficiency is in the vendor datasheet, not yet received. */
        attribute efficiency : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "vendor datasheet not yet received";
                resolveBy = "detailed";
            }
        }
        satisfy requirement : VentilationR3Requirements::HeatExchangerEfficiencyMinimum;
    }

    part def AirFilter {
        doc /* Air filter with differential pressure sensor output. */
        port dpOut : DpPort;
    }

    part def Controller {
        doc /* Controller: raises the filter alarm and exchanges status and setpoints with the BMS gateway. */
        attribute filterAlarmThreshold : Real default 150.0 [Pa];
        port dpIn : ~DpPort;
        port bms : BmsPort;
        satisfy requirement : VentilationR3Requirements::FilterAlarmLimit;
    }

    part def BmsGateway {
        doc /* Gateway to the building management system (BMS). */
        port bms : BmsPort;
    }

    state def HrvMode {
        doc /* Operating modes of the controller. */
        ref part target : Controller;
        entry;
        then normal;
        state normal;
        state boost {
            @TbdMetadata::TBD {
                category = "element";
                reason = "Boost entry and exit conditions are not given in the source";
                resolveBy = "detailed";
            }
        }
        state filterAlarm;
        state frostProtection {
            @TbdMetadata::TBD {
                category = "element";
                reason = "FrostProtection entry condition is to be defined (source)";
                resolveBy = "detailed";
            }
        }
        transition first normal accept payload : FilterDpReading if payload.dp > target.filterAlarmThreshold then filterAlarm;
        transition first normal accept BmsExchange then boost {
            @TbdMetadata::TBD {
                category = "element";
                reason = "Boost entry condition not given in the source";
                resolveBy = "detailed";
            }
        }
        transition first normal accept BmsExchange then frostProtection {
            @TbdMetadata::TBD {
                category = "element";
                reason = "FrostProtection entry condition is to be defined (source)";
                resolveBy = "detailed";
            }
        }
        transition first boost accept BmsExchange then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "Boost exit condition not given in the source";
                resolveBy = "detailed";
            }
        }
        transition first filterAlarm accept FilterDpReading then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "alarm clearance condition not given in the source";
                resolveBy = "detailed";
            }
        }
        transition first frostProtection accept BmsExchange then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "FrostProtection exit condition not given in the source";
                resolveBy = "detailed";
            }
        }
    }
}

package VentilationR3Context {
    doc /* Layer 4: the single top-level context that composes and wires the unit. */
    private import VentilationR3Types::*;
    private import VentilationR3Subsystems::*;
    private import ScalarValues::*;
    private import SI::*;

    part def HeatRecoveryVentilationUnit {
        doc /* Heat-recovery ventilation unit: top-level context part. */
        attribute mainsVoltage : Real default 230.0 [V];
        part supplyFan : SupplyFan;
        part exhaustFan : ExhaustFan;
        part heatExchanger : PlateHeatExchanger;
        part airFilter : AirFilter;
        part controller : Controller;
        part bmsGateway : BmsGateway;
        interface filterDpToController : DpLink connect airFilter.dpOut to controller.dpIn;
        connection controllerToBmsGateway connect controller.bms to bmsGateway.bms {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "BMS protocol to be decided by the BMS vendor";
                resolveBy = "detailed";
            }
        }
        satisfy requirement : VentilationR3Requirements::TotalPowerLimit;
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
