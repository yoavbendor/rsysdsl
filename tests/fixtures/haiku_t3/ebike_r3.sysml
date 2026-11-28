package EbikeR3Types {
    doc /* Layer 1: payloads, ports and interfaces. */
    private import ScalarValues::*;
    private import SI::*;

    item def CanFrame {
        doc /*
            CAN frame group exchanged between the BMS / motor controller and the handlebar display.
            EB-SPEC v2 names the CAN bus and the two reported quantities but gives neither units,
            ranges nor message layout, so both fields are TBDs.
        */
        attribute stateOfCharge : Real {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "EB-SPEC v2 gives no unit or range for battery state of charge";
                resolveBy = "detailed";
            }
        }
        attribute assistLevel : Real {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "EB-SPEC v2 gives no unit or assist level scale";
                resolveBy = "detailed";
            }
        }
    }

    item def PedalTorqueSignal {
        doc /* Pedal torque reading from the pedal torque sensor (EB-SPEC v2 names the signal, not its unit). */
        attribute torque : Real {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "EB-SPEC v2 gives no unit for pedal torque";
                resolveBy = "detailed";
            }
        }
    }

    item def WheelSpeedSignal {
        doc /*
            Wheel speed reading from the wheel speed sensor. Also the event that drives the assist-mode
            state machine, which compares it against the EB-02 cutoff speed.
        */
        attribute speed : Real;
    }

    item def PowerOnRequest {
        doc /* Trigger for leaving `Off` (power-on source not described in EB-SPEC v2). */
    }

    item def PowerOffRequest {
        doc /* Trigger for returning to `Off` (power-off source not described in EB-SPEC v2). */
    }

    item def FaultReport {
        doc /* Trigger for entering `Fault` (fault sources not described in EB-SPEC v2). */
    }

    port def CanBusPort {
        doc /* Bidirectional CAN bus port (both ends use this same port type). */
        inout item 'frame' : CanFrame;
    }

    port def PedalTorquePort {
        doc /* Directed output port of the pedal torque sensor. */
        out item torque : PedalTorqueSignal;
    }

    port def WheelSpeedPort {
        doc /* Directed output port of the wheel speed sensor. */
        out item speed : WheelSpeedSignal;
    }

    interface def CanLink {
        doc /* CAN bus link between two nodes. */
        end nodeA : CanBusPort;
        end nodeB : CanBusPort;
    }
}

package EbikeR3Requirements {
    doc /*
        Layer 2: requirements with EB-SPEC v2 provenance. Limits are the spec values; measured values
        are `evidence` with no default, so the model never claims a verification it does not have.
    */
    private import ModelingMetadata::*;
    private import ScalarValues::*;
    private import SI::*;

    requirement def <'EB-01'> EB01ContinuousMotorPower {
        doc /* EB-01: Continuous motor power shall not exceed 250 W. */
        @Rationale {
            text = "Source: EB-SPEC v2, §4.1";
        }
        attribute maxContinuousPower : Real default 250.0 [W] {
            doc /* Spec limit (EB-SPEC v2 §4.1). */
        }
        attribute measuredContinuousPower : Real {
            doc /* [evidence] Measured continuous motor power: no default, supplied by a verification case. */
        }
        require constraint { measuredContinuousPower <= maxContinuousPower }
    }

    requirement def <'EB-02'> EB02AssistCutoff {
        doc /*
            EB-02: Motor assistance shall cut off above 6.94 m/s (25 km/h).
            The measured cutoff threshold must not exceed the spec value.
        */
        @Rationale {
            text = "Source: EB-SPEC v2, §4.2";
        }
        attribute cutoffSpeed : Real default 6.94 [m/s] {
            doc /* Spec cutoff speed (EB-SPEC v2 §4.2). */
        }
        attribute measuredCutoffSpeed : Real {
            doc /* [evidence] Measured cutoff threshold: no default, supplied by a verification case. */
        }
        require constraint { measuredCutoffSpeed <= cutoffSpeed }
    }

    requirement def <'EB-03'> EB03BatteryVoltage {
        doc /* EB-03: Battery voltage shall stay at or below 42 V. */
        @Rationale {
            text = "Source: EB-SPEC v2, §5.3";
        }
        attribute maxBatteryVoltage : Real default 42.0 [V] {
            doc /* Spec limit (EB-SPEC v2 §5.3). */
        }
        attribute measuredBatteryVoltage : Real {
            doc /* [evidence] Measured battery voltage: no default, supplied by a verification case. */
        }
        require constraint { measuredBatteryVoltage <= maxBatteryVoltage }
    }
}

package EbikeR3Subsystems {
    doc /* Layer 3: part definitions (boxes) and the operating-mode state machine. */
    private import EbikeR3Types::*;
    private import EbikeR3Requirements::*;
    private import SysmlStdFlows::*;
    private import SysmlStdMechanics::*;
    private import ScalarValues::*;
    private import SI::*;

    part def BatteryPack {
        doc /*
            36 V, 14 Ah battery pack (EB-SPEC v2 system description). Realizes EB-03: the pack voltage
            is the quantity EB-03 limits.
        */
        attribute nominalVoltage : Real default 36.0 [V];
        attribute capacity : Real default 14.0 [A*h];
        port powerOut : ElectricalPowerPort;
        satisfy requirement : EB03BatteryVoltage;
    }

    part def BatteryManagementSystem {
        doc /*
            Battery management system: supplies DC power to the motor controller and reports state of
            charge to the display over CAN.
        */
        port powerIn : ~ElectricalPowerPort;
        port powerOut : ElectricalPowerPort;
        port can : CanBusPort;
    }

    part def MotorController {
        doc /*
            Motor controller: reads pedal torque and wheel speed, drives the hub motor with 3-phase
            power, reports assist level to the display over CAN. Realizes EB-02 (assist cutoff).
        */
        attribute cutoffSpeed : Real default 6.94 [m/s] {
            doc /* Assist cutoff speed (EB-02 value). */
        }
        attribute phaseCount : Real default 3.0 {
            doc /* 3-phase drive (EB-SPEC v2 system description). */
        }
        port dcIn : ~ElectricalPowerPort;
        port phaseOut : ElectricalPowerPort;
        port pedalIn : ~PedalTorquePort;
        port speedIn : ~WheelSpeedPort;
        port can : CanBusPort;
        satisfy requirement : EB02AssistCutoff;
    }

    part def HubMotor :> ElectricMotor {
        doc /*
            Hub motor: 250 W, specialized from the sysml-std electric motor. The library defaults for
            rotation frequency (50 Hz), efficiency (0.85), mass (1.0 kg), heat dissipation (0 W) and noise
            (30 dB) are placeholders, not EB-SPEC v2 values, so they are blanked as TBDs. Realizes EB-01
            (continuous power limit).
        */
        attribute :>> ratedPower default 250.0 [W];
        attribute :>> ratedRotationFreq {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EB-SPEC v2 gives no motor rotation frequency; library 50 Hz is a mains placeholder";
                resolveBy = "detailed";
            }
        }
        attribute :>> efficiency {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EB-SPEC v2 gives no motor efficiency; library 0.85 is a placeholder";
                resolveBy = "detailed";
            }
        }
        attribute :>> mass {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EB-SPEC v2 gives no hub motor mass; library 1.0 kg is a placeholder";
                resolveBy = "detailed";
            }
        }
        attribute :>> heatDissipation {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EB-SPEC v2 gives no hub motor heat dissipation; library 0.0 W is a placeholder";
                resolveBy = "detailed";
            }
        }
        attribute :>> noiseLevelDb {
            @TbdMetadata::TBD {
                category = "value";
                reason = "EB-SPEC v2 gives no hub motor noise level; library 30.0 dB is a placeholder";
                resolveBy = "detailed";
            }
        }
        port :>> powerIn;
        satisfy requirement : EB01ContinuousMotorPower;
    }

    part def PedalTorqueSensor {
        doc /* Pedal torque sensor: source of the pedal torque signal read by the motor controller. */
        port pedalOut : PedalTorquePort;
    }

    part def WheelSpeedSensor {
        doc /* Wheel speed sensor: source of the wheel speed signal read by the motor controller. */
        port speedOut : WheelSpeedPort;
    }

    part def HandlebarDisplay {
        doc /*
            Handlebar display: receives state of charge from the BMS and assist level from the motor
            controller, both over the same CAN bus.
        */
        port canBms : CanBusPort;
        port canMc : CanBusPort;
    }

    state def AssistMode {
        doc /*
            Operating modes of the drive (EB-SPEC v2 §Operating modes). `NoAssist` is the state above
            the EB-02 cutoff speed. Trigger sources for power on/off and faults are not described in
            the spec, so those transitions are TBDs. `Fault` is latched: recovery is not described.
        */
        ref part target : MotorController;
        entry;
        then off;
        state off {
            doc /* Start state. Power-on trigger is a TBD. */
        }
        state assist {
            doc /* Assistance active while wheel speed is at or below the cutoff. */
        }
        state noAssist {
            doc /* Assistance cut off above the EB-02 cutoff speed. */
        }
        state fault {
            doc /* Latched fault state: no exit is described in EB-SPEC v2. */
        }
        transition first off accept PowerOnRequest then assist {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 does not describe the power-on trigger";
                resolveBy = "detailed";
            }
        }
        transition first assist accept payload : WheelSpeedSignal if payload.speed > target.cutoffSpeed then noAssist;
        transition first assist accept PowerOffRequest then off {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 does not describe the power-off trigger";
                resolveBy = "detailed";
            }
        }
        transition first assist accept FaultReport then fault {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 does not describe the fault triggers";
                resolveBy = "detailed";
            }
        }
        transition first noAssist accept payload : WheelSpeedSignal if payload.speed <= target.cutoffSpeed then assist;
        transition first noAssist accept PowerOffRequest then off {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 does not describe the power-off trigger";
                resolveBy = "detailed";
            }
        }
        transition first noAssist accept FaultReport then fault {
            @TbdMetadata::TBD {
                category = "element";
                reason = "EB-SPEC v2 does not describe the fault triggers";
                resolveBy = "detailed";
            }
        }
    }
}

package EbikeR3Context {
    doc /* Layer 4: the single top-level context that composes and wires the subsystems. */
    private import EbikeR3Types::*;
    private import EbikeR3Subsystems::*;
    private import SysmlStdFlows::*;

    part def EbikeR3System {
        doc /* Top-level e-bike pedal-assist drive: all subsystems and their links. */
        part batteryPack : BatteryPack;
        part bms : BatteryManagementSystem;
        part motorController : MotorController;
        part hubMotor : HubMotor;
        part pedalSensor : PedalTorqueSensor;
        part wheelSensor : WheelSpeedSensor;
        part display : HandlebarDisplay;
        connection batteryToBms connect batteryPack.powerOut to bms.powerIn {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "EB-SPEC v2 names the battery pack and BMS but does not describe how they are wired";
                resolveBy = "detailed";
            }
        }
        interface bmsToMotorController : ElectricalLink connect bms.powerOut to motorController.dcIn;
        interface motorControllerToHubMotor : ElectricalLink connect motorController.phaseOut to hubMotor.powerIn;
        interface bmsToDisplayCan : CanLink connect bms.can to display.canBms;
        interface motorControllerToDisplayCan : CanLink connect motorController.can to display.canMc;
        connection pedalTorqueToMotorController connect pedalSensor.pedalOut to motorController.pedalIn {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "EB-SPEC v2 names the pedal torque signal but not its interface";
                resolveBy = "detailed";
            }
        }
        connection wheelSpeedToMotorController connect wheelSensor.speedOut to motorController.speedIn {
            @TbdMetadata::TBD {
                category = "channel";
                reason = "EB-SPEC v2 names the wheel speed signal but not its interface";
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
