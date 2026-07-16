//! Shared asynchronous I2C bus.
//!
//! The touch controller (CST9217) and the power-management IC (AXP2101) both
//! hang off the same I2C bus (SDA = GPIO15, SCL = GPIO14). We wrap the esp-hal
//! peripheral in an `embassy-sync` mutex so both drivers can share it safely.

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::Async;
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::peripherals::{GPIO14, GPIO15, I2C0};
use esp_hal::time::Rate;
use static_cell::StaticCell;

use crate::board::I2C_FREQ_KHZ;

/// The owned I2C bus, guarded by a mutex for shared access.
pub type I2cBus = Mutex<NoopRawMutex, I2c<'static, Async>>;

/// A cloneable handle to the shared bus, usable by one device driver.
pub type SharedI2c = I2cDevice<'static, NoopRawMutex, I2c<'static, Async>>;

/// Peripherals required to drive the shared I2C bus.
pub struct I2cPeripherals {
    pub i2c: I2C0<'static>,
    pub sda: GPIO15<'static>,
    pub scl: GPIO14<'static>,
}

/// Initializes the async I2C bus and returns a reference to the shared mutex.
///
/// Call [`SharedI2c::new`] (via [`device`]) once per device that lives on the
/// bus.
pub fn init(p: I2cPeripherals) -> &'static I2cBus {
    static BUS: StaticCell<I2cBus> = StaticCell::new();

    let i2c = I2c::new(
        p.i2c,
        Config::default().with_frequency(Rate::from_khz(I2C_FREQ_KHZ)),
    )
    .unwrap()
    .with_sda(p.sda)
    .with_scl(p.scl)
    .into_async();

    BUS.init(Mutex::new(i2c))
}

/// Creates a new device handle on the shared bus.
pub fn device(bus: &'static I2cBus) -> SharedI2c {
    I2cDevice::new(bus)
}
