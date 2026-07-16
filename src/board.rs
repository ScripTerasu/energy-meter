//! Board-specific constants for the Waveshare ESP32-S3-Touch-AMOLED-1.75.
//!
//! Pin assignments come from the vendor `pin_config.h`. The panel is a CO5300
//! AMOLED driven over QSPI.

/// Visible width of the AMOLED panel, in pixels.
pub const LCD_WIDTH: u16 = 466;
/// Visible height of the AMOLED panel, in pixels.
pub const LCD_HEIGHT: u16 = 466;

/// Size of a full RGB565 framebuffer, in bytes.
pub const FB_BYTES: usize = LCD_WIDTH as usize * LCD_HEIGHT as usize * 2;

/// QSPI bus clock, in MHz.
pub const LCD_SPI_FREQ_MHZ: u32 = 40;

/// Size of the DMA transfer buffer, which also caps a single QSPI chunk.
pub const DMA_BUFFER_SIZE: usize = 8192;

/// Shared I2C bus clock, in kHz (touch controller + power-management IC).
pub const I2C_FREQ_KHZ: u32 = 400;

/// 7-bit I2C address of the CST9217 capacitive touch controller.
pub const TOUCH_I2C_ADDR: u8 = 0x5A;

/// 7-bit I2C address of the AXP2101 power-management IC.
///
/// The board's PWR side button is wired to this chip's power key, not to a
/// GPIO, so we detect presses by polling the PMU's interrupt registers.
pub const PMU_I2C_ADDR: u8 = 0x34;
