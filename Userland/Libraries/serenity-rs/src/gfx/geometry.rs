/*
 * Copyright (c) 2020-2022, Andreas Kling <kling@serenityos.org>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

#[derive(Clone)]
pub struct Point {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

#[derive(Clone)]
pub struct Size {
    width: f32,
    height: f32,
}

pub struct Rect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub struct AffineTransform {
    pub(crate) a: f32,
    pub(crate) b: f32,
    pub(crate) c: f32,
    pub(crate) d: f32,
    pub(crate) e: f32,
    pub(crate) f: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Point { Point { x, y } }

    pub fn x(&self) -> f32 { self.x }

    pub fn y(&self) -> f32 { self.y }

    pub fn dx_relative_to(&self, other: &Point) -> f32 { return self.x - other.x; }

    pub fn dy_relative_to(&self, other: &Point) -> f32 { return self.y - other.y; }
}

impl Size {
    pub fn new(width: f32, height: f32) -> Size { Size { width, height } }

    pub fn width(&self) -> f32 { self.width }

    pub fn height(&self) -> f32 { self.height }
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn centered_on(center: Point, size: Size) -> Rect {
        Rect {
            x: center.x - size.width / 2.0,
            y: center.y - size.height / 2.0,
            width: size.width,
            height: size.height,
        }
    }

    pub fn top_left(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }

    pub fn top_right(&self) -> Point {
        Point {
            x: self.x + self.width - 1.0,
            y: self.y,
        }
    }

    pub fn bottom_right(&self) -> Point {
        Point {
            x: self.x + self.width - 1.0,
            y: self.y + self.height - 1.0,
        }
    }

    pub fn bottom_left(&self) -> Point {
        Point {
            x: self.x,
            y: self.y + self.height - 1.0,
        }
    }
}

impl AffineTransform {
    pub fn new_identity() -> AffineTransform {
        AffineTransform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> AffineTransform {
        AffineTransform { a, b, c, d, e, f }
    }

    pub fn is_identity(&self) -> bool {
        self.a == 1.0
            && self.b == 0.0
            && self.c == 0.0
            && self.d == 1.0
            && self.e == 0.0
            && self.f == 0.0
    }

    pub fn is_identity_or_translation(&self) -> bool {
        self.a == 1.0 && self.b == 0.0 && self.c == 0.0 && self.d == 1.0
    }

    pub fn map(&self, point: &Point) -> Point {
        Point {
            x: self.a * point.x + self.c * point.y + self.e,
            y: self.b * point.x + self.d * point.y + self.f,
        }
    }

    pub fn translate(&mut self, x: f32, y: f32) {
        self.e += x * self.a + y * self.c;
        self.f += x * self.b + y * self.d;
    }

    pub fn multiply(&mut self, other: &AffineTransform) {
        let a = other.a * self.a + other.b * self.c;
        let b = other.a * self.b + other.b * self.d;
        let c = other.c * self.a + other.d * self.c;
        let d = other.c * self.b + other.d * self.d;
        let e = other.e * self.a + other.f * self.c + self.e;
        let f = other.e * self.b + other.f * self.d + self.f;
        self.a = a;
        self.b = b;
        self.c = c;
        self.d = d;
        self.e = e;
        self.f = f;
    }

    pub fn rotate_radians(&mut self, radians: f32) {
        let sin_angle = radians.sin();
        let cos_angle = radians.cos();
        let rotation = AffineTransform::new(cos_angle, sin_angle, -sin_angle, cos_angle, 0.0, 0.0);
        self.multiply(&rotation);
    }
}
