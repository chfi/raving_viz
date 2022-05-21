use nalgebra as na;

use nalgebra::*;

pub type Vec2 = Vector2<f32>;

pub fn vector_field_vertices<F>(
    width: f32,
    height: f32,
    rows: usize,
    cols: usize,
    color: [f32; 4],
    buf: &mut Vec<[u8; 40]>,
    f: F,
) where
    F: Fn(Vec2) -> Vec2,
{
    buf.clear();

    for r in 0..rows {
        for c in 0..cols {
            let i_x = (c as f32) / cols as f32;
            let i_y = (r as f32) / rows as f32;

            let point = Vec2::new(i_x, i_y);
            let out = f(point);

            let s0 = Vec2::new(i_x * width, i_y * height);

            // let mult = Vec::new(1.0 / width, 1.0 / height);

            let n_v = out.normalize();

            let s1 = s0 + n_v * 10.0;

            let mut vertex = [0u8; 40];

            vertex[0..12]
                .clone_from_slice(bytemuck::cast_slice(&[s0.x, s0.y, 3.0]));
            vertex[12..24]
                .clone_from_slice(bytemuck::cast_slice(&[s1.x, s1.y, 1.0]));
            vertex[24..40].clone_from_slice(bytemuck::cast_slice(&color));

            buf.push(vertex);
        }
    }
}
