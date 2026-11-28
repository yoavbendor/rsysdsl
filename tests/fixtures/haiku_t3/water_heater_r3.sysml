package WaterHeaterR3Types {
    doc /* Layer 1: payloads, ports and interfaces. */
    private import ScalarValues::*;
    private import SI::*;

    item def PowerFeed {
        doc /* Mains power feed (230 V supply) carried from mains to thermal cutoff to element. */
        attribute voltage : Real default 230.0 [V];
    }

    item def TempReading {
        doc /* Tank temperature sample from the NTC sensor. */
        attribute temperature : Real;
    }

    item def RelayCommand {
        doc /* Relay command from the controller to the heating element. */
        attribute elementOn : Boolean;
    }

    item def HeatDemand {
        doc /* Heat demand event for the operating-mode machine (true = controller requests heat). */
        attribute demand : Boolean;
    }

    item def CutoffTripped {
        doc /* Event: the mechanical thermal cutoff has opened. */
    }

    item def CutoffResetRequest {
        doc /* Event: manual reset request (method not given in the source; see the transition TBD). */
    }

    port def PowerPort {
        doc /* Power port: producer uses `PowerPort`, consumer uses `Conjugated<PowerPort>`. */
        out item feed : PowerFeed;
    }

    port def TempPort {
        doc /* Temperature output port (sensor side). */
        out item reading : TempReading;
    }

    port def RelayCmdPort {
        doc /* Relay command output port (controller side). */
        out item cmd : RelayCommand;
    }

    abstract interface def WaterHeaterChannel {
        doc /* Abstract classification base for every arrow type in this model. */
    }

    interface def MainsFeedLink :> WaterHeaterChannel {
        doc /* Mains supply to thermal cutoff. */
        end source : PowerPort;
        end sink : ~PowerPort;
    }

    interface def ElementFeedLink :> WaterHeaterChannel {
        doc /* Thermal cutoff to heating element (element power is disconnected through the cutoff). */
        end source : PowerPort;
        end sink : ~PowerPort;
    }

    interface def SensorLink :> WaterHeaterChannel {
        doc /* NTC sensor to controller. */
        end source : TempPort;
        end sink : ~TempPort;
    }

    interface def RelayLink :> WaterHeaterChannel {
        doc /* Controller to heating element relay command. */
        end source : RelayCmdPort;
        end sink : ~RelayCmdPort;
    }
}

package WaterHeaterR3Requirements {
    doc /* Layer 2: requirements with document provenance, and a modeled calculation. */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'WH-001'> WaterOutletTempLimit {
        doc /* Water outlet temperature shall not exceed 358 K. */
        @Rationale {
            text = "Source: WH-SPEC rev A, §2.1";
        }
        attribute maxOutletTempK : Real default 358.0 [K] {
            doc /* Spec LIMIT (WH-SPEC rev A §2.1). */
        }
        attribute measuredOutletTempK : Real {
            doc /* [evidence] EVIDENCE: measured outlet temperature, supplied by a verification case. */
        }
        require constraint { measuredOutletTempK <= maxOutletTempK }
    }

    requirement def <'WH-002'> CutoffTripLimit {
        doc /* The thermal cutoff shall open at or below 368 K. */
        @Rationale {
            text = "Source: WH-SPEC rev A, §2.2";
        }
        attribute cutoffTripLimitK : Real default 368.0 [K] {
            doc /* Spec LIMIT (WH-SPEC rev A §2.2): the cutoff opens at or below this temperature. */
        }
        attribute measuredTripTempK : Real {
            doc /* [evidence] EVIDENCE: measured cutoff opening temperature, supplied by a verification case. */
        }
        require constraint { measuredTripTempK <= cutoffTripLimitK }
    }

    requirement def <'WH-003'> ElementPowerLimit {
        doc /* Heating element power shall not exceed 2000 W. */
        @Rationale {
            text = "Source: WH-SPEC rev A, §3.1";
        }
        attribute maxElementPowerW : Real default 2000.0 [W] {
            doc /* Spec LIMIT (WH-SPEC rev A §3.1). */
        }
        attribute measuredElementPowerW : Real {
            doc /* [evidence] EVIDENCE: measured element power, supplied by a verification case. */
        }
        require constraint { measuredElementPowerW <= maxElementPowerW }
    }

    calc def cutoffMarginK {
        doc /* Margin between the cutoff trip limit and the measured trip temperature. */
        in limitK : Real;
        in measuredK : Real;
        return result : Real = limitK - measuredK;
    }
}

package WaterHeaterR3Subsystems {
    doc /* Layer 3: subsystem blueprints (boxes) and the operating-mode state machine. */
    private import WaterHeaterR3Types::*;
    private import SI::*;
    private import ScalarValues::*;

    part def MainsSupply {
        doc /* Mains supply (230 V). */
        attribute supplyVoltage : Real default 230.0 [V];
        port powerOut : PowerPort;
    }

    part def ThermalCutoff {
        doc /* Mechanical thermal cutoff: disconnects element power independently of the controller. */
        attribute tripTempK : Real default 368.0 [K];
        port powerIn : ~PowerPort;
        port powerOut : PowerPort;
        satisfy requirement : WaterHeaterR3Requirements::CutoffTripLimit;
    }

    part def HeatingElement {
        doc /* Resistive heating element. */
        attribute ratedPowerW : Real default 2000.0 [W];
        port powerIn : ~PowerPort;
        port cmdIn : ~RelayCmdPort;
        satisfy requirement : WaterHeaterR3Requirements::ElementPowerLimit;
    }

    part def WaterTank {
        doc /* Water tank: 80 L, stored in SI as 0.08 m^3. */
        attribute volumeM3 : Real default 0.08 [m^3];
        satisfy requirement : WaterHeaterR3Requirements::WaterOutletTempLimit;
    }

    part def NtcSensor {
        doc /* NTC temperature sensor sampling the tank at 1 Hz. */
        attribute sampleRate : Real default 1.0 [Hz];
        port tempOut : TempPort;
    }

    part def ThermostatController {
        doc /* Thermostat controller: reads tank temperature, switches the element via relay command. */
        attribute setpointTempK : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "thermostat setpoint not given in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        port tempIn : ~TempPort;
        port cmdOut : RelayCmdPort;
    }

    state def WaterHeaterMode {
        doc /* Operating modes of the heater. */
        entry;
        then idle;
        state idle;
        state heating;
        state overtemp;
        transition first idle accept payload : HeatDemand if payload.demand then heating;
        transition first idle accept CutoffTripped then overtemp;
        transition first heating accept payload : HeatDemand if not payload.demand then idle;
        transition first heating accept CutoffTripped then overtemp;
        transition first overtemp accept CutoffResetRequest then idle {
            @TbdMetadata::TBD {
                category = "element";
                reason = "cutoff reset method not given in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
    }
}

package WaterHeaterR3Context {
    doc /* Layer 4: the single top-level context wiring the subsystems. */
    private import WaterHeaterR3Types::*;
    private import WaterHeaterR3Subsystems::*;

    part def WaterHeaterR3System {
        doc /*
            Top-level water heater: mains feeds the element through the thermal cutoff; the
            controller switches the element via relay; the sensor feeds the controller.
        */
        part mains : MainsSupply;
        part cutoff : ThermalCutoff;
        part element : HeatingElement;
        part tank : WaterTank;
        part sensor : NtcSensor;
        part controller : ThermostatController;
        interface mainsToCutoff : MainsFeedLink connect mains.powerOut to cutoff.powerIn;
        interface cutoffToElement : ElementFeedLink connect cutoff.powerOut to element.powerIn;
        interface sensorToController : SensorLink connect sensor.tempOut to controller.tempIn;
        interface controllerToElement : RelayLink connect controller.cmdOut to element.cmdIn;
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
