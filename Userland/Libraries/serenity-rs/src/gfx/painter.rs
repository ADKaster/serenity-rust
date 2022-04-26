/*
 * Copyright (c) 2021, Ali Mohammad Pur <mpfard@serenityos.org>
 * Copyright (c) 2020-2022, Andreas Kling <kling@serenityos.org>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::mem::swap;

use crate::gfx::path::PathSegment;
use crate::gfx::{AffineTransform, Bitmap, Color, Path, Point, Rect, Size};

pub enum WindingRule {
    Nonzero,
    EvenOdd,
}

pub struct Painter<'a> {
    target: &'a mut Bitmap,
    transform: AffineTransform,
    antialiasing: bool,
}

impl<'a> Painter<'a> {
    pub fn new(target: &'a mut Bitmap) -> Painter<'a> {
        Painter {
            target,
            transform: AffineTransform::new_identity(),
            antialiasing: true,
        }
    }

    pub fn rotate(&mut self, degrees: f32) { self.transform.rotate_radians(degrees.to_radians()); }

    pub fn translate(&mut self, x: f32, y: f32) { self.transform.translate(x, y); }

    pub fn is_antialiasing_enabled(&self) -> bool { self.antialiasing }

    pub fn set_antialiasing_enabled(&mut self, enabled: bool) { self.antialiasing = enabled; }

    pub fn fill_rect(&mut self, rect: Rect, color: &Color) {
        if self.transform.is_identity_or_translation() {
            let start_x = rect.x + self.transform.e;
            let start_y = rect.y + self.transform.f;
            let end_x = start_x + rect.width;
            let end_y = start_y + rect.height;
            let mut y = start_y;
            while y < end_y {
                let mut x = start_x;
                while x < end_x {
                    self.target.blend_pixel(x as i32, y as i32, color);
                    x += 1.0
                }
                y += 1.0
            }
        } else {
            let mut path = Path::new();
            path.add_rect(rect);
            self.fill_path(&path, color, WindingRule::Nonzero);
        }
    }

    fn clear_rect_ignoring_transform(&mut self, rect: Rect, color: &Color) {
        let start_x = rect.x;
        let start_y = rect.y;
        let end_x = start_x + rect.width;
        let end_y = start_y + rect.height;
        let mut y = start_y;
        while y < end_y {
            let mut x = start_x;
            while x < end_x {
                self.target.set_pixel(x as i32, y as i32, color);
                x += 1.0
            }
            y += 1.0
        }
    }

    pub fn draw_line(&mut self, from: &Point, to: &Point, color: &Color, antialias: bool) {
        let mapped_from = self.transform.map(&from);
        let mapped_to = self.transform.map(&to);

        let mut plot = |x: f32, y: f32, c: f32| {
            self.target.blend_pixel(
                x as i32,
                y as i32,
                &color.with_alpha((color.alpha as f32 * c) as u8),
            );
        };

        let integer_part = |x: f32| x.floor();
        let round = |x: f32| integer_part(x + 0.5);
        let fractional_part = |x: f32| x - x.floor();
        let one_minus_fractional_part = |x: f32| {
            return 1.0 - fractional_part(x);
        };

        let mut draw_line = |mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32| {
            let steep = (y1 - y0).abs() > (x1 - x0).abs();

            if steep {
                swap(&mut x0, &mut y0);
                swap(&mut x1, &mut y1);
            }

            if x0 > x1 {
                swap(&mut x0, &mut x1);
                swap(&mut y0, &mut y1);
            }

            let dx = x1 - x0;
            let dy = y1 - y0;

            let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

            // Handle first endpoint.
            let x_end = round(x0);
            let y_end = y0 + gradient * (x_end - x0);
            let x_gap = one_minus_fractional_part(x0 + 0.5);

            let xpxl1 = x_end; // This will be used in the main loop.
            let ypxl1 = integer_part(y_end);

            if steep {
                plot(ypxl1, xpxl1, one_minus_fractional_part(y_end) * x_gap);
                plot(ypxl1 + 1.0, xpxl1, fractional_part(y_end) * x_gap);
            } else {
                plot(xpxl1, ypxl1, one_minus_fractional_part(y_end) * x_gap);
                plot(xpxl1, ypxl1 + 1.0, fractional_part(y_end) * x_gap);
            }

            let mut intery = y_end + gradient; // First y-intersection for the main loop.

            // Handle second endpoint.
            let x_end = round(x1);
            let y_end = y1 + gradient * (x_end - x1);
            let x_gap = fractional_part(x1 + 0.5);
            let xpxl2 = x_end; // This will be used in the main loop
            let ypxl2 = integer_part(y_end);

            if steep {
                plot(ypxl2, xpxl2, one_minus_fractional_part(y_end) * x_gap);
                plot(ypxl2 + 1.0, xpxl2, fractional_part(y_end) * x_gap);
            } else {
                plot(xpxl2, ypxl2, one_minus_fractional_part(y_end) * x_gap);
                plot(xpxl2, ypxl2 + 1.0, fractional_part(y_end) * x_gap);
            }

            // Main loop.
            if steep {
                let mut x = xpxl1 + 1.0;
                while x <= xpxl2 - 1.0 {
                    if antialias {
                        plot(integer_part(intery), x, one_minus_fractional_part(intery));
                    } else {
                        plot(integer_part(intery), x, 1.0);
                    }
                    plot(integer_part(intery) + 1.0, x, fractional_part(intery));
                    intery += gradient;
                    x += 1.0;
                }
            } else {
                let mut x = xpxl1 + 1.0;
                while x <= xpxl2 - 1.0 {
                    if antialias {
                        plot(x, integer_part(intery), one_minus_fractional_part(intery));
                    } else {
                        plot(x, integer_part(intery), 1.0);
                    }
                    plot(x, integer_part(intery) + 1.0, fractional_part(intery));
                    intery += gradient;
                    x += 1.0;
                }
            }
        };

        draw_line(mapped_from.x, mapped_from.y, mapped_to.x, mapped_to.y);
    }

    pub fn stroke_path(&mut self, path: &Path, color: &Color) {
        let mut cursor = Point::new(0.0, 0.0);
        for segment in path.segments.iter() {
            match segment {
                PathSegment::MoveTo(point) => {
                    cursor = point.clone();
                }
                PathSegment::LineTo(point) => {
                    self.draw_line(&cursor, point, color, true);
                    cursor = point.clone();
                }
                _ => {}
            }
        }
    }

    pub fn fill_path(&mut self, path: &Path, color: &Color, winding_rule: WindingRule) {
        let (bounding_box, lines) = path.segmentize();
        if lines.is_empty() {
            return;
        }

        let first_y = bounding_box.bottom_right().y + 1.0;
        let last_y = bounding_box.top_left().y - 1.0;
        let mut scanline = first_y;

        let mut last_active_segment: usize = 0;
        let mut active_list = Vec::new();

        for segment in lines.iter() {
            if segment.maximum_y != scanline {
                break;
            }
            active_list.push((*segment).clone());
            last_active_segment += 1;
        }

        let increment_winding =
            |winding_number: &mut i32, from: &Point, to: &Point| match winding_rule {
                WindingRule::EvenOdd => {
                    *winding_number += 1;
                }
                WindingRule::Nonzero => {
                    if from.dy_relative_to(to) < 0.0 {
                        *winding_number += 1;
                    } else {
                        *winding_number -= 1;
                    }
                }
            };
        let mut n: usize = 0;

        while scanline >= last_y {
            if !active_list.is_empty() {
                // sort the active list by 'x' from right to left
                active_list.sort_by(|line0, line1| {
                    return line1.x.total_cmp(&line0.x);
                });

                if active_list.len() > 1 {
                    let mut winding_number = match winding_rule {
                        WindingRule::Nonzero => 1,
                        _ => 0,
                    };
                    for i in 1..active_list.len() {
                        let previous = active_list.get(i - 1).unwrap();
                        let current = &active_list[i];

                        let from = Point::new(previous.x, scanline);
                        let to = Point::new(current.x, scanline);

                        let is_inside_shape = match winding_rule {
                            WindingRule::Nonzero => winding_number != 0,
                            WindingRule::EvenOdd => winding_number % 2 == 0,
                        };

                        if is_inside_shape {
                            // The points between this segment and the previous are
                            // inside the shape
                            if n % 3 == 0 {
                                self.draw_line(&from, &to, color, false);
                            }
                            n += 1;
                        }

                        let is_passing_through_maxima = scanline == previous.maximum_y
                            || scanline == previous.minimum_y
                            || scanline == current.maximum_y
                            || scanline == current.minimum_y;

                        let mut is_passing_through_vertex = false;

                        if is_passing_through_maxima {
                            is_passing_through_vertex = previous.x == current.x;
                        }

                        if !is_passing_through_vertex
                            || previous.inverse_slope * current.inverse_slope < 0.0
                        {
                            increment_winding(&mut winding_number, &from, &to);
                        }

                        // update the x coord
                        let inverse_slope = active_list[i - 1].inverse_slope;
                        active_list.get_mut(i - 1).unwrap().x -= inverse_slope;
                    }

                    active_list.last_mut().unwrap().x -= active_list.last().unwrap().inverse_slope;
                } else {
                    let point = Point::new(active_list[0].x, scanline);
                    self.draw_line(&point, &point, color, false);

                    // update the x coord
                    active_list.first_mut().unwrap().x -=
                        active_list.first().unwrap().inverse_slope;
                }
            }

            scanline -= 1.0;

            // remove any edge that goes out of bound from the active list
            active_list.retain(|x| scanline >= x.minimum_y);

            let mut j = last_active_segment;
            while j < lines.len() {
                if lines[j].maximum_y < scanline {
                    break;
                }
                if lines[j].minimum_y >= scanline {
                    last_active_segment += 1;
                    j += 1;
                    continue;
                }

                active_list.push(lines[j].clone());
                last_active_segment += 1;
                j += 1;
            }
        }
    }
}
