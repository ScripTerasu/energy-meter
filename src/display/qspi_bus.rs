//! QSPI [`DisplayBus`] implementation for the CO5300 AMOLED panel.

use display_driver::DisplayError;
use display_driver::bus::{DisplayBus, ErrorType, Metadata};
use esp_hal::Async;
use esp_hal::spi::Error as SpiError;
use esp_hal::spi::master::{Address, Command, DataMode, SpiDmaBus};

use crate::board::DMA_BUFFER_SIZE;

/// CO5300 continuation opcode (0x32) + address byte 0x3C (CONTINUE_WRITE_RAM).
/// Used to keep streaming pixel data after the first WRITE_RAM (0x2C) chunk.
const QSPI_CONTINUE_ADDR: u32 = 0x00_3C_00;
const QSPI_WRITE_RAM_OPCODE: u8 = 0x32;

/// A [`DisplayBus`] backed by the ESP32-S3 QSPI peripheral.
///
/// The `QspiFlashBus` wrapper from `display-driver` turns every logical command
/// into a 4-byte header `[opcode, addr_hi, addr_mid, addr_lo]`, where `opcode`
/// is `0x02` for register writes and `0x32` for pixel (RAM) writes. We map that
/// header onto the ESP32 half-duplex QSPI command/address phases: the command
/// and 24-bit address always travel on a single line, while pixel data uses all
/// four data lines.
pub struct Co5300QspiBus {
    spi: SpiDmaBus<'static, Async>,
}

impl Co5300QspiBus {
    /// Wraps a configured DMA SPI bus for use with the CO5300 driver.
    pub fn new(spi: SpiDmaBus<'static, Async>) -> Self {
        Self { spi }
    }

    fn write_qspi(&mut self, header: &[u8], data: &[u8], quad_data: bool) -> Result<(), SpiError> {
        let opcode = header[0];
        let addr =
            (u32::from(header[1]) << 16) | (u32::from(header[2]) << 8) | u32::from(header[3]);
        let data_mode = if quad_data {
            DataMode::Quad
        } else {
            DataMode::Single
        };

        if data.is_empty() {
            return self.spi.half_duplex_write(
                data_mode,
                Command::_8Bit(u16::from(opcode), DataMode::Single),
                Address::_24Bit(addr, DataMode::Single),
                0,
                &[],
            );
        }

        let mut first = true;
        for chunk in data.chunks(DMA_BUFFER_SIZE) {
            let (op, ad) = if first || !quad_data {
                (opcode, addr)
            } else {
                // Continue the RAM write instead of restarting the window.
                (QSPI_WRITE_RAM_OPCODE, QSPI_CONTINUE_ADDR)
            };

            self.spi.half_duplex_write(
                data_mode,
                Command::_8Bit(u16::from(op), DataMode::Single),
                Address::_24Bit(ad, DataMode::Single),
                0,
                chunk,
            )?;
            first = false;
        }

        Ok(())
    }
}

impl ErrorType for Co5300QspiBus {
    type Error = SpiError;
}

impl DisplayBus for Co5300QspiBus {
    async fn write_cmd(&mut self, cmd: &[u8]) -> Result<(), Self::Error> {
        self.write_qspi(cmd, &[], false)
    }

    async fn write_cmd_with_params(
        &mut self,
        cmd: &[u8],
        params: &[u8],
    ) -> Result<(), Self::Error> {
        self.write_qspi(cmd, params, false)
    }

    async fn write_pixels(
        &mut self,
        cmd: &[u8],
        data: &[u8],
        _metadata: Metadata,
    ) -> Result<(), DisplayError<Self::Error>> {
        self.write_qspi(cmd, data, true)
            .map_err(DisplayError::BusError)
    }
}
