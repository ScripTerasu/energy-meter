//! AXP2101 power-management IC: PWR button detection.
//!
//! On this board the PWR side button is wired to the AXP2101's power key (PWRON)
//! rather than to an ESP32 GPIO. The PMU latches key events in its interrupt
//! status registers, which we poll over the shared I2C bus (see [`crate::i2c`]).
//!
//! Register and bit layout follow the AXP2101 datasheet / `XPowersLib`.

use embedded_hal_async::i2c::I2c;

use crate::board::PMU_I2C_ADDR;
use crate::i2c::SharedI2c;

/// Chip ID register; reads `0x4A` on a genuine AXP2101.
const REG_IC_TYPE: u8 = 0x03;
/// Expected value of [`REG_IC_TYPE`].
const CHIP_ID: u8 = 0x4A;

/// Interrupt-enable register bank (INTEN1..3 at 0x40..0x42).
const REG_INTEN1: u8 = 0x40;
/// Interrupt-status register bank (INTSTS1..3 at 0x48..0x4A).
const REG_INTSTS1: u8 = 0x48;
/// Number of interrupt registers in each bank.
const INT_REG_COUNT: usize = 3;

/// PWRON short-press flag, in INTSTS2 (`_BV(11) >> 8`).
const PKEY_SHORT_MASK: u8 = 1 << 3;
/// PWRON long-press flag, in INTSTS2 (`_BV(10) >> 8`).
const PKEY_LONG_MASK: u8 = 1 << 2;

/// A latched power-key event.
#[derive(Copy, Clone, Debug, defmt::Format, PartialEq, Eq)]
pub enum PowerKeyEvent {
    /// The PWR button was tapped (short press).
    ShortPress,
    /// The PWR button was held (long press).
    LongPress,
}

/// Driver for the AXP2101 power-management IC.
pub struct Axp2101 {
    i2c: SharedI2c,
}

impl Axp2101 {
    /// Probes the PMU and enables the power-key interrupts.
    ///
    /// Returns `None` if the chip does not identify as an AXP2101.
    pub async fn init(mut i2c: SharedI2c) -> Option<Self> {
        let mut id = [0u8; 1];
        i2c.write_read(PMU_I2C_ADDR, &[REG_IC_TYPE], &mut id)
            .await
            .ok()?;
        if id[0] != CHIP_ID {
            return None;
        }

        let mut pmu = Self { i2c };

        // Enable only the PWRON short/long-press interrupts (INTEN2 bank), then
        // clear any pending flags so we start from a clean slate.
        pmu.write_reg(REG_INTEN1 + 1, PKEY_SHORT_MASK | PKEY_LONG_MASK)
            .await;
        pmu.clear_irq().await;

        Some(pmu)
    }

    /// Reads and clears the latched power-key event, if any.
    pub async fn poll_key_event(&mut self) -> Option<PowerKeyEvent> {
        let mut status = [0u8; INT_REG_COUNT];
        self.i2c
            .write_read(PMU_I2C_ADDR, &[REG_INTSTS1], &mut status)
            .await
            .ok()?;

        // Power-key flags live in the second status register (INTSTS2).
        let key = status[1];
        let event = if key & PKEY_SHORT_MASK != 0 {
            Some(PowerKeyEvent::ShortPress)
        } else if key & PKEY_LONG_MASK != 0 {
            Some(PowerKeyEvent::LongPress)
        } else {
            None
        };

        if event.is_some() {
            self.clear_irq().await;
        }
        event
    }

    /// Clears every interrupt-status flag by writing all ones.
    async fn clear_irq(&mut self) {
        for i in 0..INT_REG_COUNT as u8 {
            self.write_reg(REG_INTSTS1 + i, 0xFF).await;
        }
    }

    async fn write_reg(&mut self, reg: u8, value: u8) {
        let _ = self.i2c.write(PMU_I2C_ADDR, &[reg, value]).await;
    }
}
