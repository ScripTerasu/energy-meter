//! RGB565 framebuffer backed by PSRAM.

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::board::{LCD_HEIGHT, LCD_WIDTH};

/// A simple RGB565 framebuffer living in PSRAM.
///
/// `embedded-graphics` is synchronous, so we render into this buffer and then
/// asynchronously push the whole frame to the AMOLED panel (see
/// [`crate::display::flush`]).
pub struct FrameBuf {
    buf: &'static mut [u8],
}

impl FrameBuf {
    /// Wraps a byte buffer (typically a PSRAM slice) as a framebuffer.
    ///
    /// The buffer must be at least [`crate::board::FB_BYTES`] long.
    pub fn new(buf: &'static mut [u8]) -> Self {
        Self { buf }
    }

    /// Returns the raw RGB565 bytes, ready to be streamed to the panel.
    pub fn as_bytes(&self) -> &[u8] {
        self.buf
    }

    /// Sets an individual pixel without going through the `DrawTarget` API.
    pub fn set_pixel(&mut self, point: Point, color: Rgb565) {
        if point.x < 0
            || point.y < 0
            || point.x >= i32::from(LCD_WIDTH)
            || point.y >= i32::from(LCD_HEIGHT)
        {
            return;
        }
        let idx = (point.y as usize * LCD_WIDTH as usize + point.x as usize) * 2;
        let raw = color.into_storage().to_be_bytes();
        self.buf[idx] = raw[0];
        self.buf[idx + 1] = raw[1];
    }
}

impl OriginDimensions for FrameBuf {
    fn size(&self) -> Size {
        Size::new(u32::from(LCD_WIDTH), u32::from(LCD_HEIGHT))
    }
}

impl DrawTarget for FrameBuf {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            if coord.x < 0
                || coord.y < 0
                || coord.x >= i32::from(LCD_WIDTH)
                || coord.y >= i32::from(LCD_HEIGHT)
            {
                continue;
            }
            let idx = (coord.y as usize * LCD_WIDTH as usize + coord.x as usize) * 2;
            let raw = color.into_storage().to_be_bytes();
            self.buf[idx] = raw[0];
            self.buf[idx + 1] = raw[1];
        }
        Ok(())
    }
}
