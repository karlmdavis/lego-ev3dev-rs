extern crate ev3dev_lang_rust;

use std::time::Duration;

use rand::prelude::*;

use ev3dev_lang_rust::motors::{LargeMotor, MotorPort};
use ev3dev_lang_rust::sensors::{TouchSensor, UltrasonicSensor};
use ev3dev_lang_rust::{sound, Ev3Button, Ev3Result, Led};

const PROXIMITY_CM_THRESHOLD_SLOW: f32 = 40.0;
const PROXIMITY_CM_THRESHOLD_STOP: f32 = 15.0;

fn main() -> Ev3Result<()> {
    // Get motors and sensors.
    let motors = LargeMotorSet {
        motors: vec![
            LargeMotor::get(MotorPort::OutB)?,
            LargeMotor::get(MotorPort::OutC)?,
        ],
    };
    let ultrasonic_sensor = UltrasonicSensor::find()?;
    let touch_sensor = TouchSensor::find()?;
    let buttons = Ev3Button::new()?;

    println!(
        "Waiting for button push. Press backspace to exit or anything else to start auto-driving."
    );
    loop {
        buttons.process();
        let buttons_pressed = buttons.get_pressed_buttons();
        println!("Buttons pushed: {:?}", buttons_pressed);
        if buttons_pressed.contains("backspace") {
            println!("Backspace pressed. Bye!");
            break;
        } else if !buttons_pressed.is_empty() {
            match auto_drive(&motors, &ultrasonic_sensor, &touch_sensor, &buttons) {
                Err(err) => {
                    // If the driving errored out, make sure we try to stop the motors.
                    eprintln!("Driving error: {:?}", err);
                    stop(&motors)?;
                }
                _ => {
                    /*
                     * If the driving stopped without error, wait for a moment before accepting
                     * input again, to avoid jitter caused by long presses.
                     */
                    std::thread::sleep(Duration::from_millis(1000));
                }
            };
        } else {
            // Wait for a button press.
            std::thread::sleep(Duration::from_millis(1000));
        }
    }

    Ok(())
}

/// Runs an "auto pilot" Roomba-esque sequence until one of the brick's buttons is pushed.
fn auto_drive(
    motors: &LargeMotorSet,
    ultrasonic_sensor: &UltrasonicSensor,
    touch_sensor: &TouchSensor,
    buttons: &Ev3Button,
) -> Ev3Result<()> {
    println!("Auto drive: starting. Press any brick button to stop.");
    start_straight(motors)?;

    loop {
        let mut distance_cm = ultrasonic_sensor.get_distance_centimeters()?;

        while touch_sensor.get_pressed_state()? || distance_cm < PROXIMITY_CM_THRESHOLD_STOP {
            change_direction(motors)?;
            distance_cm = ultrasonic_sensor.get_distance_centimeters()?;
        }

        /*
         * Our target speed is calculated as whatever percentage we are between the two
         * thresholds.
         */
        let duty_cycle_percentage = (distance_cm.min(PROXIMITY_CM_THRESHOLD_SLOW)
            - PROXIMITY_CM_THRESHOLD_STOP)
            / (PROXIMITY_CM_THRESHOLD_SLOW - PROXIMITY_CM_THRESHOLD_STOP);
        let duty_cycle = (100.0 * duty_cycle_percentage) as i32;
        motors.set_duty_cycle_sp(duty_cycle)?;

        // Wait for a bit before looping again.
        std::thread::sleep(Duration::from_millis(1000));

        buttons.process();
        if !buttons.get_pressed_buttons().is_empty() {
            println!("Auto drive: request to exit received.");
            break;
        }
    }

    stop(motors)?;

    Ok(())
}

fn stop(motors: &LargeMotorSet) -> Ev3Result<()> {
    println!("Auto drive: stopping.");
    motors.set_stop_action("brake")?;
    motors.stop()?;
    println!("Auto drive: brake commands issued.");
    motors.wait_until_not_moving(None);
    println!("Auto drive: stopped.");

    Ok(())
}

fn start_straight(motors: &LargeMotorSet) -> Ev3Result<()> {
    motors.set_duty_cycle_sp(100)?;
    motors.run_direct()?;

    Ok(())
}

fn change_direction(motors: &LargeMotorSet) -> Ev3Result<()> {
    backup(motors)?;
    turn_random(motors)?;
    start_straight(motors)?;

    Ok(())
}

fn backup(motors: &LargeMotorSet) -> Ev3Result<()> {
    stop(motors)?;

    let leds = Led::new()?;

    // Play fun backing-up sound and turn on backing-up lights.
    sound::tone_sequence(&[(1000.0, 500, 500), (1000.0, 500, 500), (1000.0, 500, 500)])?.wait()?;
    leds.set_left_color(Led::COLOR_RED)?;
    leds.set_right_color(Led::COLOR_RED)?;

    motors.set_speed_sp(-500)?;
    motors.run_timed(Some(Duration::from_millis(1500)))?;
    motors.wait_until(LargeMotor::STATE_RUNNING, None);
    motors.wait_until_not_moving(None);

    // Turn off backing-up lights.
    leds.set_left_color(Led::COLOR_GREEN)?;
    leds.set_right_color(Led::COLOR_GREEN)?;

    Ok(())
}

fn turn_random(motors: &LargeMotorSet) -> Ev3Result<()> {
    // Flip a coin for left or right turn.
    let direction = if rand::random() {
        vec![-1, 1]
    } else {
        vec![1, -1]
    };

    // Randomly decide how many millis to backup for.
    let backup_time = Duration::from_millis(rand::thread_rng().gen_range(250..=750));

    // Run the random turn.
    for (motor, direction) in motors.motors.iter().zip(direction) {
        // Set this wheel to run at 750, either forwards or backwards.
        motor.set_speed_sp(750 * direction)?;
    }
    motors.run_timed(Some(backup_time))?;
    motors.wait_until(LargeMotor::STATE_RUNNING, None);
    motors.wait_until_not_moving(None);

    Ok(())
}

/// Represents a set of [LargeMotors] that ought to be managed in concert.
struct LargeMotorSet {
    motors: Vec<LargeMotor>,
}

impl LargeMotorSet {
    /// Proxies [LargeMotor::set_stop_action].
    pub fn set_stop_action(&self, stop_action: &str) -> Ev3Result<()> {
        for motor in &self.motors {
            motor.set_stop_action(stop_action)?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::stop].
    pub fn stop(&self) -> Ev3Result<()> {
        for motor in &self.motors {
            motor.stop()?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::set_duty_cycle_sp].
    pub fn set_duty_cycle_sp(&self, duty_cycle: i32) -> Ev3Result<()> {
        for motor in &self.motors {
            motor.set_duty_cycle_sp(duty_cycle)?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::run_direct].
    pub fn run_direct(&self) -> Ev3Result<()> {
        for motor in &self.motors {
            motor.run_direct()?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::set_speed_sp].
    pub fn set_speed_sp(&self, speed_sp: i32) -> Ev3Result<()> {
        for motor in &self.motors {
            motor.set_speed_sp(speed_sp)?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::run_timed].
    pub fn run_timed(&self, time_sp: Option<Duration>) -> Ev3Result<()> {
        for motor in &self.motors {
            motor.run_timed(time_sp)?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::wait_until].
    pub fn wait_until(&self, state: &str, timeout: Option<Duration>) -> bool {
        let mut result = true;
        for motor in &self.motors {
            result = match motor.wait_until(state, timeout) {
                true => result,
                false => false,
            };
        }

        result
    }

    /// Proxies [LargeMotor::wait_until_not_moving].
    pub fn wait_until_not_moving(&self, timeout: Option<Duration>) -> bool {
        let mut result = true;
        for motor in &self.motors {
            result = match motor.wait_until_not_moving(timeout) {
                true => result,
                false => false,
            };
        }

        result
    }
}
