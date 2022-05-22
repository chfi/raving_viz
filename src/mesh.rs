use ash::vk;

use nalgebra::Point3;
use nalgebra_glm::Vec3;
use raving::vk::{BufferIx, VkEngine};

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
