package GreenhouseR2Types {
    doc /* Layer 1: payloads, ports and interfaces (the "arrows"). */
    private import ScalarValues::*;
    private import SI::*;

    abstract interface def GreenhouseChannel {
        doc /* Abstract classification base for every greenhouse arrow (no ends needed). */
    }

    item def AirTemperatureReading {
        doc /* Air temperature reading sent from the air temperature sensor to the controller. */
        attribute temp : Real default 0.0 [K];
    }

    item def SoilMoistureReading {
        doc /* Soil moisture reading sent from the soil moisture sensor to the controller. */
        attribute moisture : Real default 0.0 {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "GH-CONOPS 1.3 gives no soil moisture measure or unit";
                resolveBy = "detailed";
            }
        }
    }

    item def DcPowerFlow {
        doc /* 24 V DC supply power flow. */
        attribute voltage : Real default 24.0 [V];
    }

    item def MainsPowerFlow {
        doc /* 230 V mains power flow (heater and pump). */
        attribute voltage : Real default 230.0 [V];
    }

    item def HeaterCommand {
        doc /* Heater power command from the controller. */
        attribute powerSetpoint : Real default 0.0 [W];
    }

    item def FanCommand {
        doc /* Ventilation fan run command from the controller. */
        attribute run : Boolean;
    }

    item def ValveCommand {
        doc /* Irrigation solenoid valve command from the controller. */
        attribute open : Boolean;
    }

    item def IrrigationElapsed {
        doc /* Elapsed continuous irrigation time (timer event for the Irrigating mode). */
        attribute elapsed : Real default 0.0 [s];
    }

    item def SensorDropout {
        doc /* Field-less event: a sensor stops delivering readings. */
    }

    port def DcPowerPort {
        out item dc : DcPowerFlow;
    }

    port def MainsPowerPort {
        out item mains : MainsPowerFlow;
    }

    port def AirTemperaturePort {
        out item reading : AirTemperatureReading;
    }

    port def SoilMoisturePort {
        out item reading : SoilMoistureReading;
    }

    port def HeaterCommandPort {
        out item cmd : HeaterCommand;
    }

    port def FanCommandPort {
        out item cmd : FanCommand;
    }

    port def ValveCommandPort {
        out item cmd : ValveCommand;
    }

    interface def DcPowerLink :> GreenhouseChannel {
        end source : DcPowerPort;
        end sink : ~DcPowerPort;
    }

    interface def MainsPowerLink :> GreenhouseChannel {
        end source : MainsPowerPort;
        end sink : ~MainsPowerPort;
    }

    interface def AirTemperatureLink :> GreenhouseChannel {
        end source : AirTemperaturePort;
        end sink : ~AirTemperaturePort;
    }

    interface def SoilMoistureLink :> GreenhouseChannel {
        end source : SoilMoisturePort;
        end sink : ~SoilMoisturePort;
    }

    interface def HeaterCommandLink :> GreenhouseChannel {
        end source : HeaterCommandPort;
        end sink : ~HeaterCommandPort;
    }

    interface def FanCommandLink :> GreenhouseChannel {
        end source : FanCommandPort;
        end sink : ~FanCommandPort;
    }

    interface def ValveCommandLink :> GreenhouseChannel {
        end source : ValveCommandPort;
        end sink : ~ValveCommandPort;
    }
}

package GreenhouseR2Requirements {
    doc /* Layer 2: requirements with document provenance (Source: GH-CONOPS 1.3). */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'GH-R1'> AirTemperatureBand {
        doc /* GH-R1: Air temperature shall be kept between 288 K and 303 K. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 3";
        }
        attribute minAirTempK : Real default 288.0 [K];
        attribute maxAirTempK : Real default 303.0 [K];
        attribute measuredAirTempK : Real {
            doc /* [evidence] EVIDENCE: no default; a verification case supplies it. */
        }
        require constraint { measuredAirTempK >= minAirTempK and measuredAirTempK <= maxAirTempK }
    }

    requirement def <'GH-R2'> IrrigationRunLimit {
        doc /* GH-R2: Irrigation shall not run for more than 600 s continuously. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 4";
        }
        attribute maxIrrigationRunS : Real default 600.0 [s];
        attribute measuredIrrigationRunS : Real {
            doc /* [evidence] EVIDENCE: no default; a verification case supplies it. */
        }
        require constraint { measuredIrrigationRunS <= maxIrrigationRunS }
    }

    requirement def <'GH-R3'> HeaterPowerLimit {
        doc /* GH-R3: Heater power shall not exceed 1500 W. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 6";
        }
        attribute maxHeaterPowerW : Real default 1500.0 [W];
        attribute measuredHeaterPowerW : Real {
            doc /* [evidence] EVIDENCE: no default; a verification case supplies it. */
        }
        require constraint { measuredHeaterPowerW <= maxHeaterPowerW }
    }
}

package GreenhouseR2Subsystems {
    doc /* Layer 3: subsystem blueprints (the "boxes"), operating modes and calculations. */
    private import GreenhouseR2Types::*;
    private import ScalarValues::*;
    private import SI::*;

    abstract part def GreenhouseSensor {
        doc /* Abstract sensor base: 24 V supply input and sampling rate (rate not given in the source). */
        attribute sampleRate : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "sensor sample rate not given in GH-CONOPS 1.3";
                resolveBy = "detailed";
            }
        }
        port powerIn : ~DcPowerPort;
    }

    part def AirTemperatureSensor :> GreenhouseSensor {
        port :>> powerIn;
        port readingOut : AirTemperaturePort;
    }

    part def SoilMoistureSensor :> GreenhouseSensor {
        port :>> powerIn;
        port readingOut : SoilMoisturePort;
    }

    part def ElectricHeater {
        doc /* 1500 W electric heater on 230 V mains. */
        attribute ratedPowerW : Real default 1500.0 [W];
        port mainsIn : ~MainsPowerPort;
        port cmdIn : ~HeaterCommandPort;
        satisfy requirement : GreenhouseR2Requirements::HeaterPowerLimit;
    }

    part def VentilationFan {
        doc /* Ventilation fan, flow 0.5 m^3/s. Its electrical supply is not stated in the source. */
        @TbdMetadata::TBD {
            category = "element";
            reason = "fan electrical supply and power rating not given in GH-CONOPS 1.3";
            resolveBy = "preliminary";
        }
        attribute flowRate : Real default 0.5 [m^3/s];
        port cmdIn : ~FanCommandPort;
    }

    part def IrrigationPump {
        doc /* Irrigation pump with solenoid valve, on 230 V mains. The valve is commanded by the controller. */
        @TbdMetadata::TBD {
            category = "element";
            reason = "pump run command path not given in GH-CONOPS 1.3 (only the valve command is stated)";
            resolveBy = "preliminary";
        }
        attribute flowRate : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "pump flow rate not given in GH-CONOPS 1.3";
                resolveBy = "detailed";
            }
        }
        port mainsIn : ~MainsPowerPort;
        port valveCmdIn : ~ValveCommandPort;
    }

    part def MainsSupply230V {
        doc /* 230 V mains source. */
        attribute nominalVoltage : Real default 230.0 [V];
        port mainsOut : MainsPowerPort;
    }

    part def DcPowerSupply24V {
        doc /* 24 V DC supply feeding the controller and sensors. Its own input source is not stated. */
        attribute outputVoltage : Real default 24.0 [V];
        attribute outputCurrent : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "24 V supply output current not given in GH-CONOPS 1.3";
                resolveBy = "detailed";
            }
        }
        port dcOut : DcPowerPort;
    }

    action def RegulateClimate {
        doc /* Function: regulate air temperature and irrigation run time. */
    }

    part def ClimateController {
        doc /* Climate controller: receives sensor readings, commands heater, fan and irrigation valve. */
        attribute minAirTempK : Real default 288.0 [K] {
            doc /* Lower air temperature setpoint (GH-R1 limit). */
        }
        attribute maxAirTempK : Real default 303.0 [K] {
            doc /* Upper air temperature setpoint (GH-R1 limit). */
        }
        attribute maxIrrigationRunS : Real default 600.0 [s] {
            doc /* Continuous irrigation limit (GH-R2 limit). */
        }
        port dcIn : ~DcPowerPort;
        port tempIn : ~AirTemperaturePort;
        port moistureIn : ~SoilMoisturePort;
        port heaterCmd : HeaterCommandPort;
        port fanCmd : FanCommandPort;
        port valveCmd : ValveCommandPort;
        perform action regulate : RegulateClimate;
        satisfy requirement : GreenhouseR2Requirements::AirTemperatureBand;
        satisfy requirement : GreenhouseR2Requirements::IrrigationRunLimit;
    }

    state def ClimateMode {
        doc /* Operating modes of the climate controller. */
        ref part target : ClimateController;
        entry;
        then normal;
        state normal;
        state heating;
        state venting;
        state irrigating;
        state sensorFault {
            doc /* Latched fault: GH-CONOPS 1.3 gives no recovery path, so no exit transition is modeled. */
        }
        transition first normal accept payload : AirTemperatureReading if payload.temp < target.minAirTempK then heating;
        transition first normal accept payload : AirTemperatureReading if payload.temp > target.maxAirTempK then venting;
        transition first normal accept SoilMoistureReading then irrigating {
            @TbdMetadata::TBD {
                category = "element";
                reason = "soil moisture irrigation trigger threshold not given in GH-CONOPS 1.3";
                resolveBy = "detailed";
            }
        }
        transition first normal accept SensorDropout then sensorFault {
            @TbdMetadata::TBD {
                category = "element";
                reason = "sensor fault detection criteria not given in GH-CONOPS 1.3";
                resolveBy = "detailed";
            }
        }
        transition first heating accept payload : AirTemperatureReading if payload.temp >= target.minAirTempK then normal;
        transition first venting accept payload : AirTemperatureReading if payload.temp <= target.maxAirTempK then normal;
        transition first irrigating accept payload : IrrigationElapsed if payload.elapsed >= target.maxIrrigationRunS then normal;
    }

    calc def heaterMarginW {
        doc /* Pure calculation: heater power margin to the GH-R3 limit. */
        in limitW : Real;
        in powerW : Real;
        return result : Real = limitW - powerW;
    }
}

package GreenhouseR2Context {
    doc /* Layer 4: the single top-level context that composes and wires the subsystems. */
    private import GreenhouseR2Types::*;
    private import GreenhouseR2Subsystems::*;

    part def GreenhouseR2System {
        doc /* Top-level greenhouse climate system. */
        part controller : ClimateController;
        part airSensor : AirTemperatureSensor;
        part soilSensor : SoilMoistureSensor;
        part heater : ElectricHeater;
        part fan : VentilationFan;
        part irrigation : IrrigationPump;
        part dcSupply : DcPowerSupply24V;
        part mains : MainsSupply230V;
        interface dcToController : DcPowerLink connect dcSupply.dcOut to controller.dcIn;
        interface dcToAirSensor : DcPowerLink connect dcSupply.dcOut to airSensor.powerIn;
        interface dcToSoilSensor : DcPowerLink connect dcSupply.dcOut to soilSensor.powerIn;
        interface airTempToController : AirTemperatureLink connect airSensor.readingOut to controller.tempIn;
        interface soilToController : SoilMoistureLink connect soilSensor.readingOut to controller.moistureIn;
        interface controllerToHeater : HeaterCommandLink connect controller.heaterCmd to heater.cmdIn;
        interface controllerToFan : FanCommandLink connect controller.fanCmd to fan.cmdIn;
        interface controllerToValve : ValveCommandLink connect controller.valveCmd to irrigation.valveCmdIn;
        interface mainsToHeater : MainsPowerLink connect mains.mainsOut to heater.mainsIn;
        interface mainsToIrrigation : MainsPowerLink connect mains.mainsOut to irrigation.mainsIn;
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
