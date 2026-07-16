//! CST9217 capacitive touch controller driver (I2C).
//!
//! The protocol follows the vendor `TouchDrvCST92xx` driver: a fixed read
//! command (`0xD000`) returns a status/coordinate buffer that must then be
//! acknowledged by writing the same register followed by an ACK byte.
//!
//! Pin mapping comes from the Waveshare `pin_config.h`:
//! - SDA = GPIO15, SCL = GPIO14
//! - TP_INT (IRQ, active low) = GPIO11
//! - TP_RESET = GPIO40 (independent from the LCD reset on GPIO39)

use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::peripherals::{GPIO11, GPIO14, GPIO15, GPIO40, I2C0};
use esp_hal::time::Rate;

use crate::board::{LCD_HEIGHT, LCD_WIDTH, TOUCH_I2C_ADDR, TOUCH_I2C_FREQ_KHZ};

/// Read-report register of the CST9217 (big-endian).
const READ_COMMAND: [u8; 2] = [0xD0, 0x00];
/// Acknowledge byte the host must echo back after reading a report.
const ACK: u8 = 0xAB;
/// Maximum number of simultaneous fingers reported by the controller.
const MAX_FINGERS: usize = 2;
/// Length of a single report: `MAX_FINGERS * 5 + 5`.
const REPORT_LEN: usize = MAX_FINGERS * 5 + 5;
/// Finger event marking a valid "pressed" contact.
const EVENT_PRESSED: u8 = 0x06;

/// A single touch coordinate, in panel pixels.
#[derive(Copy, Clone, Debug, defmt::Format)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
}

/// Peripherals owned by the CST9217 touch controller.
///
/// Grouping them mirrors [`crate::display::DisplayPeripherals`] and documents
/// exactly which pins the touch driver takes.
pub struct TouchPeripherals {
    pub i2c: I2C0<'static>,
    pub sda: GPIO15<'static>,
    pub scl: GPIO14<'static>,
    pub int: GPIO11<'static>,
    pub reset: GPIO40<'static>,
}

/// Driver for the CST9217 capacitive touch controller.
pub struct Cst9217 {
    i2c: I2c<'static, Async>,
    int: Input<'static>,
    reset: Output<'static>,
}

impl Cst9217 {
    /// Initializes the I2C bus, resets the controller and returns a ready
    /// driver.
    pub async fn init(p: TouchPeripherals) -> Self {
        let i2c = I2c::new(
            p.i2c,
            Config::default().with_frequency(Rate::from_khz(TOUCH_I2C_FREQ_KHZ)),
        )
        .unwrap()
        .with_sda(p.sda)
        .with_scl(p.scl)
        .into_async();

        let int = Input::new(p.int, InputConfig::default().with_pull(Pull::Up));
        let reset = Output::new(p.reset, Level::High, OutputConfig::default());

        let mut touch = Self { i2c, int, reset };
        touch.reset().await;
        touch
    }

    /// Hardware-resets the controller (RST low ≥10 ms, then high) and waits for
    /// the firmware to boot.
    pub async fn reset(&mut self) {
        self.reset.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.reset.set_high();
        Timer::after(Duration::from_millis(50)).await;
    }

    /// Returns `true` while the IRQ line signals a pending report (active low).
    pub fn is_touched(&self) -> bool {
        self.int.is_low()
    }

    /// Waits until the controller asserts its IRQ line (a new touch report).
    pub async fn wait_for_touch(&mut self) {
        self.int.wait_for_falling_edge().await;
    }

    /// Reads the current touch report and returns the first pressed finger, if
    /// any.
    ///
    /// Returns `None` when there is no valid contact or on a bus error.
    pub async fn read(&mut self) -> Option<TouchPoint> {
        let mut report = [0u8; REPORT_LEN];
        self.i2c
            .write_read_async(TOUCH_I2C_ADDR, &READ_COMMAND, &mut report)
            .await
            .ok()?;

        // Acknowledge the report so the controller can prepare the next one.
        let ack = [READ_COMMAND[0], READ_COMMAND[1], ACK];
        self.i2c.write_async(TOUCH_I2C_ADDR, &ack).await.ok()?;

        // Device ACK: byte 6 must echo the ACK value for a valid frame.
        if report[6] != ACK {
            return None;
        }

        // Number of reported fingers (low 7 bits of byte 5).
        let num_points = (report[5] & 0x7F) as usize;
        if num_points == 0 || num_points > MAX_FINGERS {
            return None;
        }

        parse_finger(&report[0..4])
    }
}

/// Parses one finger record (`data[0..4]`) into a [`TouchPoint`].
fn parse_finger(data: &[u8]) -> Option<TouchPoint> {
    // Low nibble of byte 0 is the event; only 0x06 counts as pressed.
    if data[0] & 0x0F != EVENT_PRESSED {
        return None;
    }

    let raw_x = (u16::from(data[1]) << 4) | (u16::from(data[3]) >> 4);
    let raw_y = (u16::from(data[2]) << 4) | (u16::from(data[3]) & 0x0F);

    // The panel is mounted mirrored on both axes (vendor BSP: mirror_x/y = 1),
    // so flip the raw controller coordinates to match the display space.
    let x = (LCD_WIDTH - 1).saturating_sub(raw_x.min(LCD_WIDTH - 1));
    let y = (LCD_HEIGHT - 1).saturating_sub(raw_y.min(LCD_HEIGHT - 1));
    Some(TouchPoint { x, y })
}
