//! A simple remote controlled driving routine for the
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

use actix_web::{get, web, App, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use ev3dev_lang_rust::motors::{LargeMotor, MotorPort};
use std::time::Duration;
use tokio::sync::Mutex;

/// The main method for the application, which will be run when the application is launched.
/// It mostly just configures and runs the backend Actix webserver.
#[actix_web::main]
async fn main() -> Result<()> {
    // Ev3 devices
    let ev3_devices_app = web::Data::new(Mutex::new(Ev3Devices::new()?));
    let ev3_devices_server = ev3_devices_app.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(ev3_devices_server.clone())
            .service(index)
            .service(move_forward)
            .service(move_backward)
            .service(turn_left)
            .service(turn_right)
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

/// This API endpoint is called when the user clicks the "go forward" button in the web application.
/// Drives the robot straight ahead for a brief bit.
///
/// Parameters:
/// * `ev3_devices`: the [Ev3Devices] instance managed/shared by the application
#[get("/move/forward")]
async fn move_forward(
    ev3_devices: web::Data<Mutex<Ev3Devices>>,
) -> actix_web::Result<HttpResponse> {
    let motor_set = &ev3_devices.lock().await.motor_set;

    // Drive forward a bit.
    motor_set.set_duty_cycle_sp(100)?;
    motor_set.run_direct()?;
    motor_set.wait_until(LargeMotor::STATE_RUNNING, None);
    tokio::time::delay_for(Duration::from_millis(1000)).await;
    motor_set.set_stop_action("coast")?;
    motor_set.stop()?;

    // Send the client back to the home page.
    Ok(HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, "/")
        .finish()
        .into_body())
}

/// This API endpoint is called when the user clicks the "go backward" button in the web application.
/// Drives the robot straight back for a brief bit.
///
/// Parameters:
/// * `ev3_devices`: the [Ev3Devices] instance managed/shared by the application
#[get("/move/backward")]
async fn move_backward(
    ev3_devices: web::Data<Mutex<Ev3Devices>>,
) -> actix_web::Result<HttpResponse> {
    let motor_set = &ev3_devices.lock().await.motor_set;

    // Drive forward a bit.
    motor_set.set_duty_cycle_sp(-100)?;
    motor_set.run_direct()?;
    motor_set.wait_until(LargeMotor::STATE_RUNNING, None);
    tokio::time::delay_for(Duration::from_millis(1000)).await;
    motor_set.set_stop_action("coast")?;
    motor_set.stop()?;

    // Send the client back to the home page.
    Ok(HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, "/")
        .finish()
        .into_body())
}

/// This API endpoint is called when the user clicks the "turn left" button in the web application.
/// Turns the robot backwards and to the left a bit.
///
/// Parameters:
/// * `ev3_devices`: the [Ev3Devices] instance managed/shared by the application
#[get("/turn/left")]
async fn turn_left(ev3_devices: web::Data<Mutex<Ev3Devices>>) -> actix_web::Result<HttpResponse> {
    let motor_set = &ev3_devices.lock().await.motor_set;

    // Set the direction and time for the turn.
    let direction = vec![-1, 1];
    let backup_time = Duration::from_millis(150);

    // Run the turn.
    for (motor, direction) in motor_set.motors.iter().zip(direction) {
        // Set this wheel to run at 750, either forwards or backwards.
        motor
            .set_speed_sp(750 * direction)
            .map_err(|cause| Ev3ErrorWrapper { cause })?;
    }
    motor_set.run_timed(Some(backup_time))?;
    motor_set.wait_until(LargeMotor::STATE_RUNNING, None);
    motor_set.wait_until_not_moving(None);

    // Send the client back to the home page.
    Ok(HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, "/")
        .finish()
        .into_body())
}

/// This API endpoint is called when the user clicks the "turn right" button in the web application.
/// Turns the robot backwards and to the right a bit.
///
/// Parameters:
/// * `ev3_devices`: the [Ev3Devices] instance managed/shared by the application
#[get("/turn/right")]
async fn turn_right(ev3_devices: web::Data<Mutex<Ev3Devices>>) -> actix_web::Result<HttpResponse> {
    let motor_set = &ev3_devices.lock().await.motor_set;

    // Set the direction and time for the turn.
    let direction = vec![1, -1];
    let backup_time = Duration::from_millis(150);

    // Run the turn.
    for (motor, direction) in motor_set.motors.iter().zip(direction) {
        // Set this wheel to run at 750, either forwards or backwards.
        motor
            .set_speed_sp(750 * direction)
            .map_err(|cause| Ev3ErrorWrapper { cause })?;
    }
    motor_set.run_timed(Some(backup_time))?;
    motor_set.wait_until(LargeMotor::STATE_RUNNING, None);
    motor_set.wait_until_not_moving(None);

    // Send the client back to the home page.
    Ok(HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, "/")
        .finish()
        .into_body())
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

    /// Proxies [LargeMotor::set_duty_cycle_sp()].
    pub fn set_duty_cycle_sp(&self, duty_cycle: i32) -> std::result::Result<(), Ev3ErrorWrapper> {
        for motor in &self.motors {
            motor
                .set_duty_cycle_sp(duty_cycle)
                .map_err(|cause| Ev3ErrorWrapper { cause })?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::run_direct()].
    pub fn run_direct(&self) -> std::result::Result<(), Ev3ErrorWrapper> {
        for motor in &self.motors {
            motor
                .run_direct()
                .map_err(|cause| Ev3ErrorWrapper { cause })?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::run_timed()].
    pub fn run_timed(&self, time_sp: Option<Duration>) -> std::result::Result<(), Ev3ErrorWrapper> {
        for motor in &self.motors {
            motor
                .run_timed(time_sp)
                .map_err(|cause| Ev3ErrorWrapper { cause })?;
        }

        Ok(())
    }

    /// Proxies [LargeMotor::wait_until()].
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
