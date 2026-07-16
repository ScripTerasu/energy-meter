#![no_std]

//! Firmware library for the energy-meter board
//! (Waveshare ESP32-S3-Touch-AMOLED-1.75).
//!
//! The code is split into focused modules:
//! - [`board`]: pin assignments and panel geometry.
//! - [`display`]: CO5300 AMOLED bring-up, QSPI bus and PSRAM framebuffer.
//! - [`touch`]: CST9217 capacitive touch controller over I2C.
//! - [`ui`]: rendering of the on-screen interface with `embedded-graphics`.

pub mod board;
pub mod display;
pub mod touch;
pub mod ui;
