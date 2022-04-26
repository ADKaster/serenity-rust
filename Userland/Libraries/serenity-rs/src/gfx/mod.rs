/*
 * Copyright (c) 2022, Andreas Kling <kling@serenityos.org>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

pub mod geometry;
pub mod painter;
pub mod path;

pub use geometry::{AffineTransform, Point, Rect, Size};
pub use painter::Painter;
pub use path::Path;

#[derive(Clone)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Color {
        Color {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    pub fn from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn with_alpha(&self, alpha: u8) -> Color {
        Color {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha,
        }
    }

    pub fn blend(&self, source: &Color) -> Color {
        if self.alpha == 0 || source.alpha == 255 {
            return source.clone();
        }

        if source.alpha == 0 {
            return self.clone();
        }

        let dst_int_red = self.red as i32;
        let dst_int_green = self.green as i32;
        let dst_int_blue = self.blue as i32;
        let dst_int_alpha = self.alpha as i32;

        let src_int_red = source.red as i32;
        let src_int_green = source.green as i32;
        let src_int_blue = source.blue as i32;
        let src_int_alpha = source.alpha as i32;

        let d = 255 * (dst_int_alpha + src_int_alpha) - dst_int_alpha * src_int_alpha;
        let r = (dst_int_red * dst_int_alpha * (255 - src_int_alpha)
            + 255 * src_int_alpha * src_int_red)
            / d;
        let g = (dst_int_green * dst_int_alpha * (255 - src_int_alpha)
            + 255 * src_int_alpha * src_int_green)
            / d;
        let b = (dst_int_blue * dst_int_alpha * (255 - src_int_alpha)
            + 255 * src_int_alpha * src_int_blue)
            / d;
        let a = d / 255;

        return Color::from_rgba(r as u8, g as u8, b as u8, a as u8);
    }
}

pub struct Bitmap {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
}

impl Bitmap {
    pub fn new(size: Size) -> Result<Bitmap, String> {
        let pitch = size.width() as usize * 4;
        let size_in_bytes = pitch * size.height() as usize;

        Ok(Bitmap {
            data: vec![0; size_in_bytes as usize],
            width: size.width() as u32,
            height: size.height() as u32,
            pitch: pitch as u32,
        })
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: &Color) {
        let slice = self.data.as_mut_slice();
        let base = (y as u32 * self.pitch + x as u32 * 4) as usize;
        slice[base + 0] = color.blue;
        slice[base + 1] = color.green;
        slice[base + 2] = color.red;
        slice[base + 3] = color.alpha;
    }

    pub fn get_pixel(&mut self, x: i32, y: i32) -> Color {
        let slice = self.data.as_mut_slice();
        let base = (y as u32 * self.pitch + x as u32 * 4) as usize;
        Color {
            blue: slice[base + 0],
            green: slice[base + 1],
            red: slice[base + 2],
            alpha: slice[base + 3],
        }
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, color: &Color) {
        if color.alpha == 255 {
            self.set_pixel(x, y, color);
        } else {
            let current = self.get_pixel(x, y);
            self.set_pixel(x, y, &current.blend(color));
        }
    }
}
