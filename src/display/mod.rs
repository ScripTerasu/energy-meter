//! CO5300 AMOLED display: QSPI bus, PSRAM framebuffer and bring-up.

mod framebuffer;
mod qspi_bus;

pub use framebuffer::FrameBuf;
pub use qspi_bus::Co5300QspiBus;

use display_driver::bus::QspiFlashBus;
use display_driver::{ColorFormat, DisplayDriver, LCDResetOption};
use display_driver_co5300::{Co5300, spec::AM151Q466466LK_151_C};
use embassy_time::Delay;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::dma_buffers;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::peripherals::{
    DMA_CH0, GPIO4, GPIO5, GPIO6, GPIO7, GPIO12, GPIO38, GPIO39, PSRAM, SPI2,
};
use esp_hal::psram::{Psram, PsramConfig};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;

use crate::board::{DMA_BUFFER_SIZE, FB_BYTES, LCD_SPI_FREQ_MHZ};

/// Concrete panel type for this board.
pub type DisplayPanel = Co5300<AM151Q466466LK_151_C, Output<'static>, QspiFlashBus<Co5300QspiBus>>;
/// Fully assembled display driver type.
pub type Display = DisplayDriver<QspiFlashBus<Co5300QspiBus>, DisplayPanel>;

/// Peripherals required to bring up the AMOLED display.
///
/// Grouping them keeps [`init`]'s signature readable and documents exactly
/// which pins the display owns.
pub struct DisplayPeripherals {
    pub psram: PSRAM<'static>,
    pub spi: SPI2<'static>,
    pub dma: DMA_CH0<'static>,
    pub sclk: GPIO38<'static>,
    pub cs: GPIO12<'static>,
    pub d0: GPIO4<'static>,
    pub d1: GPIO5<'static>,
    pub d2: GPIO6<'static>,
    pub d3: GPIO7<'static>,
    pub reset: GPIO39<'static>,
}

/// Initializes PSRAM, the QSPI bus and the CO5300 panel.
///
/// Returns the ready-to-use [`Display`] together with a [`FrameBuf`] backed by
/// PSRAM. Panics if PSRAM cannot host a full framebuffer or if bring-up fails.
pub async fn init(p: DisplayPeripherals) -> (Display, FrameBuf) {
    // --- PSRAM framebuffer --------------------------------------------------
    // The 466x466 RGB565 framebuffer (~434 KB) does not fit in internal SRAM,
    // so we place it in the 8 MB PSRAM.
    let psram = Psram::new(p.psram, PsramConfig::default());
    let (psram_ptr, psram_len) = psram.raw_parts();
    assert!(psram_len >= FB_BYTES, "PSRAM too small for the framebuffer");
    // SAFETY: the PSRAM range is mapped by `Psram::new`, is large enough for
    // `FB_BYTES`, and this slice is the only handle to that region.
    let fb_slice = unsafe { core::slice::from_raw_parts_mut(psram_ptr, FB_BYTES) };
    let framebuffer = FrameBuf::new(fb_slice);

    // --- QSPI bus -----------------------------------------------------------
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(1, DMA_BUFFER_SIZE);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let spi = Spi::new(
        p.spi,
        Config::default()
            .with_frequency(Rate::from_mhz(LCD_SPI_FREQ_MHZ))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(p.sclk)
    .with_cs(p.cs)
    .with_sio0(p.d0)
    .with_sio1(p.d1)
    .with_sio2(p.d2)
    .with_sio3(p.d3)
    .with_dma(p.dma)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    let bus = QspiFlashBus::new(Co5300QspiBus::new(spi));

    // --- Panel bring-up -----------------------------------------------------
    let reset = Output::new(p.reset, Level::High, OutputConfig::default());
    let panel = Co5300::<AM151Q466466LK_151_C, _, _>::new(LCDResetOption::new_pin(reset));

    let mut delay = Delay;
    let mut display = DisplayDriver::builder(bus, panel)
        .with_color_format(ColorFormat::RGB565)
        .init(&mut delay)
        .await
        .unwrap();

    display.set_brightness(0xFF).await.unwrap();

    (display, framebuffer)
}

/// Pushes the whole framebuffer to the panel over QSPI.
pub async fn flush(display: &mut Display, fb: &FrameBuf) -> Result<(), ()> {
    display.write_frame(fb.as_bytes()).await.map_err(|_| ())
}
