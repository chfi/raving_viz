use std::collections::BTreeMap;

use ash::vk;

use nalgebra::Point3;
use nalgebra_glm::{mat4, vec2, vec3, vec4, Mat4, Vec2, Vec3};
use raving::vk::{
    descriptor::DescriptorLayoutInfo, BufferIx, DescSetIx, GpuResources,
    VkEngine,
};
use rspirv_reflect::DescriptorInfo;

pub struct Camera {
    eye: Vec3,

    u: Vec3, // points right
    v: Vec3, // points up
    n: Vec3, // points back

    pub buffer: BufferIx,
    pub desc_set: DescSetIx,
}

impl Camera {
    /*
    pub fn move_fwd(&mut self, dt: f32, dist: f32) {
        dbg!(self.n);
        dbg!(self.eye);

        // let fwd = nalgebra_glm::cross(&self.u, &self.v);
        // let fwd = nalgebra_glm::cross(&self.n, &self.v);
        let fwd = nalgebra_glm::cross(&self.u, &self.n);

        self.eye += fwd * dist * dt;

        // self.eye += self.n * dist * dt;
        // nalgebra_glm::translate(m, v)
        // self.eye.append_translation(shift)
    }

    pub fn move_right(&mut self, dt: f32, dist: f32) {
        self.eye += self.u * dist * dt;
    }

    pub fn rotate_hor(&mut self, angle: f32) {
        // let u = nalgebra_glm::rotate_vec3(&self.u, angle, &self.v);
        // let n = nalgebra_glm::rotate_vec3(&self.n, angle, &self.v);
        let v = nalgebra_glm::cross(&self.u, &self.n);
        let u = nalgebra_glm::rotate_vec3(&self.u, angle, &v);
        let n = nalgebra_glm::rotate_vec3(&self.n, angle, &v);
        self.u = u;
        self.n = n;
    }

    pub fn rotate_ver(&mut self, angle: f32) {
        // let v = nalgebra_glm::rotate_vec3(&self.v, angle, &self.u);
        // let n = nalgebra_glm::rotate_vec3(&self.n, angle, &self.u);
        let u = nalgebra_glm::cross(&self.v, &self.n);
        let v = nalgebra_glm::rotate_vec3(&self.v, angle, &u);
        let n = nalgebra_glm::rotate_vec3(&self.n, angle, &u);
        self.v = v;
        self.n = n;
    }
    */

    pub fn new(engine: &mut VkEngine) -> anyhow::Result<Self> {
        let eye = vec3(0f32, 0.7, -10.0);
        let u = vec3(1f32, 0.0, 0.0);
        let v = vec3(0f32, 1.0, 0.0);
        let n = vec3(0f32, 0.0, -1.0);

        let (buffer, desc_set) =
            engine.with_allocators(|ctx, res, alloc| {
                let usage = vk::BufferUsageFlags::UNIFORM_BUFFER;

                let buf = res.allocate_buffer(
                    ctx,
                    alloc,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                    4,  // f32
                    16, // 4x4 matrix
                    usage,
                    Some("camera uniform buffer"),
                )?;

                let buf = res.insert_buffer(buf);

                let set = allocate_uniform_desc_set(res, buf)?;

                let set = res.insert_desc_set(set);

                Ok((buf, set))
            })?;

        let mut result = Self {
            eye,
            u,
            v,
            n,

            buffer,
            desc_set,
        };

        Ok(result)
    }

    pub fn write_uniform_fixed(
        &self,
        res: &mut GpuResources,
        eye: Vec3,
        tgt: Vec3,
        dims: [f32; 2],
    ) {
        let mat = nalgebra_glm::look_at_rh(&eye, &tgt, &vec3(0f32, 1.0, 0.0));

        let [width, height] = dims;
        let proj =
            nalgebra_glm::perspective_fov(1.4f32, width, height, 1.0, 1000.0);

        let mat = proj * mat;

        let buf = &mut res[self.buffer];
        if let Some(slice) = buf.mapped_slice_mut() {
            slice.clone_from_slice(bytemuck::cast_slice(mat.as_slice()));
        }
    }

    pub fn write_uniform(&self, res: &mut GpuResources, dims: [f32; 2]) {
        let [width, height] = dims;
        let buf = &mut res[self.buffer];

        #[rustfmt::skip]
        let translate = mat4(
                 1.0, 0.0, 0.0, -self.eye.x,
                 0.0, 1.0, 0.0, -self.eye.y,
                 0.0, 0.0, 1.0, -self.eye.z,
                 0.0, 0.0, 0.0,         1.0);

        let u = self.u;
        let v = self.v;
        let n = self.n;

        #[rustfmt::skip]
        let rot_align = mat4(
            u.x, u.y, u.z, 0.0,
            v.x, v.y, v.z, 0.0,
            n.x, n.y, n.z, 0.0,
            0.0, 0.0, 0.0, 1.0);

        let proj =
            nalgebra_glm::perspective_fov(1.4, width, height, 1.0, 1000.0);

        // let mat = proj * rot_align * translate;
        let mat = proj * translate * rot_align;

        if let Some(slice) = buf.mapped_slice_mut() {
            slice.clone_from_slice(bytemuck::cast_slice(mat.as_slice()));
        }
    }
}

/*
pub struct Uniform {
    buffer: BufferIx,
    desc_set: DescSetIx,
}
*/

pub fn sampled_disc(
    engine: &mut VkEngine,
    clear_queue: &crossbeam::channel::Sender<
        Box<dyn std::any::Any + Send + Sync>,
    >,
    buf: &mut Vec<[u8; 40]>,
    count: usize,
) -> anyhow::Result<(BufferIx, usize)> {
    use rand::prelude::*;
    use rand_distr::{Normal, StandardNormal};
    // use rand_distr::

    buf.clear();

    let mut colors = Vec::new();

    let rgba = |r, g, b| {
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;
        [r, g, b, 1.0]
    };

    colors.push(rgba(0xff, 0xff, 0xff));
    colors.push(rgba(0x1f, 0x77, 0xb4));
    colors.push(rgba(0xff, 0x7f, 0x0e));
    colors.push(rgba(0x2c, 0xa0, 0x2c));
    colors.push(rgba(0xd6, 0x27, 0x28));
    colors.push(rgba(0x94, 0x67, 0xbd));
    colors.push(rgba(0x8c, 0x56, 0x4b));
    colors.push(rgba(0xe3, 0x77, 0xc2));
    colors.push(rgba(0x7f, 0x7f, 0x7f));
    colors.push(rgba(0xbc, 0xbd, 0x22));
    colors.push(rgba(0x17, 0xbe, 0xcf));

    let mut get_color = {
        let mut i = 0;
        move || {
            let color = colors[i % colors.len()];
            i += 1;
            color
        }
    };

    let mut tri_indices: Vec<usize> = Vec::new();

    // let mut distr = Normal::from_mean_cv(0.0, 0.3)?;

    let mut gen_point = {
        use delaunator::Point;

        let mut rng = rand::thread_rng();
        let distr = Normal::from_mean_cv(0.5, 0.5)?;

        move || loop {
            let x = distr.sample(&mut rng);
            let y = distr.sample(&mut rng);

            let v = vec2(x, y);

            if v.norm() <= 1.0 {
                return Point { x, y };
            }
        }
    };

    let mut rng = rand::thread_rng();

    let mut tri_points: Vec<delaunator::Point> = Vec::new();
    let mut points: Vec<Vec2> = Vec::new();

    for i in 0..count {
        let p = gen_point();
        let v = vec2(p.x as f32, p.y as f32);
        points.push(v);
        // points.push(v + vec2(0.5, -0.5));
        tri_points.push(p);
    }

    let mut vx = |x: f32, y: f32, z: f32| {
        let mut v0 = [0u8; 40];
        v0[0..12].clone_from_slice(bytemuck::cast_slice(&[x, y, z]));
        v0[12..24].clone_from_slice(bytemuck::cast_slice(&[1f32, 0.0, 0.0]));

        v0[24..40].clone_from_slice(bytemuck::cast_slice(&get_color()));
        // v0[24..40].clone_from_slice(bytemuck::cast_slice(&[x, y, z, 1.0]));
        v0
    };

    for point in points {
        // let o = vec2(0.5f32, 0.5);
        let p = vec2(point.x, point.y);
        // let z = (o - p).norm();
        let z = p.norm();

        buf.push(vx(point.x, z, point.y));
        // buf.push(vx(point.x, 1.0, point.y));
    }

    let result = delaunator::triangulate(&tri_points);

    index_buffer(
        engine,
        clear_queue,
        result.triangles.into_iter().map(|s| s as u32),
        // result.triangles.into_iter().rev().map(|s| s as u32),
        // tri_indices.into_iter().map(|s| s as u32),
    )
}

pub fn index_buffer(
    engine: &mut VkEngine,
    clear_queue: &crossbeam::channel::Sender<
        Box<dyn std::any::Any + Send + Sync>,
    >,
    indices: impl IntoIterator<Item = u32>,
) -> anyhow::Result<(BufferIx, usize)> {
    let indices = indices.into_iter().collect::<Vec<_>>();
    let ix_count = indices.len();

    let ix_buf = engine.with_allocators(|ctx, res, alloc| {
        let usage = vk::BufferUsageFlags::TRANSFER_DST
            | vk::BufferUsageFlags::INDEX_BUFFER;

        let buf = res.allocate_buffer(
            ctx,
            alloc,
            gpu_allocator::MemoryLocation::GpuOnly,
            4,
            ix_count,
            usage,
            Some("index_buffer tmp"),
        )?;

        let ix = res.insert_buffer(buf);

        Ok(ix)
    })?;

    let staging = engine.submit_queue_fn(|ctx, res, alloc, cmd| {
        let buf = &mut res[ix_buf];

        let staging = buf.upload_to_self_bytes(
            ctx,
            alloc,
            bytemuck::cast_slice(&indices),
            cmd,
        )?;

        Ok(staging)
    })?;

    clear_queue.send(Box::new(staging))?;

    Ok((ix_buf, ix_count))
}

pub fn cube(buf: &mut Vec<[u8; 40]>) {
    buf.clear();

    let vx = |[x, y, z]: [f32; 3]| {
        let mut v0 = [0u8; 40];
        v0[0..12].clone_from_slice(bytemuck::cast_slice(&[x, y, z]));
        v0[12..24].clone_from_slice(bytemuck::cast_slice(&[1f32, 0.0, 0.0]));
        v0[24..40].clone_from_slice(bytemuck::cast_slice(&[x, y, z, 1.0]));
        v0
    };

    buf.extend(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ]
        .into_iter()
        .map(vx),
    );

    // fn vertex(x: f32, y: f32, rgb: [f32; 3]) -> [u8; 40] {
    //     let mut v = [0u8; 40];

    //     vertex[0..12].clone_from_slice

    //     v
    // }
}

/*
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Index(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Size(pub usize);

//

pub struct Vertex {
    id: usize,
    pos: Vec3,

    halfedge: HalfedgeRef,
    // on_boundary: bool,
    // degree: usize,
}

impl Vertex {
    pub fn on_boundary(&self) -> bool {
        todo!();
    }

    pub fn degree(&self) -> usize {
        todo!();
    }

    pub fn normal(&self) -> Vec3 {
        todo!();
    }

    pub fn center(&self) -> Vec3 {
        todo!();
    }

    pub fn neighborhood_center(&self) -> Vec3 {
        todo!();
    }

    pub fn id(&self) -> usize {
        todo!();
    }
}

pub struct Edge {
    //
}

pub struct Face {
    //
}

pub struct Halfedge {
    //
}

*/

fn allocate_uniform_desc_set(
    res: &mut GpuResources,
    buffer: BufferIx,
) -> anyhow::Result<vk::DescriptorSet> {
    // TODO also do uniforms if/when i add them, or keep them in a
    // separate set
    let layout_info = {
        let mut info = DescriptorLayoutInfo::default();

        let binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        info.bindings.push(binding);
        info
    };

    let set_info = {
        let info = DescriptorInfo {
            ty: rspirv_reflect::DescriptorType::UNIFORM_BUFFER,
            binding_count: rspirv_reflect::BindingCount::One,
            name: "camera".to_string(),
        };

        Some((0u32, info)).into_iter().collect::<BTreeMap<_, _>>()
    };

    res.allocate_desc_set_raw(&layout_info, &set_info, |res, builder| {
        let buffer = &res[buffer];
        let info = ash::vk::DescriptorBufferInfo::builder()
            .buffer(buffer.buffer)
            .offset(0)
            .range(ash::vk::WHOLE_SIZE)
            .build();
        let buffer_info = [info];
        builder.bind_buffer(0, &buffer_info);
        Ok(())
    })
}
