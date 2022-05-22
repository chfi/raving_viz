use nalgebra as na;

use nalgebra::*;

use nalgebra_glm::Vec2;

use palette::{FromColor, Hsl, IntoColor, Srgb};

// pub fn curve_vertices<F>(
//     width: f32,
//     height: f32,
//     curve_start: (Vec2,

/*
pub fn vector_field_vertices<F>(
    width: f32,
    height: f32,
    rows: usize,
    cols: usize,
    time: f32,
    color: [f32; 4],
    buf: &mut Vec<[u8; 40]>,
    f: F,
) where
    F: Fn(Vec2) -> Vec2,
{
    buf.clear();

    for r in 0..rows {
        for c in 0..cols {
            let i_x = (0.5 + c as f32) / cols as f32;
            let i_y = (0.5 + r as f32) / rows as f32;

            let point = Vec2::new(i_x, i_y);
            let out = f(point);

            let s0 = Vec2::new(i_x * width, i_y * height);

            // let len =

            // let n_v = out.normalize();

            let s1 = s0 + out * 50.0;

            let mut vertex = [0u8; 40];

            let w0 = 4.0;
            let w1 = 0.5;

            let hsl = Hsl::new(out.norm() * 1800.0, 1.0f32, 0.5);
            let rgb: Srgb = hsl.into_color();

            let color = [rgb.red, rgb.green, rgb.blue, 1.0];

            vertex[0..12]
                .clone_from_slice(bytemuck::cast_slice(&[s0.x, s0.y, w0]));
            vertex[12..24]
                .clone_from_slice(bytemuck::cast_slice(&[s1.x, s1.y, w1]));
            vertex[24..40].clone_from_slice(bytemuck::cast_slice(&color));

            buf.push(vertex);
        }
    }
}
*/
