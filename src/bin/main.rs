#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use display_driver::bus::{DisplayBus, ErrorType, Metadata, QspiFlashBus};
use display_driver::{
    Area, ColorFormat, DisplayDriver, DisplayError, FrameControl, LCDResetOption,
};
use display_driver_co5300::{Co5300, spec::AM151Q466466LK_151_C};
use embassy_executor::Spawner;
use embassy_time::Delay;
use esp_backtrace as _;
use esp_hal::Async;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::dma_buffers;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::spi::master::{Address, Command, Config, DataMode, Spi, SpiDmaBus};
use esp_hal::spi::{Error as SpiError, Mode};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

/// CO5300 continuation opcode (0x32) + address byte 0x3C (CONTINUE_WRITE_RAM).
/// Used to keep streaming pixel data after the first WRITE_RAM (0x2C) chunk.
const QSPI_CONTINUE_ADDR: u32 = 0x00_3C_00;
const QSPI_WRITE_RAM_OPCODE: u8 = 0x32;

/// Size of the DMA transfer buffer (also the max size of a single QSPI chunk).
const DMA_BUFFER_SIZE: usize = 8192;

/// A `DisplayBus` implementation for the CO5300 AMOLED panel over the ESP32-S3
/// QSPI peripheral.
///
/// The `QspiFlashBus` wrapper from `display-driver` turns every logical command
/// into a 4-byte header `[opcode, addr_hi, addr_mid, addr_lo]`, where `opcode`
/// is `0x02` for register writes and `0x32` for pixel (RAM) writes. We map that
/// header onto the ESP32 half-duplex QSPI command/address phases: the command
/// and 24-bit address always travel on a single line, while pixel data uses all
/// four data lines.
struct Co5300QspiBus {
    spi: SpiDmaBus<'static, Async>,
}

impl Co5300QspiBus {
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
            let (op, ad) = if first {
                (opcode, addr)
            } else if quad_data {
                // Continue the RAM write instead of restarting the window.
                (QSPI_WRITE_RAM_OPCODE, QSPI_CONTINUE_ADDR)
            } else {
                (opcode, addr)
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

type DisplayPanel = Co5300<AM151Q466466LK_151_C, Output<'static>, QspiFlashBus<Co5300QspiBus>>;
type Display = DisplayDriver<QspiFlashBus<Co5300QspiBus>, DisplayPanel>;

const LCD_WIDTH: u16 = 466;
const LCD_HEIGHT: u16 = 466;
// Fill the screen two rows at a time (CO5300 requires 2-row alignment).
const STRIPE_LINES: u16 = 2;
const STRIPE_BYTES: usize = LCD_WIDTH as usize * STRIPE_LINES as usize * 2;

#[allow(
    clippy::large_stack_frames,
    reason = "small RGB565 stripe buffer used to repaint the AMOLED panel"
)]
async fn fill_screen(display: &mut Display, color: u16) {
    let hi = (color >> 8) as u8;
    let lo = (color & 0xFF) as u8;

    let mut stripe = [0u8; STRIPE_BYTES];
    for pixel in stripe.chunks_exact_mut(2) {
        pixel[0] = hi;
        pixel[1] = lo;
    }

    let mut y = 0;
    while y < LCD_HEIGHT {
        let area = Area::new(0, y, LCD_WIDTH, STRIPE_LINES);
        if display
            .write_pixels(area, FrameControl::new_standalone(), &stripe)
            .await
            .is_err()
        {
            info!("Fallo al escribir en el display");
            return;
        }
        y += STRIPE_LINES;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o unstable-hal -o embassy -o defmt -o esp-backtrace -o vscode -o zed

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let mut boot_button = Input::new(
        peripherals.GPIO0,
        InputConfig::default().with_pull(Pull::Up),
    );

    // --- CO5300 AMOLED display (QSPI) ---------------------------------------
    // Pines de la placa Waveshare ESP32-S3-Touch-AMOLED-1.75:
    //   SCLK = GPIO38, CS = GPIO12, D0..D3 = GPIO4/5/6/7, RESET = GPIO39.
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(1, DMA_BUFFER_SIZE);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO38)
    .with_cs(peripherals.GPIO12)
    .with_sio0(peripherals.GPIO4)
    .with_sio1(peripherals.GPIO5)
    .with_sio2(peripherals.GPIO6)
    .with_sio3(peripherals.GPIO7)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    let bus = QspiFlashBus::new(Co5300QspiBus { spi });

    let reset = Output::new(peripherals.GPIO39, Level::High, OutputConfig::default());
    let panel = Co5300::<AM151Q466466LK_151_C, _, _>::new(LCDResetOption::new_pin(reset));

    let mut delay = Delay;
    let mut display = DisplayDriver::builder(bus, panel)
        .with_color_format(ColorFormat::RGB565)
        .init(&mut delay)
        .await
        .unwrap();

    // Brillo al máximo y una primera pantalla en rojo para comprobar que funciona.
    display.set_brightness(0xFF).await.unwrap();
    fill_screen(&mut display, 0xF800).await;

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    // Colores RGB565 que rotaremos con cada pulsación del BOOT button.
    let colors = [0xF800u16, 0x07E0, 0x001F, 0xFFFF, 0x0000];
    let mut color_index = 0usize;

    loop {
        // Espera hasta que el BOOT button se presione
        boot_button.wait_for_falling_edge().await;
        info!("BOOT button pressed");

        color_index = (color_index + 1) % colors.len();
        fill_screen(&mut display, colors[color_index]).await;

        // Espera hasta que se suelte el BOOT button
        boot_button.wait_for_rising_edge().await;
        info!("BOOT button released");
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
