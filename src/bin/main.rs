#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use energy_meter::display::{self, DisplayPeripherals};
use energy_meter::touch::{Cst9217, TouchPeripherals};
use energy_meter::ui;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

/// Simulated meter readings, cycled through with the BOOT button.
const READINGS: [u32; 5] = [0, 125, 480, 1234, 3300];

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

    // Bring up the AMOLED display (see `display::init` for the pin mapping).
    let (mut display, mut framebuffer) = display::init(DisplayPeripherals {
        psram: peripherals.PSRAM,
        spi: peripherals.SPI2,
        dma: peripherals.DMA_CH0,
        sclk: peripherals.GPIO38,
        cs: peripherals.GPIO12,
        d0: peripherals.GPIO4,
        d1: peripherals.GPIO5,
        d2: peripherals.GPIO6,
        d3: peripherals.GPIO7,
        reset: peripherals.GPIO39,
    })
    .await;

    // Bring up the CST9217 capacitive touch controller (I2C).
    let mut touch = Cst9217::init(TouchPeripherals {
        i2c: peripherals.I2C0,
        sda: peripherals.GPIO15,
        scl: peripherals.GPIO14,
        int: peripherals.GPIO11,
        reset: peripherals.GPIO40,
    })
    .await;

    // First screen.
    let mut index = 0usize;
    ui::draw(&mut framebuffer, READINGS[index]);
    display::flush(&mut display, &framebuffer).await.ok();

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    loop {
        // Reacciona a lo que ocurra primero: el BOOT button o un toque.
        match select(boot_button.wait_for_falling_edge(), touch.wait_for_touch()).await {
            Either::First(()) => {
                info!("BOOT button pressed");

                // Cada pulsación muestra la siguiente lectura simulada.
                index = (index + 1) % READINGS.len();
                ui::draw(&mut framebuffer, READINGS[index]);
                display::flush(&mut display, &framebuffer).await.ok();

                // Espera hasta que se suelte el BOOT button
                boot_button.wait_for_rising_edge().await;
                info!("BOOT button released");
            }
            Either::Second(()) => {
                if let Some(point) = touch.read().await {
                    info!("Touch at ({}, {})", point.x, point.y);
                    ui::draw(&mut framebuffer, READINGS[index]);
                    ui::draw_touch_marker(&mut framebuffer, point);
                    display::flush(&mut display, &framebuffer).await.ok();
                }
            }
        }
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
