package WaterHeaterR1Types {
    doc /* Layer 1: payloads, events, ports and interfaces (the "arrows"). */
    private import ScalarValues::*;
    private import SI::*;

    abstract interface def WaterHeaterChannel {
        doc /* Abstract classification base for every arrow in the water heater (no ends needed). */
    }

    item def MainsFeed {
        doc /* Mains feed payload carried from the 230 V supply to the heating element. */
        attribute voltageV : Real;
    }

    item def RelayCommand {
        doc /* Relay command from the controller to the heating element. */
        attribute elementOn : Boolean;
    }

    item def CutoffTrip {
        doc /* Trip command from the independent thermal cutoff: disconnects element power. */
        attribute openElement : Boolean;
    }

    item def TemperatureReading {
        doc /* NTC sensor reading of tank temperature. */
        attribute tempK : Real;
    }

    item def HeatFlow {
        doc /* Heat flow from the heating element into the tank water. */
        attribute powerW : Real;
    }

    item def HeatDemand {
        doc /* Event: thermostat calls for heat. */
    }

    item def HeatSatisfied {
        doc /* Event: thermostat is satisfied and stops calling for heat. */
    }

    item def CutoffTripped {
        doc /* Event: the independent thermal cutoff has opened. */
    }

    item def ResetRequest {
        doc /* Event: a reset request after an over-temperature trip. */
    }

    port def PowerFeedPort {
        doc /* Directed mains feed port (supply side). */
        out item feed : MainsFeed;
    }

    port def RelayCommandPort {
        doc /* Directed relay command port (controller side). */
        out item command : RelayCommand;
    }

    port def CutoffPort {
        doc /* Directed cutoff trip port (thermal cutoff side). */
        out item trip : CutoffTrip;
    }

    port def SensorPort {
        doc /* Directed sensor reading port (NTC side). */
        out item reading : TemperatureReading;
    }

    port def ThermalPort {
        doc /* Directed heat port (heating element side). */
        out item heat : HeatFlow;
    }

    interface def PowerFeedLink :> WaterHeaterChannel {
        doc /* Mains supply to heating element power link. */
        end source : PowerFeedPort;
        end sink : ~PowerFeedPort;
    }

    interface def RelayCommandLink :> WaterHeaterChannel {
        doc /* Controller to heating element relay command link. */
        end source : RelayCommandPort;
        end sink : ~RelayCommandPort;
    }

    interface def CutoffLink :> WaterHeaterChannel {
        doc /* Thermal cutoff to heating element trip link. */
        end source : CutoffPort;
        end sink : ~CutoffPort;
    }

    interface def SensorLink :> WaterHeaterChannel {
        doc /* NTC sensor to controller reading link. */
        end source : SensorPort;
        end sink : ~SensorPort;
    }

    interface def ThermalLink :> WaterHeaterChannel {
        doc /* Heating element to tank heat link. */
        end source : ThermalPort;
        end sink : ~ThermalPort;
    }
}

package WaterHeaterR1Requirements {
    doc /* Layer 2: requirements with document provenance (WH-SPEC rev A). */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'WH-001'> OutletTemperatureLimit {
        doc /* WH-001: Water outlet temperature shall not exceed 358 K. */
        @Rationale {
            text = "Source: WH-SPEC rev A, §2.1";
        }
        attribute maxOutletTempK : Real default 358.0 [K] {
            doc /* LIMIT from WH-001. */
        }
        attribute outletTempK : Real {
            doc /* [evidence] EVIDENCE: measured outlet temperature. No default. */
        }
        require constraint { outletTempK <= maxOutletTempK }
    }

    requirement def <'WH-002'> ThermalCutoffOpenLimit {
        doc /* WH-002: The thermal cutoff shall open at or below 368 K. */
        @Rationale {
            text = "Source: WH-SPEC rev A, §2.2";
        }
        attribute maxCutoffOpenTempK : Real default 368.0 [K] {
            doc /* LIMIT from WH-002 (upper bound on the opening temperature). */
        }
        attribute cutoffOpenTempK : Real {
            doc /* [evidence] EVIDENCE: measured cutoff opening temperature. No default. */
        }
        require constraint { cutoffOpenTempK <= maxCutoffOpenTempK }
    }

    requirement def <'WH-003'> ElementPowerLimit {
        doc /* WH-003: Heating element power shall not exceed 2000 W. */
        @Rationale {
            text = "Source: WH-SPEC rev A, §3.1";
        }
        attribute maxElementPowerW : Real default 2000.0 [W] {
            doc /* LIMIT from WH-003. */
        }
        attribute elementPowerW : Real {
            doc /* [evidence] EVIDENCE: measured element power. No default. */
        }
        require constraint { elementPowerW <= maxElementPowerW }
    }
}

package WaterHeaterR1Subsystems {
    doc /* Layer 3: subsystem blueprints (the "boxes"), actions, modes and calculations. */
    private import WaterHeaterR1Types::*;
    private import ScalarValues::*;
    private import SI::*;

    action def ReadTankTemperature {
        doc /* Function: sample the NTC sensor reading of tank temperature. */
    }

    action def SwitchElementRelay {
        doc /* Function: switch the heating element on/off by relay command. */
    }

    action def OpenElementPowerPath {
        doc /* Function: disconnect element power, independently of the controller. */
    }

    calc def elementPowerMarginW {
        doc /* Power margin of the heating element against its limit (WH-003 budget). */
        in limitW : Real;
        in actualW : Real;
        attribute margin = limitW - actualW;
        return result : Real = margin;
    }

    part def MainsSupply {
        doc /* 230 V mains supply. */
        attribute supplyVoltageV : Real default 230.0 [V];
        port powerOut : PowerFeedPort;
    }

    part def HeatingElement {
        doc /* 2000 W resistive heating element. */
        attribute ratedPowerW : Real default 2000.0 [W];
        port powerIn : ~PowerFeedPort;
        port relayIn : ~RelayCommandPort;
        port tripIn : ~CutoffPort;
        port heatOut : ThermalPort;
        satisfy requirement : WaterHeaterR1Requirements::ElementPowerLimit;
    }

    part def WaterTank {
        doc /* Tank holding 80 L of water. */
        attribute waterVolumeL : Real default 80.0 [L];
        port heatIn : ~ThermalPort;
    }

    part def NtcSensor {
        doc /* NTC temperature sensor sampling at 1 Hz. */
        attribute sampleRateHz : Real default 1.0 [Hz];
        port readingOut : SensorPort;
    }

    part def ThermostatController {
        doc /*
            Thermostat controller: reads tank temperature, switches the element via relay.
            Its state machine holds the operating modes Idle, Heating and Overtemp.
        */
        attribute setpointK : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "thermostat setpoint not given in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        port readingIn : ~SensorPort;
        port relayOut : RelayCommandPort;
        perform action readTank : ReadTankTemperature;
        perform action switchRelay : SwitchElementRelay;
        satisfy requirement : WaterHeaterR1Requirements::OutletTemperatureLimit;
    }

    state def WaterHeaterMode {
        doc /* Operating modes of the water heater (SPEC: Idle, Heating, Overtemp (cutoff opened)). */
        ref part target : ThermostatController;
        entry;
        then idle;
        state idle {
            doc /* Element off; waiting for heat demand. */
        }
        state heating {
            doc /* Element energised by relay command. */
        }
        state overtemp {
            doc /* Thermal cutoff opened; element power disconnected independently of the controller. */
        }
        transition first idle accept HeatDemand then heating {
            @TbdMetadata::TBD {
                category = "element";
                reason = "thermostat on-threshold relative to setpoint not given in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        transition first idle accept CutoffTripped then overtemp;
        transition first heating accept HeatSatisfied then idle {
            @TbdMetadata::TBD {
                category = "element";
                reason = "thermostat off-threshold relative to setpoint not given in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        transition first heating accept CutoffTripped then overtemp;
        transition first overtemp accept ResetRequest then idle {
            @TbdMetadata::TBD {
                category = "element";
                reason = "reset behaviour after a cutoff trip not given in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
    }

    part def ThermalCutoff {
        doc /* Independent mechanical thermal cutoff: disconnects element power. */
        attribute tripTempK : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "WH-SPEC rev A gives only the upper bound (at or below 368 K), not the rated trip point";
                resolveBy = "detailed";
            }
        }
        port tripOut : CutoffPort;
        perform action openPath : OpenElementPowerPath;
        satisfy requirement : WaterHeaterR1Requirements::ThermalCutoffOpenLimit;
    }
}

package WaterHeaterR1Context {
    doc /* Layer 4: the single top-level context composing the subsystems. */
    private import WaterHeaterR1Types::*;
    private import WaterHeaterR1Subsystems::*;

    part def WaterHeaterR1System {
        doc /* Top-level water heater context: all subsystems and their arrows. */
        part mains : MainsSupply;
        part element : HeatingElement;
        part tank : WaterTank;
        part sensor : NtcSensor;
        part controller : ThermostatController;
        part cutoff : ThermalCutoff;
        interface mainsToElement : PowerFeedLink connect mains.powerOut to element.powerIn;
        interface controllerToElementRelay : RelayCommandLink connect controller.relayOut to element.relayIn;
        interface cutoffToElement : CutoffLink connect cutoff.tripOut to element.tripIn;
        interface sensorToController : SensorLink connect sensor.readingOut to controller.readingIn;
        interface elementToTank : ThermalLink connect element.heatOut to tank.heatIn;
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
