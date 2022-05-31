use nalgebra_glm::{vec3, Vec2};
use raving::compositor::label_space::LabelSpace;
use raving::compositor::{Compositor, SublayerAllocMsg};
use raving::script::console::frame::Resolvable;
use raving::vk::{
    BufferIx, BufferRes, DescSetIx, FenceIx, ImageIx, ImageViewIx, SemaphoreIx,
    VkEngine, WindowResources,
};

use ash::vk;

use crossbeam::atomic::AtomicCell;
use parking_lot::{Mutex, RwLock};

use flexi_logger::{Duplicate, FileSpec, Logger};

use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::{event_loop::EventLoop, window::WindowBuilder};

use std::collections::HashMap;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};

use rand::prelude::*;

use argh::FromArgs;
use std::path::PathBuf;

#[derive(FromArgs)]
/// Viewer arguments.
struct Args {
    // /// image path to display,
    // #[argh(positional)]
    // pub img_path: PathBuf,
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();

    let spec = "debug";
    let _logger = Logger::try_with_env_or_str(spec)?
        .log_to_file(FileSpec::default().suppress_timestamp())
        .duplicate_to_stderr(Duplicate::Debug)
        .start()?;

    let event_loop: EventLoop<()>;

    #[cfg(target_os = "linux")]
    {
        use winit::platform::unix::EventLoopExtUnix;
        log::debug!("Using X11 event loop");
        event_loop = EventLoop::new_x11()?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        log::debug!("Using default event loop");
        event_loop = EventLoop::new();
    }

    let width = 800u32;
    let height = 600u32;

    let swapchain_dims = Arc::new(AtomicCell::new([width, height]));

    let window = {
        WindowBuilder::new()
            .with_title("Raving Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .build(&event_loop)?
    };

    let mut engine = VkEngine::new(&window)?;

    let mut camera = raving_viz::mesh::Camera::new(&mut engine)?;

    camera.write_uniform(&mut engine.resources, [width as f32, height as f32]);

    let (clear_queue_tx, clear_queue_rx) =
        crossbeam::channel::unbounded::<Box<dyn std::any::Any + Send + Sync>>();

    /*
    let image = {
        use image::io::Reader as ImageReader;
        let img = ImageReader::open(&args.img_path)?;
        let img_data = img.decode()?;

        let img_name = args.img_path.file_stem().unwrap().to_str().unwrap();

        let image = engine.with_allocators(|ctx, res, alloc| {
            let usage = vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC;

            let img = res.allocate_image(
                ctx,
                alloc,
                img_data.width(),
                img_data.height(),
                vk::Format::R8G8B8A8_UNORM,
                usage,
                Some(img_name),
            )?;

            let img = res.insert_image(img);

            Ok(img)
        })?;

        let vk_img = engine.resources[image].image;

        let staging = engine.submit_queue_fn(|ctx, res, alloc, cmd| {
            VkEngine::transition_image(
                cmd,
                ctx.device(),
                vk_img,
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            let bytes = img_data.to_rgba8();
            let pixel_bytes =
                bytes.enumerate_pixels().flat_map(|(_, _, col)| {
                    let [r, g, b, a] = col.0;
                    [r, g, b, a].into_iter()
                });

            let staging = res[image].fill_from_pixels(
                ctx.device(),
                ctx,
                alloc,
                pixel_bytes,
                4,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                cmd,
            )?;

            VkEngine::transition_image(
                cmd,
                ctx.device(),
                vk_img,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
                vk::AccessFlags::NONE,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            );

            Ok(staging)
        })?;

        clear_queue_tx.send(Box::new(staging))?;

        image
    };
    */

    let mut compositor = Compositor::init(
        &mut engine,
        &swapchain_dims,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
    )?;

    raving_viz::sublayers::add_sublayer_defs(&mut engine, &mut compositor)?;

    {
        compositor.new_layer("main_layer", 0, true);

        compositor.sublayer_alloc_tx.send(SublayerAllocMsg::new(
            "main_layer",
            "rects",
            "rect-rgb",
            &[],
        ))?;

        compositor.sublayer_alloc_tx.send(SublayerAllocMsg::new(
            "main_layer",
            "lines",
            "line-rgb",
            &[],
        ))?;

        compositor.sublayer_alloc_tx.send(SublayerAllocMsg::new(
            "main_layer",
            "triangles",
            "tri-3d",
            &[camera.desc_set],
        ))?;

        compositor.allocate_sublayers(&mut engine)?;

        let mut vertices = Vec::new();

        let indices = raving_viz::mesh::sampled_disc(
            &mut engine,
            &clear_queue_tx,
            &mut vertices,
            1000,
        )?;
        /*
        let indices = raving_viz::mesh::index_buffer(
            &mut engine,
            &clear_queue_tx,
            [
                0, 1, 2, 3, 2, 1, // front
                5, 4, 7, 6, 7, 4, // back
                1, 5, 3, 7, 3, 5, // right
                4, 0, 6, 2, 6, 0, // left
                4, 5, 0, 1, 0, 5, // top
                2, 3, 6, 7, 6, 3, // bottom
            ], // 0..10,
        )?;
        */

        compositor.with_layer("main_layer", |layer| {
            if let Some(sublayer) = layer.get_sublayer_mut("triangles") {
                // if let Some(sublayer) = layer.get_sublayer_mut("lines") {

                // raving_viz::mesh::cube(&mut vertices);

                sublayer.draw_data_mut().try_for_each(|data| {
                    data.set_indices(Some(indices));
                    data.update_vertices_array(vertices.iter().copied())
                })?;
            }

            Ok(())
        })?;

        /*
        let target = nalgebra::Vector2::new(0.5f32, 0.5);

        raving_viz::vector_field::vector_field_vertices(
            width as f32,
            height as f32,
            32,
            32,
            0.0,
            [1.0, 0.0, 0.0, 1.0],
            &mut vertices,
            |p| {
                //
                target - p
            },
        );

        compositor.with_layer("main_layer", |layer| {
            if let Some(sublayer) = layer.get_sublayer_mut("lines") {
                sublayer.update_vertices_array(vertices)?;
            }

            Ok(())
        })?;
        */
    }

    // let mut main_layer = compositor.new_layer(name, depth, enabled)

    let mut recreate_swapchain = false;

    let should_exit = Arc::new(AtomicCell::new(false));

    {
        let exit = should_exit.clone();
        ctrlc::set_handler(move || {
            exit.store(true);
        })?;
    }

    let mut vertices: Vec<[u8; 40]> = Vec::new();

    {
        let x_dist = rand_distr::Normal::from_mean_cv(0.5, 0.3)?;
        let y_dist = rand_distr::Normal::from_mean_cv(0.5, 0.3)?;

        let points = x_dist
            .sample_iter(rand::thread_rng())
            .zip(y_dist.sample_iter(rand::thread_rng()))
            .map(|(x, y)| Vec2::new(x, y));

        raving_viz::vector_field::dot_plot(
            width as f32,
            height as f32,
            [1.0, 0.0, 0.0, 1.0],
            &mut vertices,
            points.take(1000),
        );
    }

    let start = std::time::Instant::now();
    let mut last_frame_i = std::time::Instant::now();

    let mut dt = 0.0;

    let mut frame_i = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                if frame_i % 1000 == 0 {
                    // log::error!("
                }

                dt = last_frame_i.elapsed().as_secs_f32();
                last_frame_i = std::time::Instant::now();

                if let Err(e) = compositor.allocate_sublayers(&mut engine) {
                    log::error!("Compositor error: {:?}", e);
                }

                if let Err(e) = compositor.write_layers(&mut engine.resources) {
                    log::error!("Compositor error: {:?}", e);
                }

                empty_clear_queue(&mut engine, &clear_queue_rx).unwrap();

                if recreate_swapchain {
                    let size = window.inner_size();

                    if size.width != 0 && size.height != 0 {
                        log::debug!(
                            "Recreating swapchain with window size {:?}",
                            size
                        );

                        engine
                            .recreate_swapchain(Some([size.width, size.height]))
                            .unwrap();

                        swapchain_dims.store(engine.swapchain_dimensions());

                        /*
                        {
                            let [width, height] = swapchain_dims.load();

                            let x_dist =
                                rand_distr::Normal::from_mean_cv(0.5, 0.3)
                                    .unwrap();
                            let y_dist =
                                rand_distr::Normal::from_mean_cv(0.5, 0.3)
                                    .unwrap();

                            let points = x_dist
                                .sample_iter(rand::thread_rng())
                                .zip(y_dist.sample_iter(rand::thread_rng()))
                                .map(|(x, y)| Vec2::new(x, y));

                            raving_viz::vector_field::dot_plot(
                                width as f32,
                                height as f32,
                                [1.0, 0.0, 0.0, 1.0],
                                &mut vertices,
                                points.take(1000),
                            );
                        }
                        */

                        recreate_swapchain = false;
                    }
                }

                let [width, height] = swapchain_dims.load();

                /*
                raving_viz::vector_field::vector_field_vertices(
                    width as f32,
                    height as f32,
                    32,
                    32,
                    start.elapsed().as_secs_f32(),
                    [1.0, 0.0, 0.0, 1.0],
                    &mut vertices,
                    |p| target - p,
                );
                */

                /*
                compositor
                    .with_layer("main_layer", |layer| {
                        if let Some(sublayer) = layer.get_sublayer_mut("lines")
                        {
                            sublayer.update_vertices_array(
                                vertices.iter().copied(),
                            )?;
                        }

                        Ok(())
                    })
                    .unwrap();
                */

                {
                    let t = start.elapsed().as_secs_f32();

                    let r = 10.0;

                    let x = t.cos() * r;
                    let y = t.sin() * r;

                    camera.write_uniform_fixed(
                        &mut engine.resources,
                        // vec3(10.0, -4.0, 10.0),
                        vec3(x, 3.0, y),
                        vec3(0.0, 0.5, 0.0),
                        // vec3(0.5, 0.5, 0.5),
                        [width as f32, height as f32],
                    );
                }

                // camera.write_uniform(
                //     &mut engine.resources,
                //     [width as f32, height as f32],
                // );

                if let Ok((img, view)) = engine.draw_compositor(
                    &compositor,
                    [0.3, 0.3, 0.3],
                    width,
                    height,
                ) {
                    match engine.display_image(
                        img,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    ) {
                        Ok(_) => {
                            frame_i += 1;
                        }
                        Err(e) => {
                            log::error!("display_image error: {:?}", e);
                            recreate_swapchain = true;
                        }
                    }

                    clear_queue_tx.send(Box::new(view)).unwrap();
                    clear_queue_tx.send(Box::new(img)).unwrap();
                }
            }
            Event::RedrawEventsCleared => {}
            Event::DeviceEvent { event, .. } => {
                if let winit::event::DeviceEvent::MouseMotion { delta } = event
                {
                    // camera.rotate_hor(delta.0 as f32 / 500.0);
                    // camera.rotate_ver(-delta.1 as f32 / 1000.0);
                }
            }
            Event::WindowEvent { event, .. } => {
                // viewer.handle_input(&console, &event);

                match event {
                    // WindowEvent::ModifiersChanged(mod_state) => {
                    //     waragraph::input::set_modifiers(mod_state);
                    // }
                    // WindowEvent::ReceivedCharacter(c) => {
                    //     if !c.is_ascii_control() && c.is_ascii() {
                    //         console
                    //             .handle_input(&db, &buffers, ConsoleInput::AppendChar(c))
                    //             .unwrap();
                    //     }
                    // }
                    WindowEvent::MouseInput { button, state, .. } => {
                        if button == winit::event::MouseButton::Left
                            && state == winit::event::ElementState::Pressed
                        // && !mouse_clicked
                        {
                            log::error!("mouse clicked!");
                            // mouse_clicked = true;
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        /*
                        let [width, height] = swapchain_dims.load();
                        let x = position.x as f32 / width as f32;
                        let y = position.y as f32 / height as f32;

                        target = [x, y].into();
                        */
                    }

                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(kc) = input.virtual_keycode {
                            use VirtualKeyCode as VK;

                            if input.state
                                == winit::event::ElementState::Pressed
                            {
                                /*
                                if matches!(kc, VK::Up) {
                                    camera.move_fwd(dt, 10.1);
                                } else if matches!(kc, VK::Down) {
                                    camera.move_fwd(dt, -10.1);
                                } else if matches!(kc, VK::Left) {
                                    camera.move_right(dt, -1.1);
                                } else if matches!(kc, VK::Right) {
                                    camera.move_right(dt, 1.1);
                                }
                                */
                                /*
                                if matches!(kc, VK::Left) {
                                    let rot = nalgebra_glm::rotate(
                                        &camera.mat,
                                        0.2,
                                        &nalgebra_glm::vec3(0f32, 1.0, 0.0),
                                    );

                                    camera.mat = rot;

                                    camera.write_uniform(&mut engine.resources);
                                } else if matches!(kc, VK::Right) {
                                    let rot = nalgebra_glm::rotate(
                                        &camera.mat,
                                        -0.2,
                                        &nalgebra_glm::vec3(0f32, 1.0, 0.0),
                                    );

                                    camera.mat = rot;

                                    camera.write_uniform(&mut engine.resources);
                                }
                                */
                            }
                        }
                    }
                    WindowEvent::CloseRequested => {
                        log::debug!("WindowEvent::CloseRequested");
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    WindowEvent::Resized { .. } => {
                        recreate_swapchain = true;
                    }
                    _ => (),
                }
            }
            Event::LoopDestroyed => {
                log::debug!("Event::LoopDestroyed");
                log::debug!("Freeing resources");

                // let _ = clipboard;

                unsafe {
                    let queue = engine.queues.thread.queue;
                    engine.context.device().queue_wait_idle(queue).unwrap();
                };

                empty_clear_queue(&mut engine, &clear_queue_rx).unwrap();

                let ctx = &engine.context;
                let res = &mut engine.resources;
                let alloc = &mut engine.allocator;

                res.cleanup(ctx, alloc).unwrap();
            }
            _ => (),
        }

        if should_exit.load() {
            log::debug!("Ctrl-C received, exiting");
            *control_flow = winit::event_loop::ControlFlow::Exit;
        }
    });

    Ok(())
}

fn empty_clear_queue(
    engine: &mut VkEngine,
    clear_queue_rx: &crossbeam::channel::Receiver<
        Box<dyn std::any::Any + Send + Sync>,
    >,
) -> Result<()> {
    while let Ok(val) = clear_queue_rx.try_recv() {
        if val.type_id() == std::any::TypeId::of::<BufferRes>() {
            log::warn!("freeing buffer");
            let buf = val.downcast::<BufferRes>().unwrap();
            buf.cleanup(&engine.context, &mut engine.allocator)?;
        } else if val.type_id() == std::any::TypeId::of::<BufferIx>() {
            log::warn!("freeing buffer");
            let ix = *val.downcast::<BufferIx>().unwrap();
            engine.resources.destroy_buffer(
                &engine.context,
                &mut engine.allocator,
                ix,
            )?;
        } else if val.type_id() == std::any::TypeId::of::<ImageViewIx>() {
            // log::warn!("destroying image view");
            let ix = *val.downcast::<ImageViewIx>().unwrap();
            engine.resources.destroy_image_view(&engine.context, ix);
        } else if val.type_id() == std::any::TypeId::of::<ImageIx>() {
            // log::warn!("freeing image");
            let ix = *val.downcast::<ImageIx>().unwrap();
            engine.resources.destroy_image(
                &engine.context,
                &mut engine.allocator,
                ix,
            )?
        }
    }

    Ok(())
}
