package GreenhouseR3Types {
    doc /* Layer 1: payloads, ports and interfaces (the arrows). */
    private import ScalarValues::*;
    private import SI::*;
    private import SysmlStdFlows::*;

    item def AirTempReading {
        doc /* Air temperature sample from the air temperature sensor to the controller. */
        attribute tempK : Real;
    }

    item def SoilMoistureReading {
        doc /*
            Soil moisture sample from the soil moisture sensor to the controller.
            The source gives no unit or range for the reading.
        */
        attribute moisture : Real {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "GH-CONOPS 1.3 gives no soil moisture unit or range";
                resolveBy = "detailed";
            }
        }
    }

    item def HeaterCommand {
        doc /* Heater power setpoint from the controller to the heater. */
        attribute powerW : Real default 0.0 [W];
    }

    item def FanCommand {
        doc /* Ventilation fan flow setpoint from the controller to the fan. */
        attribute flowRate : Real default 0.0 [m^3/s];
    }

    item def ValveCommand {
        doc /* Irrigation valve command from the controller to the solenoid valve. */
        attribute open : Boolean;
    }

    item def IrrigationTimer {
        doc /* Timer event: elapsed continuous irrigation run time. */
        attribute runS : Real;
    }

    item def SensorFaultDetected {
        doc /* Fault event raised by the controller when a sensor reading is lost. */
    }

    item def SensorRecovered {
        doc /* Event that clears a sensor fault. */
    }

    port def AirTempPort {
        doc /* Directed air temperature output port (connect to `Conjugated<AirTempPort>`). */
        out item reading : AirTempReading;
    }

    port def SoilMoisturePort {
        doc /* Directed soil moisture output port (connect to `Conjugated<SoilMoisturePort>`). */
        out item reading : SoilMoistureReading;
    }

    port def HeaterCmdPort {
        doc /* Directed heater command port (connect to `Conjugated<HeaterCmdPort>`). */
        out item cmd : HeaterCommand;
    }

    port def FanCmdPort {
        doc /* Directed fan command port (connect to `Conjugated<FanCmdPort>`). */
        out item cmd : FanCommand;
    }

    port def ValveCmdPort {
        doc /* Directed valve command port (connect to `Conjugated<ValveCmdPort>`). */
        out item cmd : ValveCommand;
    }

    abstract interface def GreenhouseChannel {
        doc /* Abstract classification base for every arrow type in this system. */
    }

    interface def AirTempLink :> GreenhouseChannel {
        doc /* Air temperature link from the air sensor to the controller. */
        end source : AirTempPort;
        end sink : ~AirTempPort;
    }

    interface def SoilMoistureLink :> GreenhouseChannel {
        doc /* Soil moisture link from the soil sensor to the controller. */
        end source : SoilMoisturePort;
        end sink : ~SoilMoisturePort;
    }

    interface def HeaterCmdLink :> GreenhouseChannel {
        doc /* Command link from the controller to the heater. */
        end source : HeaterCmdPort;
        end sink : ~HeaterCmdPort;
    }

    interface def FanCmdLink :> GreenhouseChannel {
        doc /* Command link from the controller to the ventilation fan. */
        end source : FanCmdPort;
        end sink : ~FanCmdPort;
    }

    interface def ValveCmdLink :> GreenhouseChannel {
        doc /* Command link from the controller to the irrigation solenoid valve. */
        end source : ValveCmdPort;
        end sink : ~ValveCmdPort;
    }

    interface def PowerLink :> GreenhouseChannel {
        doc /* Electrical power link (24 V DC or 230 V mains) from a supply to a load. */
        end supply : ElectricalPowerPort;
        end load : ~ElectricalPowerPort;
    }
}

package GreenhouseR3Requirements {
    doc /* Layer 2: requirements with document provenance, and the calculations that use them. */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'GH-R1'> AirTemperatureBand {
        doc /* GH-R1: Air temperature shall be kept between 288 K and 303 K. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 3";
        }
        attribute minAirTempK : Real default 288.0 [K] {
            doc /* LIMIT from GH-R1. */
        }
        attribute maxAirTempK : Real default 303.0 [K] {
            doc /* LIMIT from GH-R1. */
        }
        attribute measuredAirTempK : Real {
            doc /* [evidence] EVIDENCE: measured air temperature. No default; a verification case supplies it. */
        }
        require constraint { measuredAirTempK >= minAirTempK and measuredAirTempK <= maxAirTempK }
    }

    requirement def <'GH-R2'> IrrigationRunLimit {
        doc /* GH-R2: Irrigation shall not run for more than 600 s continuously. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 4";
        }
        attribute maxRunS : Real default 600.0 [s] {
            doc /* LIMIT from GH-R2. */
        }
        attribute measuredRunS : Real {
            doc /* [evidence] EVIDENCE: measured continuous irrigation run time. No default. */
        }
        require constraint { measuredRunS <= maxRunS }
    }

    requirement def <'GH-R3'> HeaterPowerLimit {
        doc /* GH-R3: Heater power shall not exceed 1500 W. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 6";
        }
        attribute maxHeaterPowerW : Real default 1500.0 [W] {
            doc /* LIMIT from GH-R3. */
        }
        attribute measuredHeaterPowerW : Real {
            doc /* [evidence] EVIDENCE: measured heater power draw. No default. */
        }
        require constraint { measuredHeaterPowerW <= maxHeaterPowerW }
    }

    calc def heaterPowerMarginW {
        doc /* Margin between the heater power limit and the power in use (positive = compliant). */
        in limitW : Real;
        in powerW : Real;
        return result : Real = limitW - powerW;
    }

    calc def irrigationRunMarginS {
        doc /* Margin between the irrigation run limit and the run time so far (zero = limit reached). */
        in limitS : Real;
        in runS : Real;
        return result : Real = limitS - runS;
    }
}

package GreenhouseR3Subsystems {
    doc /*
        Layer 3: reusable subsystem blueprints (`part def`), operating modes and the controller's
        state machine.
    */
    private import GreenhouseR3Types::*;
    private import SysmlStdFlows::*;
    private import SysmlStdPower::*;
    private import SI::*;
    private import ScalarValues::*;

    part def ClimateController {
        doc /*
            Climate controller: reads the sensors, runs the operating modes, and commands the
            heater, fan and irrigation valve. Its thresholds come from GH-R1 and GH-R2.
        */
        attribute lowTempK : Real default 288.0 [K] {
            doc /* Heating threshold: lower bound of the GH-R1 band. */
        }
        attribute highTempK : Real default 303.0 [K] {
            doc /* Venting threshold: upper bound of the GH-R1 band. */
        }
        attribute maxIrrigationRunS : Real default 600.0 [s] {
            doc /* Irrigation stop limit (GH-R2). */
        }
        port airIn : ~AirTempPort;
        port moistureIn : ~SoilMoisturePort;
        port heaterCmdOut : HeaterCmdPort;
        port fanCmdOut : FanCmdPort;
        port valveCmdOut : ValveCmdPort;
        port powerIn : ~ElectricalPowerPort;
        satisfy requirement : GreenhouseR3Requirements::AirTemperatureBand;
        satisfy requirement : GreenhouseR3Requirements::IrrigationRunLimit;
    }

    part def AirTemperatureSensor {
        doc /* Air temperature sensor. */
        port airOut : AirTempPort;
        port powerIn : ~ElectricalPowerPort;
    }

    part def SoilMoistureSensor {
        doc /* Soil moisture sensor. */
        port moistureOut : SoilMoisturePort;
        port powerIn : ~ElectricalPowerPort;
    }

    part def ElectricHeater {
        doc /* 1500 W electric heater, powered from 230 V mains. Satisfies GH-R3. */
        attribute ratedPowerW : Real default 1500.0 [W];
        port powerIn : ~ElectricalPowerPort;
        port cmdIn : ~HeaterCmdPort;
        satisfy requirement : GreenhouseR3Requirements::HeaterPowerLimit;
    }

    part def VentilationFan {
        doc /* Ventilation fan. The source gives its flow but not its supply source or voltage. */
        attribute flowRate : Real default 0.5 [m^3/s];
        attribute supplyVoltage : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no supply source or voltage for the fan";
                resolveBy = "detailed";
            }
        }
        port cmdIn : ~FanCmdPort;
    }

    part def IrrigationPump {
        doc /*
            Irrigation pump with solenoid valve, powered from 230 V mains.
            The source gives no pump power.
        */
        attribute ratedPowerW : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no irrigation pump power";
                resolveBy = "detailed";
            }
        }
        port powerIn : ~ElectricalPowerPort;
        port valveCmdIn : ~ValveCmdPort;
    }

    part def DcPowerSupply {
        doc /* 24 V DC power supply feeding the controller and the sensors. */
        attribute outputVoltage : Real default 24.0 [V];
        port powerOut : ElectricalPowerPort;
    }

    part def MainsSupply :> AcMainsSupply {
        doc /*
            230 V AC mains supply, specializing the sysml-std AC mains supply. The source gives
            the voltage only; the library's current rating and frequency are not in the source.
        */
        attribute :>> nominalVoltage default 230.0 [V];
        attribute :>> mass {
            doc /* PowerElement library defaults below are not in the source: blanked as TBDs. */
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no mains supply mass";
                resolveBy = "detailed";
            }
        }
        attribute :>> ratedPower {
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no mains supply rated power";
                resolveBy = "detailed";
            }
        }
        attribute :>> efficiency {
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no mains supply efficiency";
                resolveBy = "detailed";
            }
        }
        attribute :>> wasteHeat {
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no mains supply waste heat";
                resolveBy = "detailed";
            }
        }
        attribute :>> maxCurrent {
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no mains current rating";
                resolveBy = "detailed";
            }
        }
        attribute :>> mainsFrequency {
            @TbdMetadata::TBD {
                category = "value";
                reason = "GH-CONOPS 1.3 gives no mains frequency";
                resolveBy = "detailed";
            }
        }
        port :>> powerOut;
    }

    state def GreenhouseMode {
        doc /*
            Operating modes of the climate controller.
            Normal is the initial mode. The irrigation trigger threshold is not in the source.
        */
        ref part target : ClimateController;
        entry;
        then normal;
        state normal;
        state heating;
        state venting;
        state irrigating;
        state sensorFault;
        transition first normal accept payload : AirTempReading if payload.tempK < target.lowTempK then heating;
        transition first normal accept payload : AirTempReading if payload.tempK > target.highTempK then venting;
        transition first normal accept SoilMoistureReading then irrigating {
            @TbdMetadata::TBD {
                category = "element";
                reason = "GH-CONOPS 1.3 gives no soil moisture irrigation trigger threshold";
                resolveBy = "detailed";
            }
        }
        transition first normal accept SensorFaultDetected then sensorFault;
        transition first heating accept payload : AirTempReading if payload.tempK >= target.lowTempK then normal;
        transition first heating accept SensorFaultDetected then sensorFault;
        transition first venting accept payload : AirTempReading if payload.tempK <= target.highTempK then normal;
        transition first venting accept SensorFaultDetected then sensorFault;
        transition first irrigating accept payload : IrrigationTimer if payload.runS >= target.maxIrrigationRunS then normal;
        transition first irrigating accept SensorFaultDetected then sensorFault;
        transition first sensorFault accept SensorRecovered then normal;
    }
}

package GreenhouseR3Context {
    doc /* Layer 4: the single top-level context that composes and wires the subsystems. */
    private import GreenhouseR3Types::*;
    private import GreenhouseR3Subsystems::*;

    part def GreenhouseR3System {
        doc /* Top-level greenhouse climate system: all subsystems and all arrows from GH-CONOPS 1.3. */
        part controller : ClimateController;
        part airSensor : AirTemperatureSensor;
        part soilSensor : SoilMoistureSensor;
        part heater : ElectricHeater;
        part fan : VentilationFan;
        part irrigation : IrrigationPump;
        part dcSupply : DcPowerSupply;
        part mains : MainsSupply;
        interface airTempToController : AirTempLink connect airSensor.airOut to controller.airIn;
        interface moistureToController : SoilMoistureLink connect soilSensor.moistureOut to controller.moistureIn;
        interface controllerToHeater : HeaterCmdLink connect controller.heaterCmdOut to heater.cmdIn;
        interface controllerToFan : FanCmdLink connect controller.fanCmdOut to fan.cmdIn;
        interface controllerToValve : ValveCmdLink connect controller.valveCmdOut to irrigation.valveCmdIn;
        interface dcToController : PowerLink connect dcSupply.powerOut to controller.powerIn;
        interface dcToAirSensor : PowerLink connect dcSupply.powerOut to airSensor.powerIn;
        interface dcToSoilSensor : PowerLink connect dcSupply.powerOut to soilSensor.powerIn;
        interface mainsToHeater : PowerLink connect mains.powerOut to heater.powerIn;
        interface mainsToIrrigation : PowerLink connect mains.powerOut to irrigation.powerIn;
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
