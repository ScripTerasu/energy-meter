//! On-screen interface rendered with `embedded-graphics`.

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};

use crate::board::{LCD_HEIGHT, LCD_WIDTH};
use crate::display::FrameBuf;
use crate::touch::TouchPoint;

/// Draws the energy-meter screen for the given reading (in watts).
pub fn draw(fb: &mut FrameBuf, watts: u32) {
    let bg = Rgb565::new(4, 8, 12);
    let accent = Rgb565::CSS_DEEP_SKY_BLUE;
    let white = Rgb565::WHITE;

    // Background.
    fb.clear(bg).ok();

    // Header bar.
    Rectangle::new(Point::new(0, 0), Size::new(u32::from(LCD_WIDTH), 70))
        .into_styled(PrimitiveStyleBuilder::new().fill_color(accent).build())
        .draw(fb)
        .ok();

    let title_style = MonoTextStyle::new(&FONT_10X20, white);
    Text::with_alignment(
        "ENERGY METER",
        Point::new(i32::from(LCD_WIDTH) / 2, 44),
        title_style,
        Alignment::Center,
    )
    .draw(fb)
    .ok();

    // Decorative ring in the center.
    let center = Point::new(i32::from(LCD_WIDTH) / 2, i32::from(LCD_HEIGHT) / 2);
    Circle::with_center(center, 260)
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(accent)
                .stroke_width(6)
                .build(),
        )
        .draw(fb)
        .ok();

    // Reading, formatted without alloc.
    let mut buf = [0u8; 16];
    let text = format_watts(&mut buf, watts);
    let value_style = MonoTextStyle::new(&FONT_10X20, white);
    Text::with_alignment(text, center, value_style, Alignment::Center)
        .draw(fb)
        .ok();
}

/// Draws a filled marker at the last touched coordinate.
pub fn draw_touch_marker(fb: &mut FrameBuf, point: TouchPoint) {
    let marker = Rgb565::CSS_ORANGE_RED;
    Circle::with_center(Point::new(i32::from(point.x), i32::from(point.y)), 24)
        .into_styled(PrimitiveStyleBuilder::new().fill_color(marker).build())
        .draw(fb)
        .ok();
}

/// Formats `"<watts> W"` into `buf` without heap allocation.
fn format_watts(buf: &mut [u8; 16], watts: u32) -> &str {
    let mut digits = [0u8; 10];
    let mut n = watts;
    let mut len = 0;
    if n == 0 {
        digits[len] = b'0';
        len += 1;
    } else {
        while n > 0 {
            digits[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
    }

    let mut pos = 0;
    for i in (0..len).rev() {
        buf[pos] = digits[i];
        pos += 1;
    }
    buf[pos] = b' ';
    buf[pos + 1] = b'W';
    pos += 2;

    // SAFETY: only ASCII digits, a space and 'W' were written.
    core::str::from_utf8(&buf[..pos]).unwrap_or("")
}
