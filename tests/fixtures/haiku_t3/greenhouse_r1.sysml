package GreenhouseR1Types {
    doc /* Layer 1: readings, ports and interfaces. */
    private import ScalarValues::*;
    private import SI::*;
    private import SysmlStdFlows::*;

    item def AirTempReading {
        doc /* Air temperature reading (K) with sensor health flag. */
        attribute tempK : Real;
        attribute sensorOk : Boolean;
    }

    item def SoilMoistureReading {
        doc /* Soil moisture reading. GH-CONOPS 1.3 gives neither a unit nor a range. */
        attribute moisture : Real {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "GH-CONOPS 1.3 gives no soil moisture unit or range";
                resolveBy = "detailed";
            }
        }
        attribute sensorOk : Boolean;
    }

    item def IrrigationTick {
        doc /* Run time of the current irrigation cycle (for the GH-R2 limit). */
        attribute runS : Real;
    }

    item def IrrigationDemand {
        doc /* Field-less trigger: the soil reading calls for irrigation. */
    }

    port def AirTempPort {
        doc /* Directed air temperature output port (sensor side). */
        out item reading : AirTempReading;
    }

    port def SoilMoisturePort {
        doc /* Directed soil moisture output port (sensor side). */
        out item reading : SoilMoistureReading;
    }

    abstract interface def GreenhouseChannel {
        doc /* Abstract classification base for every arrow in the greenhouse. */
    }

    interface def AirTempLink :> GreenhouseChannel {
        doc /* Air temperature readings, sensor to controller. */
        end tx : AirTempPort;
        end rx : ~AirTempPort;
    }

    interface def SoilMoistureLink :> GreenhouseChannel {
        doc /* Soil moisture readings, sensor to controller. */
        end tx : SoilMoisturePort;
        end rx : ~SoilMoisturePort;
    }

    interface def PowerLink :> GreenhouseChannel {
        doc /* Electrical power distribution (24 V DC or 230 V mains). */
        end supply : ElectricalPowerPort;
        end load : ~ElectricalPowerPort;
    }

    interface def CommandLink :> GreenhouseChannel {
        doc /* Controller commands to heater, fan and irrigation. */
        end tx : CommandPort;
        end rx : ~CommandPort;
    }
}

package GreenhouseR1Requirements {
    doc /* Layer 2: requirements with document provenance, plus margin calculations. */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'GH-R1'> AirTempBand {
        doc /* GH-R1: Air temperature shall be kept between 288 K and 303 K. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 3 (GH-R1)";
        }
        attribute minAirTempK : Real default 288.0 [K] {
            doc /* Lower limit from GH-R1. */
        }
        attribute maxAirTempK : Real default 303.0 [K] {
            doc /* Upper limit from GH-R1. */
        }
        attribute measuredAirTempK : Real {
            doc /* [evidence] EVIDENCE: measured air temperature, supplied by a verification case. */
        }
        require constraint { measuredAirTempK >= minAirTempK and measuredAirTempK <= maxAirTempK }
    }

    requirement def <'GH-R2'> IrrigationRunLimit {
        doc /* GH-R2: Irrigation shall not run for more than 600 s continuously. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 4 (GH-R2)";
        }
        attribute maxRunS : Real default 600.0 [s] {
            doc /* Limit from GH-R2. */
        }
        attribute measuredRunS : Real {
            doc /* [evidence] EVIDENCE: measured continuous irrigation run time. */
        }
        require constraint { measuredRunS <= maxRunS }
    }

    requirement def <'GH-R3'> HeaterPowerLimit {
        doc /* GH-R3: Heater power shall not exceed 1500 W. */
        @Rationale {
            text = "Source: GH-CONOPS 1.3, section 6 (GH-R3)";
        }
        attribute maxPowerW : Real default 1500.0 [W] {
            doc /* Limit from GH-R3. */
        }
        attribute measuredPowerW : Real {
            doc /* [evidence] EVIDENCE: measured heater power. */
        }
        require constraint { measuredPowerW <= maxPowerW }
    }

    calc def irrigationMarginS {
        doc /* Margin to the GH-R2 run limit (positive means within the limit). */
        in limitS : Real;
        in runS : Real;
        return result : Real = limitS - runS;
    }

    calc def heaterPowerMarginW {
        doc /* Margin to the GH-R3 power limit (positive means within the limit). */
        in limitW : Real;
        in powerW : Real;
        return result : Real = limitW - powerW;
    }
}

package GreenhouseR1Subsystems {
    doc /* Layer 3: part definitions (boxes), the operating-mode state machine. */
    private import GreenhouseR1Types::*;
    private import ScalarValues::*;
    private import SI::*;
    private import SysmlStdFlows::*;

    part def DcSupply24V {
        doc /* 24 V DC power supply feeding the controller and the sensors. Its input source is not stated. */
        @TbdMetadata::TBD {
            category = "element";
            reason = "24 V DC supply input source not stated in GH-CONOPS 1.3";
            resolveBy = "preliminary";
        }
        attribute outputVoltage : Real default 24.0 [V];
        port powerOut : ElectricalPowerPort;
    }

    part def MainsSupply230V {
        doc /* 230 V mains supply feeding the heater and the irrigation pump. */
        attribute nominalVoltage : Real default 230.0 [V];
        port powerOut : ElectricalPowerPort;
    }

    part def AirTemperatureSensor {
        doc /* Air temperature sensor: sends readings to the climate controller. */
        port powerIn : ~ElectricalPowerPort;
        port readingOut : AirTempPort;
    }

    part def SoilMoistureSensor {
        doc /* Soil moisture sensor: sends readings to the climate controller. */
        port powerIn : ~ElectricalPowerPort;
        port readingOut : SoilMoisturePort;
    }

    part def ClimateController {
        doc /*
            Climate controller: reads both sensors and commands heater, fan and irrigation.
            Its limits are the GH-R1 band and the GH-R2 run limit; it is the state machine subject.
        */
        attribute minAirTempK : Real default 288.0 [K];
        attribute maxAirTempK : Real default 303.0 [K];
        attribute maxIrrigationRunS : Real default 600.0 [s];
        port powerIn : ~ElectricalPowerPort;
        port airIn : ~AirTempPort;
        port soilIn : ~SoilMoisturePort;
        port heaterCmd : CommandPort;
        port fanCmd : CommandPort;
        port irrigationCmd : CommandPort;
        satisfy requirement : GreenhouseR1Requirements::AirTempBand;
        satisfy requirement : GreenhouseR1Requirements::IrrigationRunLimit;
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
        state sensorFault;
        transition first normal accept payload : AirTempReading if not payload.sensorOk then sensorFault;
        transition first normal accept payload : AirTempReading if payload.sensorOk and payload.tempK < target.minAirTempK then heating;
        transition first normal accept payload : AirTempReading if payload.sensorOk and payload.tempK > target.maxAirTempK then venting;
        transition first normal accept IrrigationDemand then irrigating {
            @TbdMetadata::TBD {
                category = "element";
                reason = "soil moisture trigger threshold not stated in GH-CONOPS 1.3";
                resolveBy = "detailed";
            }
        }
        transition first heating accept payload : AirTempReading if not payload.sensorOk then sensorFault;
        transition first heating accept payload : AirTempReading if payload.sensorOk and payload.tempK >= target.minAirTempK then normal;
        transition first venting accept payload : AirTempReading if not payload.sensorOk then sensorFault;
        transition first venting accept payload : AirTempReading if payload.sensorOk and payload.tempK <= target.maxAirTempK then normal;
        transition first irrigating accept payload : IrrigationTick if payload.runS > target.maxIrrigationRunS then normal;
        transition first sensorFault accept payload : AirTempReading if payload.sensorOk then normal;
    }

    part def ElectricHeater {
        doc /* Electric heater, rated 1500 W, powered from 230 V mains. Satisfies GH-R3. */
        attribute ratedPower : Real default 1500.0 [W];
        port powerIn : ~ElectricalPowerPort;
        port cmdIn : ~CommandPort;
        satisfy requirement : GreenhouseR1Requirements::HeaterPowerLimit;
    }

    part def VentilationFan {
        doc /* Ventilation fan, flow 0.5 m^3/s. Its power supply is not stated in GH-CONOPS 1.3. */
        @TbdMetadata::TBD {
            category = "element";
            reason = "ventilation fan power supply not stated in GH-CONOPS 1.3";
            resolveBy = "preliminary";
        }
        attribute flowRate : Real default 0.5 [m^3/s];
        port cmdIn : ~CommandPort;
    }

    part def IrrigationPump {
        doc /*
            Irrigation pump with solenoid valve, powered from 230 V mains. One command runs the
            pump and opens the valve. Its flow and power ratings are not stated.
        */
        @TbdMetadata::TBD {
            category = "element";
            reason = "irrigation pump and solenoid valve ratings (flow, power) not stated in GH-CONOPS 1.3";
            resolveBy = "preliminary";
        }
        port powerIn : ~ElectricalPowerPort;
        port cmdIn : ~CommandPort;
    }
}

package GreenhouseR1Context {
    doc /* Layer 4: the single top-level context that composes and wires the subsystems. */
    private import GreenhouseR1Types::*;
    private import GreenhouseR1Subsystems::*;

    part def GreenhouseR1System {
        doc /* Top-level greenhouse climate system: all subsystems and arrows. */
        part dcSupply : DcSupply24V;
        part mains : MainsSupply230V;
        part airSensor : AirTemperatureSensor;
        part soilSensor : SoilMoistureSensor;
        part controller : ClimateController;
        part heater : ElectricHeater;
        part fan : VentilationFan;
        part pump : IrrigationPump;
        interface airTempToController : AirTempLink connect airSensor.readingOut to controller.airIn;
        interface soilToController : SoilMoistureLink connect soilSensor.readingOut to controller.soilIn;
        interface dcToController : PowerLink connect dcSupply.powerOut to controller.powerIn;
        interface dcToAirSensor : PowerLink connect dcSupply.powerOut to airSensor.powerIn;
        interface dcToSoilSensor : PowerLink connect dcSupply.powerOut to soilSensor.powerIn;
        interface mainsToHeater : PowerLink connect mains.powerOut to heater.powerIn;
        interface mainsToIrrigationPump : PowerLink connect mains.powerOut to pump.powerIn;
        interface controllerToHeater : CommandLink connect controller.heaterCmd to heater.cmdIn;
        interface controllerToFan : CommandLink connect controller.fanCmd to fan.cmdIn;
        interface controllerToIrrigation : CommandLink connect controller.irrigationCmd to pump.cmdIn;
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
