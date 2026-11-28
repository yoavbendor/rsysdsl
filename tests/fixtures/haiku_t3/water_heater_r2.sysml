package WaterHeaterR2Types {
    doc /* Layer 1: events (state-machine triggers) and interfaces. */
    private import SysmlStdFlows::*;

    item def HeatDemand {
        doc /* Thermostat requests heating (tank below its setpoint). Field-less trigger. */
    }

    item def HeatSatisfied {
        doc /* Thermostat is satisfied (tank at its setpoint). Field-less trigger. */
    }

    item def CutoffOpened {
        doc /* Independent mechanical thermal cutoff has opened. Field-less trigger. */
    }

    item def ResetPressed {
        doc /* Reset request after a cutoff operation. Field-less trigger. */
    }

    abstract interface def WaterHeaterChannel {
        doc /* Abstract classification base for the custom arrows of this model (no ends needed). */
    }

    interface def RelayCommandLink :> WaterHeaterChannel {
        doc /* Controller relay command to the heating element (on/off). */
        end source : CommandPort;
        end sink : ~CommandPort;
    }

    interface def TemperatureSenseLink :> WaterHeaterChannel {
        doc /* NTC sensor reading to the thermostat controller. */
        end source : TelemetryPort;
        end sink : ~TelemetryPort;
    }
}

package WaterHeaterR2Requirements {
    doc /* Layer 2: requirements with explicit document provenance (WH-SPEC rev A). */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'WH-001'> OutletTemperatureLimit {
        doc /* WH-001: Water outlet temperature shall not exceed 358 K. */
        @Rationale {
            text = "Source: WH-SPEC rev A, section 2.1";
        }
        attribute maxOutletTempK : Real default 358.0 [K] {
            doc /* LIMIT from WH-001. */
        }
        attribute measuredOutletTempK : Real {
            doc /* [evidence] EVIDENCE: measured outlet temperature, supplied by a verification case (no default). */
        }
        require constraint { measuredOutletTempK <= maxOutletTempK }
    }

    requirement def <'WH-002'> CutoffOpenTemperatureLimit {
        doc /* WH-002: The thermal cutoff shall open at or below 368 K. */
        @Rationale {
            text = "Source: WH-SPEC rev A, section 2.2";
        }
        attribute maxCutoffOpenTempK : Real default 368.0 [K] {
            doc /* LIMIT from WH-002. */
        }
        attribute measuredCutoffOpenTempK : Real {
            doc /* [evidence] EVIDENCE: measured cutoff opening temperature (no default). */
        }
        require constraint { measuredCutoffOpenTempK <= maxCutoffOpenTempK }
    }

    requirement def <'WH-003'> ElementPowerLimit {
        doc /* WH-003: Heating element power shall not exceed 2000 W. */
        @Rationale {
            text = "Source: WH-SPEC rev A, section 3.1";
        }
        attribute maxElementPowerW : Real default 2000.0 [W] {
            doc /* LIMIT from WH-003. */
        }
        attribute measuredElementPowerW : Real {
            doc /* [evidence] EVIDENCE: measured element power (no default). */
        }
        require constraint { measuredElementPowerW <= maxElementPowerW }
    }
}

package WaterHeaterR2Subsystems {
    doc /* Layer 3: part definitions (boxes), operating modes and calculations. */
    private import WaterHeaterR2Types::*;
    private import ScalarValues::*;
    private import SI::*;
    private import SysmlStdFlows::*;
    private import SysmlStdPower::*;
    private import SysmlStdElectronics::*;

    action def SenseTankTemperature {
        doc /* Action: sample the tank temperature. */
    }

    action def ControlHeatingElement {
        doc /* Action: thermostat decides and commands the heating element on/off. */
    }

    action def HeatWater {
        doc /* Action: convert electrical power into heat delivered to the tank. */
    }

    action def CutOffElementPower {
        doc /* Action: disconnect element power independently of the controller. */
    }

    part def MainsSupply {
        doc /* Mains supply, 230 V (the spec's mains voltage). */
        attribute nominalVoltage : Real default 230.0 [V];
        port powerOut : ElectricalPowerPort;
    }

    part def HeatingElement :> ResistiveHeater {
        doc /*
            Resistive heating element, 2000 W (specializes the sysml-std resistive heater).
            Inherited std placeholders not given by the source are blanked as TBDs.
        */
        attribute :>> ratedPower default 2000.0 [W];
        attribute :>> mass {
            @TbdMetadata::TBD {
                category = "value";
                reason = "element mass not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        attribute :>> efficiency {
            @TbdMetadata::TBD {
                category = "value";
                reason = "element efficiency not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        attribute :>> wasteHeat {
            @TbdMetadata::TBD {
                category = "value";
                reason = "element waste heat not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        port :>> powerIn;
        port :>> heatDelivery;
        port relayIn : ~CommandPort {
            doc /* Relay command input (on/off), driven by the thermostat controller. */
        }
        perform action heat : HeatWater;
        satisfy requirement : WaterHeaterR2Requirements::ElementPowerLimit;
    }

    part def NtcTemperatureSensor :> TemperatureSensor {
        doc /* NTC temperature sensor sampling at 1 Hz (specializes the sysml-std temperature sensor). */
        attribute :>> sampleRate default 1.0 [Hz];
        attribute :>> maxMeasurableTemp {
            @TbdMetadata::TBD {
                category = "value";
                reason = "sensor measurable range not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        attribute :>> mass {
            @TbdMetadata::TBD {
                category = "value";
                reason = "sensor mass not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        attribute :>> supplyVoltage {
            @TbdMetadata::TBD {
                category = "value";
                reason = "sensor supply voltage not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        attribute :>> powerDraw {
            @TbdMetadata::TBD {
                category = "value";
                reason = "sensor power draw not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        port :>> readingOut;
        port :>> thermalSense;
        perform action sense : SenseTankTemperature;
    }

    part def ThermostatController {
        doc /* Thermostat controller: reads the sensor and switches the element through a relay command. */
        attribute setpointTemp : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "thermostat setpoint not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        port sensorIn : ~TelemetryPort;
        port relayOut : CommandPort;
        perform action control : ControlHeatingElement;
        satisfy requirement : WaterHeaterR2Requirements::OutletTemperatureLimit;
    }

    part def WaterTank {
        doc /* Water tank holding 80 L (0.08 m^3) of water. */
        attribute volume : Real default 0.08 [m^3];
        port heatIn : ~ThermalPort;
        port thermalOut : ThermalPort;
    }

    part def ThermalCutoff {
        doc /* Independent mechanical thermal cutoff: disconnects element power without the controller. */
        attribute tripTempK : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "design trip point not stated; WH-002 gives only the 368 K limit";
                resolveBy = "detailed";
            }
        }
        port powerIn : ~ElectricalPowerPort;
        port powerOut : ElectricalPowerPort;
        port thermalSense : ~ThermalPort;
        perform action cut : CutOffElementPower;
        satisfy requirement : WaterHeaterR2Requirements::CutoffOpenTemperatureLimit;
    }

    state def WaterHeaterMode {
        doc /* Operating modes: Idle, Heating, Overtemp (cutoff opened). */
        entry;
        then idle;
        state idle;
        state heating;
        state overtemp;
        transition first idle accept HeatDemand then heating;
        transition first idle accept CutoffOpened then overtemp;
        transition first heating accept HeatSatisfied then idle;
        transition first heating accept CutoffOpened then overtemp;
        transition first overtemp accept ResetPressed then idle {
            @TbdMetadata::TBD {
                category = "element";
                reason = "reset after a cutoff operation is not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
    }

    calc def elementPowerMarginW {
        doc /* Element power margin against its rated limit (W). */
        in maxPowerW : Real;
        in elementPowerW : Real;
        attribute margin = maxPowerW - elementPowerW;
        return result : Real = margin;
    }
}

package WaterHeaterR2Context {
    doc /* Layer 4: the single top-level context composing the subsystems and their connections. */
    private import WaterHeaterR2Types::*;
    private import WaterHeaterR2Subsystems::*;
    private import SysmlStdFlows::*;

    part def WaterHeaterR2System {
        doc /* Top-level water heater context. */
        part mains : MainsSupply;
        part cutoff : ThermalCutoff;
        part heater : HeatingElement;
        part controller : ThermostatController;
        part sensor : NtcTemperatureSensor;
        part tank : WaterTank;
        interface mainsToCutoff : ElectricalLink connect mains.powerOut to cutoff.powerIn;
        interface cutoffToElement : ElectricalLink connect cutoff.powerOut to heater.powerIn;
        interface controllerToRelay : RelayCommandLink connect controller.relayOut to heater.relayIn;
        interface sensorToController : TemperatureSenseLink connect sensor.readingOut to controller.sensorIn;
        interface elementToTank : ThermalLink connect heater.heatDelivery to tank.heatIn;
        connection tankToSensor connect tank.thermalOut to sensor.thermalSense {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "how the NTC sensor is coupled to the tank water is not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
        connection tankToCutoff connect tank.thermalOut to cutoff.thermalSense {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "how the thermal cutoff senses the tank temperature is not stated in WH-SPEC rev A";
                resolveBy = "detailed";
            }
        }
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
