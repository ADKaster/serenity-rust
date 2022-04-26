/*
 * Copyright (c) 2021, Ali Mohammad Pur <mpfard@serenityos.org>
 * Copyright (c) 2022, Andreas Kling <kling@serenityos.org>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::mem::swap;

use gfx::geometry::{Point, Rect, Size};

use crate::gfx;

#[derive(Clone)]
pub(crate) struct SplitLineSegment {
    from: Point,
    to: Point,
    pub(crate) inverse_slope: f32,
    x_of_minimum_y: f32,
    pub(crate) maximum_y: f32,
    pub(crate) minimum_y: f32,
    pub(crate) x: f32,
}

pub(crate) enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    QuadraticBezierCurveTo(Point, Point),
    CubicBezierCurveTo(Point, Point, Point),
    EllipticalArcTo(Point, Point, Point, f32, f32, f32, bool, bool),
}

pub struct Path {
    pub(crate) segments: Vec<PathSegment>,
}

impl Path {
    pub fn new() -> Path {
        Path {
            segments: Vec::new(),
        }
    }

    pub fn move_to(&mut self, point: Point) { self.segments.push(PathSegment::MoveTo(point)); }

    pub fn line_to(&mut self, point: Point) { self.segments.push(PathSegment::LineTo(point)); }

    pub fn cubic_bezier_curve_to(&mut self, c1: Point, c2: Point, point: Point) {
        self.segments
            .push(PathSegment::CubicBezierCurveTo(c1, c2, point));
    }

    pub fn quadratic_bezier_curve_to(&mut self, c: Point, point: Point) {
        self.segments
            .push(PathSegment::QuadraticBezierCurveTo(c, point));
    }

    pub fn elliptical_arc_to(
        &mut self,
        point: Point,
        center: Point,
        radii: Point,
        x_axis_rotation: f32,
        theta: f32,
        theta_delta: f32,
        large_arc: bool,
        sweep: bool,
    ) {
        self.segments.push(PathSegment::EllipticalArcTo(
            point,
            center,
            radii,
            x_axis_rotation,
            theta,
            theta_delta,
            large_arc,
            sweep,
        ));
    }

    pub fn clear(&mut self) { self.segments.clear(); }

    pub fn add_rect(&mut self, rect: Rect) {
        self.move_to(rect.top_left());
        self.line_to(rect.top_right());
        self.line_to(rect.bottom_right());
        self.line_to(rect.bottom_left());
        self.line_to(rect.top_left());
    }

    pub(crate) fn segmentize(&self) -> (Rect, Vec<SplitLineSegment>) {
        let mut segments = Vec::new();

        struct BoundingBox {
            min_x: f32,
            min_y: f32,
            max_x: f32,
            max_y: f32,
        }

        impl BoundingBox {
            pub fn add_point(&mut self, point: &Point) {
                if point.x < self.min_x {
                    self.min_x = point.x
                };
                if point.y < self.min_y {
                    self.min_y = point.y
                };
                if point.x > self.max_x {
                    self.max_x = point.x
                };
                if point.y > self.max_y {
                    self.max_y = point.y
                };
            }

            pub fn set(&mut self, point: &Point) {
                self.min_x = point.x;
                self.min_y = point.y;
                self.max_x = point.x;
                self.max_y = point.y;
            }
        }

        let mut bbox = BoundingBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        };

        let mut add_line = |p0: &Point, p1: &Point, bbox: &mut BoundingBox| {
            let mut ymax = p0.y;
            let mut ymin = p1.y;
            let mut x_of_ymin = p1.x;
            let mut x_of_ymax = p0.x;
            let slope = if p0.x == p1.x {
                0.0
            } else {
                (p0.y - p1.y) / (p0.x - p1.x)
            };
            if p0.y < p1.y {
                swap(&mut ymin, &mut ymax);
                swap(&mut x_of_ymin, &mut x_of_ymax);
            }

            segments.push(SplitLineSegment {
                from: p0.clone(),
                to: p1.clone(),
                inverse_slope: if slope == 0.0 { 0.0 } else { 1.0 / slope },
                x_of_minimum_y: x_of_ymin,
                maximum_y: ymax,
                minimum_y: ymin,
                x: x_of_ymax,
            });

            bbox.add_point(&p1);
        };

        let mut cursor = Point::new(0.0, 0.0);
        let mut first = true;

        for segment in self.segments.iter() {
            match segment {
                PathSegment::MoveTo(point) => {
                    if first {
                        bbox.set(&point);
                    } else {
                        bbox.add_point(point);
                    }
                    cursor = point.clone();
                }
                PathSegment::LineTo(point) => {
                    add_line(&cursor, point, &mut bbox);
                    cursor = point.clone();
                }
                PathSegment::QuadraticBezierCurveTo(_control, _point) => {
                    todo!("FIXME");
                }
                PathSegment::CubicBezierCurveTo(_control_1, _control_2, _point) => {
                    todo!("FIXME");
                }
                PathSegment::EllipticalArcTo(
                    _point,
                    _center,
                    _radii,
                    _x_axis_rotation,
                    _theta,
                    _theta_delta,
                    _large_arc,
                    _sweep,
                ) => {
                    todo!("FIXME");
                }
            };

            first = false;
        }

        // sort segments by ymax
        segments.sort_by(|line0, line1| {
            return line1.maximum_y.total_cmp(&line0.maximum_y);
        });

        let bounding_rect = Rect::new(
            bbox.min_x,
            bbox.min_y,
            bbox.max_x - bbox.min_x,
            bbox.max_y - bbox.min_y,
        );
        (bounding_rect, segments)
    }
}
