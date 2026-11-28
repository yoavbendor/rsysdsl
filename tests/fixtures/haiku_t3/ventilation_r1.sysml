package VentilationR1Types {
    doc /* Layer 1: payloads, ports, interfaces and mode events. */
    private import ScalarValues::*;
    private import SI::*;

    item def FilterDpSample {
        doc /* Pressure drop across the air filter, as sampled by the differential pressure sensor. */
        attribute pressureDrop : Real default 0.0 [Pa];
    }

    port def FilterDpPort {
        doc /* Directed output of the differential pressure sensor. */
        out item sample : FilterDpSample;
    }

    interface def FilterSignalLink {
        doc /*
            Typed link: filter DP sensor -> controller. The sensor-to-controller signal path is
            implied by the filter alarm requirement (HRV-2); its medium/protocol is not stated.
        */
        end source : FilterDpPort;
        end sink : ~FilterDpPort;
    }

    item def BmsFrame {
        doc /*
            Exchange frame toward the building management system (status up, setpoints down).
            The source names the content (status and setpoints) but not its encoding.
        */
    }

    port def BmsPort {
        doc /* Bidirectional BMS port (status and setpoints travel both ways). */
        inout item 'frame' : BmsFrame;
    }

    item def BoostRequest {
        doc /* Mode trigger: request to enter Boost. The source lists Boost but not its entry condition. */
    }

    item def FrostEntry {
        doc /* Mode trigger: frost condition. The source says the entry condition is to be defined. */
    }

    item def ModeReset {
        doc /* Mode trigger: return to Normal. The source gives no exit conditions for the non-Normal modes. */
    }
}

package VentilationR1Requirements {
    doc /* Layer 2: requirements with document provenance (HRV-REQ draft 0.4). */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'HRV-1'> SupplyAirflow {
        doc /* HRV-1: Supply airflow shall be at least 0.14 m^3/s. */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 2";
        }
        attribute minAirflowM3s : Real default 0.14 [m^3/s] {
            doc /* LIMIT from HRV-1. */
        }
        attribute measuredAirflowM3s : Real {
            doc /* [evidence] EVIDENCE: measured supply airflow; no default until a verification case supplies it. */
        }
        require constraint { measuredAirflowM3s >= minAirflowM3s }
    }

    requirement def <'HRV-2'> FilterAlarmLimit {
        doc /*
            HRV-2: A filter alarm shall be raised when the filter pressure drop exceeds 150 Pa.
            Modeled as: the alarm trigger level shall not exceed 150 Pa (a trigger at or below 150 Pa
            raises the alarm once the drop exceeds 150 Pa).
        */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 3";
        }
        attribute maxAlarmTriggerPa : Real default 150.0 [Pa] {
            doc /* LIMIT from HRV-2: 150 Pa. */
        }
        attribute alarmTriggerPa : Real {
            doc /* [evidence] EVIDENCE: alarm trigger level as designed/verified; no default. */
        }
        require constraint { alarmTriggerPa <= maxAlarmTriggerPa }
    }

    requirement def <'HRV-3'> TotalElectricalPower {
        doc /* HRV-3: Total electrical power shall not exceed TBD W (pending energy-label class). */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 4";
        }
        attribute maxPowerW : Real {
            doc /* LIMIT: the source leaves the value open (depends on energy-label class). */
            @TbdMetadata::TBD {
                category = "value";
                reason = "HRV-3 limit pending energy-label class; source gives no value";
                resolveBy = "detailed";
            }
        }
        attribute measuredPowerW : Real {
            doc /* [evidence] EVIDENCE: measured total electrical power; no default. */
        }
        require constraint { measuredPowerW <= maxPowerW }
    }

    requirement def <'HRV-4'> HeatExchangerEfficiency {
        doc /* HRV-4: Heat exchanger efficiency shall be at least the datasheet value. */
        @Rationale {
            text = "Source: HRV-REQ draft 0.4, section 5";
        }
        attribute minEfficiency : Real {
            doc /* LIMIT: the vendor datasheet value, which has not been received. */
            @TbdMetadata::TBD {
                category = "value";
                reason = "vendor datasheet not received; HRV-4 limit is its value";
                owner = "heat exchanger vendor";
                resolveBy = "detailed";
            }
        }
        attribute measuredEfficiency : Real {
            doc /* [evidence] EVIDENCE: measured efficiency; no default. */
        }
        require constraint { measuredEfficiency >= minEfficiency }
    }
}

package VentilationR1Subsystems {
    doc /* Layer 3: reusable part definitions (boxes), operating modes and calculations. */
    private import VentilationR1Types::*;
    private import ScalarValues::*;
    private import SI::*;

    action def SenseFilterPressure {
        doc /* Function: measure the filter pressure drop. */
    }

    action def ExchangeHeat {
        doc /* Function: transfer heat between the exhaust and supply air streams. */
    }

    action def RaiseFilterAlarm {
        doc /* Function: raise the filter alarm. */
    }

    action def ExchangeWithBms {
        doc /* Function: exchange status and setpoints with the BMS. */
    }

    part def EcMotor {
        doc /* Electronically commutated (EC) fan motor: 170 W rated at 2400 rpm. */
        attribute ratedPowerW : Real default 170.0 [W] {
            doc /* Rated power from the spec. */
        }
        attribute ratedSpeedHz : Real default 40.0 [Hz] {
            doc /* Rated speed from the spec: 2400 rpm = 2400 / 60 = 40 Hz (exact conversion). */
        }
    }

    abstract part def EcFan {
        doc /* Abstract fan: every fan in the unit is driven by an EC motor. */
        part motor : EcMotor;
    }

    part def SupplyFan :> EcFan {
        doc /* Supply fan: delivers the supply airflow (HRV-1). */
        attribute airflow : Real default 0.14 [m^3/s] {
            doc /* Supply airflow from the spec. */
        }
        satisfy requirement : VentilationR1Requirements::SupplyAirflow;
    }

    part def ExhaustFan :> EcFan {
        doc /* Exhaust fan: the spec states the exhaust flow equals the supply flow. */
        attribute airflow : Real default 0.14 [m^3/s] {
            doc /* Exhaust airflow: equal to the supply flow per the spec. */
        }
    }

    part def PlateHeatExchanger {
        doc /* Plate heat exchanger. Efficiency is verified against HRV-4 (datasheet value TBD). */
        perform action exchange : ExchangeHeat;
        satisfy requirement : VentilationR1Requirements::HeatExchangerEfficiency;
    }

    part def DpSensor {
        doc /* Differential pressure sensor across the filter. */
        port dpOut : FilterDpPort;
        perform action sense : SenseFilterPressure;
    }

    part def AirFilter {
        doc /* Air filter with its differential pressure sensor. */
        part dpSensor : DpSensor;
    }

    part def Controller {
        doc /* Controller: runs the operating modes, raises the filter alarm, talks to the BMS gateway. */
        attribute alarmTriggerPa : Real default 150.0 [Pa] {
            doc /* Filter alarm trigger level (design value; the HRV-2 spec limit is 150 Pa). */
        }
        port filterDp : ~FilterDpPort;
        port bms : BmsPort;
        perform action alarm : RaiseFilterAlarm;
        perform action bms : ExchangeWithBms;
        satisfy requirement : VentilationR1Requirements::FilterAlarmLimit;
    }

    part def BmsGateway {
        doc /* Gateway to the building management system. Its protocol is decided by the BMS vendor. */
        port link : BmsPort;
    }

    state def OperatingMode {
        doc /*
            Operating modes of the controller. Normal, Boost, FilterAlarm and FrostProtection are from
            the spec. Only the filter alarm entry condition is sourced (HRV-2); all other entry and
            exit conditions are TBD transitions.
        */
        ref part target : Controller;
        entry;
        then normal;
        state normal;
        state boost;
        state filterAlarm;
        state frostProtection;
        transition first normal accept payload : FilterDpSample if payload.pressureDrop > target.alarmTriggerPa then filterAlarm;
        transition first normal accept BoostRequest then boost {
            @TbdMetadata::TBD {
                category = "element";
                reason = "Boost entry condition not given in source";
                resolveBy = "detailed";
            }
        }
        transition first normal accept FrostEntry then frostProtection {
            @TbdMetadata::TBD {
                category = "element";
                reason = "FrostProtection entry condition to be defined (source)";
                resolveBy = "detailed";
            }
        }
        transition first boost accept ModeReset then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "Boost exit condition not given in source";
                resolveBy = "detailed";
            }
        }
        transition first filterAlarm accept ModeReset then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "filter alarm clear condition not given in source";
                resolveBy = "detailed";
            }
        }
        transition first frostProtection accept ModeReset then normal {
            @TbdMetadata::TBD {
                category = "element";
                reason = "FrostProtection exit condition not given in source";
                resolveBy = "detailed";
            }
        }
    }

    calc def fanRatedPowerW {
        doc /* Calculation: total rated fan power (sum of the two fan motor ratings). */
        in supplyW : Real;
        in exhaustW : Real;
        attribute total = supplyW + exhaustW;
        return result : Real = total;
    }
}

package VentilationR1Context {
    doc /* Layer 4: the single top-level context that composes the subsystems and wires the arrows. */
    private import VentilationR1Types::*;
    private import VentilationR1Subsystems::*;
    private import ScalarValues::*;
    private import SI::*;

    part def VentilationUnit {
        doc /* Top-level HRV unit. The mains supply (230 V) is a unit-level attribute. */
        attribute mainsVoltage : Real default 230.0 [V] {
            doc /* Mains supply voltage from the spec. */
        }
        part supplyFan : SupplyFan;
        part exhaustFan : ExhaustFan;
        part heatExchanger : PlateHeatExchanger;
        part airFilter : AirFilter;
        part controller : Controller;
        part gateway : BmsGateway;
        interface filterDpToController : FilterSignalLink connect airFilter.dpSensor.dpOut to controller.filterDp;
        connection controllerToBmsGateway connect controller.bms to gateway.link {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "BMS protocol to be decided by the BMS vendor";
                resolveBy = "detailed";
            }
        }
        satisfy requirement : VentilationR1Requirements::TotalElectricalPower;
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
