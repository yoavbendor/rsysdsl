package EbikeR1Types {
    doc /* Layer 1: payloads, ports, interfaces and the event items used by the drive mode. */
    private import ScalarValues::*;
    private import SI::*;

    item def DcPower {
        doc /* DC power delivered by the BMS to the motor controller. */
        attribute voltage : Real;
        attribute current : Real;
    }

    item def ThreePhasePower {
        doc /* 3-phase drive power from the motor controller to the hub motor. */
    }

    item def TorqueSignal {
        doc /* Pedal torque reading. */
        attribute torque : Real;
    }

    item def SpeedSignal {
        doc /* Wheel speed reading. */
        attribute speed : Real;
    }

    item def CanFrame {
        doc /* CAN payload: the BMS reports state of charge, the motor controller reports assist level. */
        attribute stateOfCharge : Real {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "EB-SPEC v2 names state of charge but gives no unit or scale";
                resolveBy = "detailed";
            }
        }
        attribute assistLevel : Real {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "EB-SPEC v2 names assist level but gives no unit or levels";
                resolveBy = "detailed";
            }
        }
    }

    item def PowerOn {
        doc /* Event: the rider or system switches the drive on. */
    }

    item def PowerOff {
        doc /* Event: the drive is switched off. */
    }

    item def FaultDetected {
        doc /* Event: a fault is detected. */
        @TbdMetadata::TBD {
            category = "element";
            reason = "EB-SPEC v2 names a Fault mode but not the fault detection criteria";
            owner = "systems";
            resolveBy = "preliminary";
        }
    }

    item def ResetFault {
        doc /* Event: the fault is cleared. */
    }

    port def DcPowerPort {
        doc /* DC power port (BMS and battery pack output side). */
        out item dc : DcPower;
    }

    port def ThreePhasePort {
        doc /* 3-phase drive output port of the motor controller. */
        out item phases : ThreePhasePower;
    }

    port def TorqueSensorPort {
        doc /* Pedal torque sensor output port. */
        out item torque : TorqueSignal;
    }

    port def SpeedSensorPort {
        doc /* Wheel speed sensor output port. */
        out item speed : SpeedSignal;
    }

    port def CanBusPort {
        doc /* CAN bus port (both ends use this type). */
        inout item 'frame' : CanFrame;
    }

    abstract interface def EbikeChannel {
        doc /* Abstract classification base for every arrow type. */
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
        end source : TorqueSensorPort;
        end sink : ~TorqueSensorPort;
    }

    interface def SpeedLink :> EbikeChannel {
        end source : SpeedSensorPort;
        end sink : ~SpeedSensorPort;
    }

    interface def CanLink :> EbikeChannel {
        end a : CanBusPort;
        end b : CanBusPort;
    }
}

package EbikeR1Requirements {
    doc /* Layer 2: requirements with document provenance (EB-SPEC v2). */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'EB-01'> ContinuousMotorPowerLimit {
        doc /* EB-01: Continuous motor power shall not exceed 250 W. */
        @Rationale {
            text = "Source: EB-SPEC v2, §4.1";
        }
        attribute maxMotorPowerW : Real default 250.0 [W] {
            doc /* Spec LIMIT (EB-01). */
        }
        attribute measuredMotorPowerW : Real {
            doc /* [evidence] EVIDENCE: measured continuous motor power. No default. */
        }
        require constraint { measuredMotorPowerW <= maxMotorPowerW }
    }

    requirement def <'EB-02'> AssistCutoffSpeedLimit {
        doc /* EB-02: Motor assistance shall cut off above 6.94 m/s (25 km/h). */
        @Rationale {
            text = "Source: EB-SPEC v2, §4.2";
        }
        attribute maxCutoffSpeedMS : Real default 6.94 [m/s] {
            doc /* Spec LIMIT (EB-02). */
        }
        attribute measuredCutoffSpeedMS : Real {
            doc /* [evidence] EVIDENCE: measured speed at which assistance cuts off. No default. */
        }
        require constraint { measuredCutoffSpeedMS <= maxCutoffSpeedMS }
    }

    requirement def <'EB-03'> BatteryVoltageLimit {
        doc /* EB-03: Battery voltage shall stay at or below 42 V. */
        @Rationale {
            text = "Source: EB-SPEC v2, §5.3";
        }
        attribute maxBatteryVoltageV : Real default 42.0 [V] {
            doc /* Spec LIMIT (EB-03). */
        }
        attribute measuredBatteryVoltageV : Real {
            doc /* [evidence] EVIDENCE: measured battery voltage. No default. */
        }
        require constraint { measuredBatteryVoltageV <= maxBatteryVoltageV }
    }
}

package EbikeR1Subsystems {
    doc /* Layer 3: subsystem blueprints (boxes), actions, a calculation and the drive mode. */
    private import EbikeR1Types::*;
    private import ScalarValues::*;
    private import SI::*;

    action def ReadSensors {
        doc /* Function: read pedal torque and wheel speed. */
    }

    action def AssistControl {
        doc /* Function: compute and drive pedal assistance. */
    }

    calc def dcPowerW {
        doc /* Pure DC power relation P = V * I used for budgets. */
        in voltageV : Real;
        in currentA : Real;
        return result : Real = voltageV * currentA;
    }

    part def BatteryPack {
        doc /* 36 V battery pack, 14 Ah capacity (EB-SPEC v2). */
        attribute nominalVoltage : Real default 36.0 [V];
        attribute capacity : Real default 14.0 [A*h];
        port packOut : DcPowerPort;
        satisfy requirement : EbikeR1Requirements::BatteryVoltageLimit;
    }

    part def BatteryManagementSystem {
        doc /* Battery management system: supplies DC power and reports state of charge over CAN. */
        port packIn : ~DcPowerPort;
        port dcOut : DcPowerPort;
        port can : CanBusPort;
    }

    state def DriveMode {
        doc /* Operating modes of the drive. */
        ref part target : MotorController;
        entry;
        then off;
        state off;
        state assist;
        state noAssist;
        state fault;
        transition first off accept PowerOn then assist {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 names the Off and Assist modes but no switch-on event";
                resolveBy = "detailed";
            }
        }
        transition first assist accept payload : SpeedSignal if payload.speed > target.cutoffSpeed then noAssist;
        transition first assist accept FaultDetected then fault;
        transition first assist accept PowerOff then off {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 names no switch-off event";
                resolveBy = "detailed";
            }
        }
        transition first noAssist accept payload : SpeedSignal if payload.speed <= target.cutoffSpeed then assist;
        transition first noAssist accept FaultDetected then fault;
        transition first noAssist accept PowerOff then off {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 names no switch-off event";
                resolveBy = "detailed";
            }
        }
        transition first fault accept ResetFault then off {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 names no fault reset or recovery path";
                resolveBy = "detailed";
            }
        }
    }

    part def MotorController {
        doc /* Motor controller: reads pedal torque and wheel speed, drives the hub motor, reports assist level. */
        attribute cutoffSpeed : Real default 6.94 [m/s] {
            doc /* Assistance cutoff speed (EB-02). */
        }
        port dcIn : ~DcPowerPort;
        port torqueIn : ~TorqueSensorPort;
        port speedIn : ~SpeedSensorPort;
        port phaseOut : ThreePhasePort;
        port can : CanBusPort;
        perform action read : ReadSensors;
        perform action assist : AssistControl;
        satisfy requirement : EbikeR1Requirements::AssistCutoffSpeedLimit;
    }

    part def HubMotor {
        doc /* 250 W hub motor (EB-01). */
        attribute ratedPower : Real default 250.0 [W];
        port phaseIn : ~ThreePhasePort;
        satisfy requirement : EbikeR1Requirements::ContinuousMotorPowerLimit;
    }

    part def PedalTorqueSensor {
        doc /* Pedal torque sensor. */
        port torqueOut : TorqueSensorPort;
    }

    part def WheelSpeedSensor {
        doc /* Wheel speed sensor. */
        port speedOut : SpeedSensorPort;
    }

    part def HandlebarDisplay {
        doc /* Handlebar display: receives over the same CAN bus from the BMS and the motor controller. */
        port canBms : CanBusPort;
        port canMc : CanBusPort;
    }
}

package EbikeR1Context {
    doc /* Layer 4: the single top-level context (IBD) composing and wiring all subsystems. */
    private import EbikeR1Types::*;
    private import EbikeR1Subsystems::*;

    part def EbikeR1System {
        doc /* Top-level e-bike pedal-assist drive. */
        part battery : BatteryPack;
        part bms : BatteryManagementSystem;
        part motorController : MotorController;
        part hubMotor : HubMotor;
        part torqueSensor : PedalTorqueSensor;
        part wheelSpeed : WheelSpeedSensor;
        part display : HandlebarDisplay;
        connection packToBms connect battery.packOut to bms.packIn {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "EB-SPEC v2 does not describe the battery pack to BMS interface";
                resolveBy = "detailed";
            }
        }
        interface bmsToMotorController : DcPowerLink connect bms.dcOut to motorController.dcIn;
        interface pedalTorqueToMotorController : TorqueLink connect torqueSensor.torqueOut to motorController.torqueIn;
        interface wheelSpeedToMotorController : SpeedLink connect wheelSpeed.speedOut to motorController.speedIn;
        interface motorControllerToHubMotor : ThreePhaseLink connect motorController.phaseOut to hubMotor.phaseIn;
        interface canBmsToDisplay : CanLink connect bms.can to display.canBms;
        interface canMotorControllerToDisplay : CanLink connect motorController.can to display.canMc;
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
