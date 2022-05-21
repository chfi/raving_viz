use crossbeam::atomic::AtomicCell;
use parking_lot::RwLock;
use raving::vk::context::VkContext;
use raving::vk::{
    BufferIx, DescSetIx, FramebufferIx, GpuResources, PipelineIx, RenderPassIx, ShaderIx, VkEngine,
};

use raving::compositor::*;
use raving::vk::resource::WindowResources;

use ash::{vk, Device};

use rhai::plugin::RhaiResult;
use rustc_hash::{FxHashMap, FxHashSet};
use winit::event::VirtualKeyCode;
use winit::window::Window;

use std::collections::{BTreeMap, HashMap};

use std::sync::Arc;

use anyhow::{anyhow, bail, Result};

// use zerocopy::{AsBytes, FromBytes};

use rhai::plugin::*;

pub fn add_sublayer_defs(engine: &mut VkEngine, compositor: &mut Compositor) -> Result<()> {
    engine.with_allocators(|ctx, res, _| {
        let clear_pass = res[compositor.clear_pass];
        let load_pass = res[compositor.load_pass];
        compositor.add_sublayer_defs([
            tri_3d_sublayer(ctx, res, clear_pass, load_pass)?,
            rect_rgb_sublayer(ctx, res, clear_pass, load_pass)?,
            line_rgb_sublayer(ctx, res, clear_pass, load_pass)?,
        ]);

        Ok(())
    })
}

pub fn tri_3d_sublayer(
    ctx: &VkContext,
    res: &mut GpuResources,
    clear_pass: vk::RenderPass,
    load_pass: vk::RenderPass,
) -> Result<SublayerDef> {
    let vert = res.load_shader("shaders/rect_window.vert.spv", vk::ShaderStageFlags::VERTEX)?;
    let frag = res.load_shader(
        "shaders/rect_window.frag.spv",
        vk::ShaderStageFlags::FRAGMENT,
    )?;

    let vert = res.insert_shader(vert);
    let frag = res.insert_shader(frag);

    let vertex_stride = std::mem::size_of::<[f32; 10]>();

    let vert_binding_desc = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(vertex_stride as u32)
        .input_rate(vk::VertexInputRate::INSTANCE)
        .build();

    let pos_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(0)
        .format(vk::Format::R32G32B32_SFLOAT)
        .offset(0)
        .build();

    let norm_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(1)
        .format(vk::Format::R32G32B32_SFLOAT)
        .offset(12)
        .build();

    let color_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(2)
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .offset(24)
        .build();

    let vert_binding_descs = [vert_binding_desc];
    let vert_attr_descs = [pos_desc, norm_desc, color_desc];

    let vert_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vert_binding_descs)
        .vertex_attribute_descriptions(&vert_attr_descs);

    let vertex_offset = 0;
    let vertex_stride = vertex_stride as usize;

    SublayerDef::new::<([f32; 3], [f32; 3], [f32; 4]), _>(
        ctx,
        res,
        "tri-3d",
        vert,
        frag,
        clear_pass,
        load_pass,
        vertex_offset,
        vertex_stride,
        false,
        None,
        Some(1),
        vert_input_info,
        None,
    )
}

pub fn rect_rgb_sublayer(
    ctx: &VkContext,
    res: &mut GpuResources,
    clear_pass: vk::RenderPass,
    load_pass: vk::RenderPass,
) -> Result<SublayerDef> {
    let vert = res.load_shader("shaders/rect_window.vert.spv", vk::ShaderStageFlags::VERTEX)?;
    let frag = res.load_shader(
        "shaders/rect_window.frag.spv",
        vk::ShaderStageFlags::FRAGMENT,
    )?;

    let vert = res.insert_shader(vert);
    let frag = res.insert_shader(frag);

    let vert_binding_desc = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(std::mem::size_of::<[f32; 8]>() as u32)
        .input_rate(vk::VertexInputRate::INSTANCE)
        .build();

    let pos_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(0)
        .format(vk::Format::R32G32_SFLOAT)
        .offset(0)
        .build();

    let size_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(1)
        .format(vk::Format::R32G32_SFLOAT)
        .offset(8)
        .build();

    let color_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(2)
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .offset(16)
        .build();

    let vert_binding_descs = [vert_binding_desc];
    let vert_attr_descs = [pos_desc, size_desc, color_desc];

    let vert_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vert_binding_descs)
        .vertex_attribute_descriptions(&vert_attr_descs);

    let vertex_offset = 0;
    let vertex_stride = 32;

    SublayerDef::new::<([f32; 2], [f32; 2], [f32; 4]), _>(
        ctx,
        res,
        "rect-rgb",
        vert,
        frag,
        clear_pass,
        load_pass,
        vertex_offset,
        vertex_stride,
        true,
        Some(6),
        None,
        vert_input_info,
        None,
    )
}

pub fn line_rgb_sublayer(
    ctx: &VkContext,
    res: &mut GpuResources,
    clear_pass: vk::RenderPass,
    load_pass: vk::RenderPass,
) -> Result<SublayerDef> {
    let vert = res.load_shader("shaders/vector.vert.spv", vk::ShaderStageFlags::VERTEX)?;
    let frag = res.load_shader("shaders/vector.frag.spv", vk::ShaderStageFlags::FRAGMENT)?;

    let vert = res.insert_shader(vert);
    let frag = res.insert_shader(frag);

    let vertex_size = std::mem::size_of::<[f32; 10]>() as u32;

    let vert_binding_desc = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(vertex_size)
        .input_rate(vk::VertexInputRate::INSTANCE)
        .build();

    let p0_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(0)
        .format(vk::Format::R32G32B32_SFLOAT)
        .offset(0)
        .build();

    let p1_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(1)
        .format(vk::Format::R32G32B32_SFLOAT)
        .offset(12)
        .build();

    let color_desc = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(2)
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .offset(24)
        .build();

    let vert_binding_descs = [vert_binding_desc];
    let vert_attr_descs = [p0_desc, p1_desc, color_desc];

    let vert_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vert_binding_descs)
        .vertex_attribute_descriptions(&vert_attr_descs);

    let vertex_offset = 0;
    let vertex_stride = vertex_size as usize;

    SublayerDef::new::<([f32; 3], [f32; 3], [f32; 4]), _>(
        ctx,
        res,
        "line-rgb",
        vert,
        frag,
        clear_pass,
        load_pass,
        vertex_offset,
        vertex_stride,
        true,
        Some(6),
        None,
        vert_input_info,
        None,
    )
}
