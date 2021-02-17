//! A somewhat fancy remote controlled driving routine for the
//!   [ev3dev platform](https://www.ev3dev.org/).
//!
//! The "remote control" interface is a webserver hosted by the EV3 brick itself,
//!   which is implemented here, and will be available at
//!   <http://ev3dev.local:8080/>.
//!
//! The backend web server is implemented using the
//!   [Actix](https://actix.rs/) framework,
//!   which was selected mostly because I'm already familiar with it
//!   (and because it's fast).
//! The frontend is just the webpage provided by the `./static/index.html` file.
//!
//! Everything here is kept to a single file as much as possible,
//!   for simplicity's sake.

use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use ev3dev_lang_rust::motors::{LargeMotor, MotorPort};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::Mutex;

/// The main method for the application, which will be run when the application is launched.
/// It mostly just configures and runs the backend Actix webserver.
#[actix_web::main]
async fn main() -> Result<()> {
    // Initialize application data.
    let control_state = web::Data::new(Mutex::new(ControlState::new()));
    let ev3_devices_app = web::Data::new(Mutex::new(Ev3Devices::new()?));
    let ev3_devices_server = ev3_devices_app.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(ev3_devices_server.clone())
            .app_data(control_state.clone())
            .service(index)
            .service(set_mode)
            .service(set_speed)
            .service(set_direction)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
    .context("Actix server errored.")?;

    // Make sure motors get stopped on exit.
    let motor_set = &ev3_devices_app.lock().await.motor_set;
    motor_set.stop()?;
    motor_set.wait_until_not_moving(None);

    Ok(())
}

/// Provides the application's frontend, via the `./static/index.html`,
///   which will be embedded in the compiled binary for this application.
///
/// Accessible by browsing to <http://ev3dev.local:8080/> from
///   another device on the same network as the EV3.
#[get("/")]
async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/index.html"))
}

/// Models the JSON parameters for [set_mode()].
#[derive(Deserialize)]
struct ModeData {
    mode: Mode,
}

/// This API endpoint is called when the user clicks one of the "gear shift" buttons in the web application.
/// Switches/shifts the robot into stop, drive, or reverse.
///
/// Parameters:
/// * `control_state`: the [ControlState] instance managed/shared by the application
/// * `ev3_devices`: the [Ev3Devices] instance managed/shared by the application
/// * `mode_data`: the [ModeData] parameters specified in the API call
#[post("/mode")]
async fn set_mode(
    control_state: web::Data<Mutex<ControlState>>,
    ev3_devices: web::Data<Mutex<Ev3Devices>>,
    mode_data: web::Json<ModeData>,
) -> actix_web::Result<HttpResponse> {
    let mut control_state = control_state.lock().await;
    let ev3_devices = &ev3_devices.lock().await;
    let motor_set = &ev3_devices.motor_set;

    match &mode_data.mode {
        Mode::Stop => {
            motor_set.set_stop_action("brake")?;
            motor_set.stop()?;

            control_state.mode = Mode::Stop;
        }
        Mode::Forward => {
            // If switching directions, stop first.
            if control_state.mode == Mode::Backward {
                motor_set.set_stop_action("brake")?;
                motor_set.stop()?;
                motor_set.wait_until_not_moving(None);
            }

            control_state.mode = Mode::Forward;
            apply_control_state(&control_state, ev3_devices)?;
        }
        Mode::Backward => {
            // If switching directions, stop first.
            if control_state.mode == Mode::Forward {
                motor_set.set_stop_action("brake")?;
                motor_set.stop()?;
                motor_set.wait_until_not_moving(None);
            }

            control_state.mode = Mode::Backward;
            apply_control_state(&control_state, ev3_devices)?;
        }
    }

    Ok(HttpResponse::Ok().finish().into_body())
}

/// Models the JSON parameters for [set_speed()].
#[derive(Deserialize)]
struct SpeedData {
    speed: u8,
}

/// This API endpoint is called when the user adjusts the speed input control in the web application.
/// Speeds up or slows down the robot.
///
/// Parameters:
/// * `control_state`: the [ControlState] instance managed/shared by the application
/// * `ev3_devices`: the [Ev3Devices] instance managed/shared by the application
/// * `speed_data`: the [SpeedData] parameters specified in the API call
#[post("/speed")]
async fn set_speed(
    control_state: web::Data<Mutex<ControlState>>,
    ev3_devices: web::Data<Mutex<Ev3Devices>>,
    speed_data: web::Json<SpeedData>,
) -> actix_web::Result<HttpResponse> {
    let mut control_state = control_state.lock().await;
    let ev3_devices = &ev3_devices.lock().await;

    // Clamp the specified speed to the allowed/expected range.
    let mut speed = speed_data.speed;
    speed = 100.min(speed);
    speed = 0.max(speed);

    control_state.speed = speed;
    apply_control_state(&control_state, ev3_devices)?;

    Ok(HttpResponse::Ok().finish().into_body())
}

/// Models the JSON parameters for [set_direction()].
#[derive(Deserialize)]
struct DirectionData {
    direction: i8,
}

/// This API endpoint is called when the user adjusts the speed input control in the web application.
/// Adjusts the robot's vector to the left or right,
///   by slowing down one of the wheels relative to the other.
///
/// Parameters:
/// * `control_state`: the [ControlState] instance managed/shared by the application
/// * `ev3_devices`: the [Ev3Devices] instance managed/shared by the application
/// * `direction_data`: the [DirectionData] parameters specified in the API call
#[post("/direction")]
async fn set_direction(
    control_state: web::Data<Mutex<ControlState>>,
    ev3_devices: web::Data<Mutex<Ev3Devices>>,
    direction_data: web::Json<DirectionData>,
) -> actix_web::Result<HttpResponse> {
    let mut control_state = control_state.lock().await;
    let ev3_devices = &ev3_devices.lock().await;

    // Clamp the specified direction to the allowed/expected range.
    let mut direction = direction_data.direction;
    direction = 100.min(direction);
    direction = -100.max(direction);

    control_state.direction = direction;
    apply_control_state(&control_state, ev3_devices)?;

    Ok(HttpResponse::Ok().finish().into_body())
}

/// Updates the motor settings to match the specified [ControlState].
///
/// Parameters:
/// * `control_state`: the desired [ControlState]
/// * `ev3_devices`: the [Ev3Devices] to update
fn apply_control_state(
    control_state: &ControlState,
    ev3_devices: &Ev3Devices,
) -> std::result::Result<(), Ev3ErrorWrapper> {
    let motor_set = &ev3_devices.motor_set;

    // Pre-calculate all of the wheel speed components.
    let speed_multipler = match control_state.mode {
        Mode::Backward => -1.0,
        _ => 1.0,
    };
    let speed_max_absolute = 900.0;
    let speed_percent: f32 = 1.0f32.min((control_state.speed as f32) / 100.0f32);
    let direction_percents = if control_state.direction > 0 {
        let left_wheel_percent = 1.0;
        let right_wheel_percent =
            1.0f32.min((100.0f32 - (control_state.direction.abs() as f32)) / 100.0f32);
        vec![left_wheel_percent, right_wheel_percent]
    } else if control_state.direction < 0 {
        let left_wheel_percent =
            1.0f32.min((100.0f32 - (control_state.direction.abs() as f32)) / 100.0f32);
        let right_wheel_percent = 1.0;
        vec![left_wheel_percent, right_wheel_percent]
    } else {
        vec![1.0, 1.0]
    };

    // Finalize and apply the wheel speed calculations.
    for (motor, direction_percent) in motor_set.motors.iter().zip(direction_percents) {
        let speed_sp =
            (speed_multipler * speed_max_absolute * speed_percent * direction_percent) as i32;
        //println!(
        //    "speed_multipler: {}, speed_percent: {}, direction_percents: {:?}, speed_sp: {}",
        //    speed_multipler, speed_percent, direction_percents, speed_sp
        //);
        motor.set_speed_sp(speed_sp)?;
    }

    // Stop/start the motors.
    match control_state.mode {
        Mode::Stop => {
            motor_set.set_stop_action("brake")?;
            motor_set.stop()?;
            motor_set.wait_until_not_moving(None);
        }
        _ => {
            motor_set.run_forever()?;
        }
    }

    Ok(())
}

/// A local wrapper of [ev3dev_lang_rust::Ev3Error], which is required so that we can implement
/// Actix's [actix_web::error::ResponseError] for it.
#[derive(Debug)]
struct Ev3ErrorWrapper {
    cause: ev3dev_lang_rust::Ev3Error,
}

impl std::fmt::Display for Ev3ErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EV3 error: {:?}", &self.cause)
    }
}

impl From<ev3dev_lang_rust::Ev3Error> for Ev3ErrorWrapper {
    fn from(cause: ev3dev_lang_rust::Ev3Error) -> Self {
        Ev3ErrorWrapper { cause }
    }
}

impl std::error::Error for Ev3ErrorWrapper {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // TODO How are we _supposed_ to implement this for non-trait-implementing errors?
        None
    }
}

impl actix_web::error::ResponseError for Ev3ErrorWrapper {}

/// The EV3 devices that will be used and shared by the application..
struct Ev3Devices {
    motor_set: LargeMotorSet,
}

impl Ev3Devices {
    /// Constructs an [Ev3Devices] for the application to use.
    pub fn new() -> std::result::Result<Ev3Devices, Ev3ErrorWrapper> {
        Ok(Ev3Devices {
            motor_set: LargeMotorSet {
                motors: vec![
                    LargeMotor::get(MotorPort::OutB).map_err(|cause| Ev3ErrorWrapper { cause })?,
                    LargeMotor::get(MotorPort::OutC).map_err(|cause| Ev3ErrorWrapper { cause })?,
                ],
            },
        })
    }
}

/// Represents a set of [LargeMotor]s that ought to be managed in concert.
struct LargeMotorSet {
    motors: Vec<LargeMotor>,
}

impl LargeMotorSet {
    /// Proxies [LargeMotor::set_stop_action()].
    pub fn set_stop_action(&self, stop_action: &str) -> std::result::Result<(), Ev3ErrorWrapper> {
        for motor in &self.motors {
            motor
                .set_stop_action(stop_action)
                .map_err(|cause| Ev3ErrorWrapper { cause })?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::stop()].
    pub fn stop(&self) -> std::result::Result<(), Ev3ErrorWrapper> {
        for motor in &self.motors {
            motor.stop().map_err(|cause| Ev3ErrorWrapper { cause })?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::run_forever].
    pub fn run_forever(&self) -> std::result::Result<(), Ev3ErrorWrapper> {
        for motor in &self.motors {
            motor
                .run_forever()
                .map_err(|cause| Ev3ErrorWrapper { cause })?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::wait_until_not_moving()].
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

/// Models the state of the driving controls presented by the web application.
struct ControlState {
    mode: Mode,
    speed: u8,
    direction: i8,
}

impl ControlState {
    /// Constructs a [ControlState] for the application to use.
    pub fn new() -> ControlState {
        ControlState {
            mode: Mode::Stop,
            speed: 0,
            direction: 0,
        }
    }
}

/// Models the different driving modes/gears.
#[derive(Deserialize, Eq, PartialEq)]
enum Mode {
    Stop,
    Forward,
    Backward,
}
