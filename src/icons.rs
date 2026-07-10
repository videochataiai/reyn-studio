//! Hand-drawn vector icons (egui painter). Crisp at any DPI, no icon-font dep.
#![allow(dead_code)] // full icon set kept; some used by later views
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};

#[derive(Clone, Copy)]
pub enum Icon {
    Upload, Download, Orbit, Brush, Chart, Gear, Book, Heart, Play, Cube, Layers,
}

pub fn draw(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = Stroke::new(1.6, color);
    let sw = Stroke::new(2.1, color);
    let pt = |fx: f32, fy: f32| Pos2::new(rect.min.x + fx * rect.width(), rect.min.y + fy * rect.height());
    let c = pt(0.5, 0.5);
    let w = rect.width();

    match icon {
        Icon::Upload => {
            p.line_segment([pt(0.2, 0.84), pt(0.8, 0.84)], s);
            p.line_segment([pt(0.5, 0.16), pt(0.5, 0.66)], s);
            p.line(vec![pt(0.32, 0.34), pt(0.5, 0.16), pt(0.68, 0.34)], s);
        }
        Icon::Download => {
            p.line_segment([pt(0.2, 0.84), pt(0.8, 0.84)], s);
            p.line_segment([pt(0.5, 0.18), pt(0.5, 0.68)], s);
            p.line(vec![pt(0.32, 0.5), pt(0.5, 0.68), pt(0.68, 0.5)], s);
        }
        Icon::Orbit => {
            p.circle_stroke(c, w * 0.24, s);
            // orbit ring (flattened) as two arcs approximated by a thin ellipse of segments
            let mut ring = Vec::new();
            for k in 0..=24 {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                ring.push(Pos2::new(c.x + a.cos() * w * 0.44, c.y + a.sin() * w * 0.20));
            }
            p.line(ring, Stroke::new(1.2, color.gamma_multiply(0.8)));
            p.circle_filled(pt(0.94, 0.5), 2.0, color);
        }
        Icon::Brush => {
            p.line_segment([pt(0.82, 0.18), pt(0.44, 0.56)], sw);
            p.add(Shape::convex_polygon(
                vec![pt(0.44, 0.54), pt(0.3, 0.6), pt(0.22, 0.8), pt(0.42, 0.72)],
                color, Stroke::NONE));
        }
        Icon::Chart => {
            for (i, h) in [0.32f32, 0.56, 0.78].iter().enumerate() {
                let x = 0.28 + i as f32 * 0.22;
                p.line_segment([pt(x, 0.82), pt(x, 0.82 - h)], sw);
            }
        }
        Icon::Gear => {
            p.circle_stroke(c, w * 0.2, s);
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                let (cs, sn) = (a.cos(), a.sin());
                p.line_segment([
                    Pos2::new(c.x + cs * w * 0.28, c.y + sn * w * 0.28),
                    Pos2::new(c.x + cs * w * 0.42, c.y + sn * w * 0.42),
                ], s);
            }
        }
        Icon::Book => {
            for seg in [
                [pt(0.24, 0.22), pt(0.24, 0.8)], [pt(0.76, 0.22), pt(0.76, 0.8)],
                [pt(0.24, 0.22), pt(0.76, 0.22)], [pt(0.24, 0.8), pt(0.76, 0.8)],
                [pt(0.5, 0.22), pt(0.5, 0.8)],
            ] { p.line_segment(seg, s); }
        }
        Icon::Heart => {
            let r = w * 0.16;
            p.circle_filled(pt(0.37, 0.4), r, color);
            p.circle_filled(pt(0.63, 0.4), r, color);
            p.add(Shape::convex_polygon(
                vec![pt(0.21, 0.45), pt(0.79, 0.45), pt(0.5, 0.82)], color, Stroke::NONE));
        }
        Icon::Play => {
            p.add(Shape::convex_polygon(
                vec![pt(0.32, 0.24), pt(0.32, 0.76), pt(0.78, 0.5)], color, Stroke::NONE));
        }
        Icon::Cube => {
            for seg in [
                [pt(0.5, 0.15), pt(0.83, 0.33)], [pt(0.83, 0.33), pt(0.83, 0.67)],
                [pt(0.83, 0.67), pt(0.5, 0.85)], [pt(0.5, 0.85), pt(0.17, 0.67)],
                [pt(0.17, 0.67), pt(0.17, 0.33)], [pt(0.17, 0.33), pt(0.5, 0.15)],
                [pt(0.17, 0.33), pt(0.5, 0.5)], [pt(0.83, 0.33), pt(0.5, 0.5)], [pt(0.5, 0.5), pt(0.5, 0.85)],
            ] { p.line_segment(seg, Stroke::new(1.3, color)); }
        }
        Icon::Layers => {
            p.line(vec![pt(0.5, 0.18), pt(0.84, 0.36), pt(0.5, 0.54), pt(0.16, 0.36), pt(0.5, 0.18)], s);
            p.line(vec![pt(0.16, 0.54), pt(0.5, 0.72), pt(0.84, 0.54)], s);
        }
    }
}
