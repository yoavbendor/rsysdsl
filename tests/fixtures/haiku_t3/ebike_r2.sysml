package EbikeR2Types {
    doc /* Layer 1: payloads, ports and interfaces. */
    private import ScalarValues::*;
    private import SI::*;

    abstract interface def EbikeChannel {
        doc /* Abstract classification base for the drive-system arrows (no ends needed). */
    }

    item def DcPowerFlow {
        doc /* DC power from the battery pack / BMS to the motor controller. */
        attribute voltage : Real default 0.0 [V];
    }

    item def ThreePhaseFlow {
        doc /* Three-phase drive power from the motor controller to the hub motor. */
        attribute power : Real default 0.0 [W];
    }

    item def TorqueSignal {
        doc /* Pedal torque reading from the pedal torque sensor. */
        @TbdMetadata::TBD {
            category = "element";
            reason = "EB-SPEC v2 names the pedal torque sensor but gives no torque signal unit or range";
            owner = "systems";
            resolveBy = "detailed";
        }
    }

    item def SpeedSignal {
        doc /* Wheel speed reading (the spec gives the cutoff speed in m/s, so the signal is m/s). */
        attribute speed : Real default 0.0 [m/s];
    }

    item def CanFrame {
        doc /* CAN frame carrying state of charge (BMS -> display) and assist level (motor controller -> display). */
        @TbdMetadata::TBD {
            category = "element";
            reason = "EB-SPEC v2 names the CAN bus and the SoC / assist-level data but not message IDs or payload layout";
            owner = "systems";
            resolveBy = "detailed";
        }
    }

    item def PowerOnRequest {
        doc /* Mode-machine trigger: rider / system powers the drive on. */
    }

    item def PowerOffRequest {
        doc /* Mode-machine trigger: drive is powered off. */
    }

    item def FaultDetected {
        doc /* Mode-machine trigger: a fault is detected. */
    }

    item def FaultCleared {
        doc /* Mode-machine trigger: a fault is cleared. */
    }

    port def DcPowerPort {
        doc /* DC power port (directed): source uses `DcPowerPort`, sink uses `Conjugated<DcPowerPort>`. */
        out item dc : DcPowerFlow;
    }

    port def ThreePhasePort {
        doc /* Three-phase drive port (directed). */
        out item phase : ThreePhaseFlow;
    }

    port def TorquePort {
        doc /* Pedal torque port (directed). */
        out item torque : TorqueSignal;
    }

    port def SpeedPort {
        doc /* Wheel speed port (directed). */
        out item speed : SpeedSignal;
    }

    port def CanBusPort {
        doc /* CAN bus port (symmetric bus endpoint). */
        inout item 'frame' : CanFrame;
    }

    interface def DcPowerLink :> EbikeChannel {
        end source : DcPowerPort;
        end sink : ~DcPowerPort;
    }

    interface def ThreePhaseLink :> EbikeChannel {
        end source : ThreePhasePort;
        end sink : ~ThreePhasePort;
    }

    interface def TorqueLink :> EbikeChannel {
        end source : TorquePort;
        end sink : ~TorquePort;
    }

    interface def SpeedLink :> EbikeChannel {
        end source : SpeedPort;
        end sink : ~SpeedPort;
    }

    interface def CanLink :> EbikeChannel {
        end a : CanBusPort;
        end b : CanBusPort;
    }
}

package EbikeR2Requirements {
    doc /* Layer 2: requirements with document provenance, plus pure calculations. */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'EB-01'> ContinuousMotorPowerLimit {
        doc /* EB-01: Continuous motor power shall not exceed 250 W. */
        @Rationale {
            text = "Source: EB-SPEC v2, §4.1";
        }
        attribute maxMotorPowerW : Real default 250.0 [W] {
            doc /* LIMIT from EB-01. */
        }
        attribute measuredMotorPowerW : Real {
            doc /* [evidence] EVIDENCE: measured continuous motor power, supplied by a verification case. */
        }
        require constraint { measuredMotorPowerW <= maxMotorPowerW }
    }

    requirement def <'EB-02'> AssistCutoffSpeedLimit {
        doc /* EB-02: Motor assistance shall cut off above 6.94 m/s (25 km/h). */
        @Rationale {
            text = "Source: EB-SPEC v2, §4.2";
        }
        attribute maxAssistSpeedMs : Real default 6.94 [m/s] {
            doc /* LIMIT from EB-02 (the spec's 25 km/h is stated only as 6.94 m/s here). */
        }
        attribute measuredCutoffSpeedMs : Real {
            doc /* [evidence] EVIDENCE: measured assist cutoff speed, supplied by a verification case. */
        }
        require constraint { measuredCutoffSpeedMs <= maxAssistSpeedMs }
    }

    requirement def <'EB-03'> BatteryVoltageLimit {
        doc /* EB-03: Battery voltage shall stay at or below 42 V. */
        @Rationale {
            text = "Source: EB-SPEC v2, §5.3";
        }
        attribute maxBatteryVoltageV : Real default 42.0 [V] {
            doc /* LIMIT from EB-03. */
        }
        attribute measuredBatteryVoltageV : Real {
            doc /* [evidence] EVIDENCE: measured battery voltage, supplied by a verification case. */
        }
        require constraint { measuredBatteryVoltageV <= maxBatteryVoltageV }
    }

    calc def powerMarginW {
        doc /* Margin of a power value against its limit (positive = within limit). */
        in limitW : Real;
        in actualW : Real;
        attribute margin = limitW - actualW;
        return result : Real = margin;
    }

    calc def voltageMarginV {
        doc /* Margin of a voltage value against its limit (positive = within limit). */
        in limitV : Real;
        in actualV : Real;
        attribute margin = limitV - actualV;
        return result : Real = margin;
    }
}

package EbikeR2Subsystems {
    doc /* Layer 3: subsystem blueprints (`part def`) and the motor controller mode machine. */
    private import EbikeR2Types::*;
    private import ScalarValues::*;
    private import SI::*;

    part def BatteryPack {
        doc /* 36 V battery pack, 14 Ah capacity (EB-SPEC v2 system description). */
        attribute nominalVoltage : Real default 36.0 [V];
        attribute capacity : Real default 14.0 [A*h];
        port dcOut : DcPowerPort;
        satisfy requirement : EbikeR2Requirements::BatteryVoltageLimit;
    }

    part def BatteryManagementSystem {
        doc /* Battery management system: supplies DC power and reports state of charge on CAN. */
        port packIn : ~DcPowerPort;
        port dcOut : DcPowerPort;
        port can : CanBusPort;
    }

    part def MotorController {
        doc /* Motor controller: reads pedal torque and wheel speed, drives the hub motor, reports assist level. */
        attribute assistCutoffSpeed : Real default 6.94 [m/s] {
            doc /* Assist cutoff speed from EB-02 (6.94 m/s). */
        }
        port dcIn : ~DcPowerPort;
        port torqueIn : ~TorquePort;
        port speedIn : ~SpeedPort;
        port phaseOut : ThreePhasePort;
        port can : CanBusPort;
        satisfy requirement : EbikeR2Requirements::AssistCutoffSpeedLimit;
    }

    state def AssistMode {
        doc /* Operating modes of the motor controller. Speed above cutoff gives NoAssist. */
        ref part target : MotorController;
        entry;
        then off;
        state off;
        state assist;
        state noAssist;
        state fault;
        transition first off accept PowerOnRequest then assist;
        transition first assist accept payload : SpeedSignal if payload.speed > target.assistCutoffSpeed then noAssist;
        transition first assist accept PowerOffRequest then off;
        transition first assist accept FaultDetected then fault;
        transition first noAssist accept payload : SpeedSignal if payload.speed <= target.assistCutoffSpeed then assist;
        transition first noAssist accept PowerOffRequest then off;
        transition first noAssist accept FaultDetected then fault;
        transition first fault accept FaultCleared then off;
    }

    part def HubMotor {
        doc /* 250 W hub motor driven by 3-phase power. */
        attribute ratedPower : Real default 250.0 [W];
        port phaseIn : ~ThreePhasePort;
        satisfy requirement : EbikeR2Requirements::ContinuousMotorPowerLimit;
    }

    part def PedalTorqueSensor {
        doc /* Pedal torque sensor. */
        port torqueOut : TorquePort;
    }

    part def WheelSpeedSensor {
        doc /* Wheel speed sensor. */
        port speedOut : SpeedPort;
    }

    part def HandlebarDisplay {
        doc /* Handlebar display: receives SoC and assist level over CAN. */
        port can : CanBusPort;
    }
}

package EbikeR2Context {
    doc /* Layer 4: the single top-level context composing the subsystems and their links. */
    private import EbikeR2Types::*;
    private import EbikeR2Subsystems::*;

    part def EbikeR2System {
        doc /* Top-level e-bike pedal-assist drive context. */
        part battery : BatteryPack;
        part bms : BatteryManagementSystem;
        part motorController : MotorController;
        part hubMotor : HubMotor;
        part pedalSensor : PedalTorqueSensor;
        part wheelSensor : WheelSpeedSensor;
        part display : HandlebarDisplay;
        connection packToBms connect battery.dcOut to bms.packIn {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "EB-SPEC v2 names the battery pack and the BMS but does not specify the pack-to-BMS link";
                resolveBy = "detailed";
            }
        }
        interface bmsToMotorController : DcPowerLink connect bms.dcOut to motorController.dcIn;
        interface motorControllerToHubMotor : ThreePhaseLink connect motorController.phaseOut to hubMotor.phaseIn;
        interface pedalTorqueToMotorController : TorqueLink connect pedalSensor.torqueOut to motorController.torqueIn;
        interface wheelSpeedToMotorController : SpeedLink connect wheelSensor.speedOut to motorController.speedIn;
        interface bmsToDisplayCan : CanLink connect bms.can to display.can;
        interface motorControllerToDisplayCan : CanLink connect motorController.can to display.can;
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
