use raving::compositor::label_space::LabelSpace;
use raving::compositor::{Compositor, SublayerAllocMsg};
use raving::script::console::frame::Resolvable;
use raving::vk::{
    DescSetIx, FenceIx, ImageIx, ImageViewIx, SemaphoreIx, VkEngine,
    WindowResources,
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
    /// image path to display,
    #[argh(positional)]
    pub img_path: PathBuf,
}

fn main() -> Result<()> {
    // let args: Args = argh::from_env();

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

        let image_res = &engine.resources[image];

        let vk_img = engine.resources[image].image;

        engine.submit_queue_fn(|device, cmd| {
            VkEngine::transition_image(
                cmd,
                device,
                vk_img,
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            let pixel_bytes = img_data.to_rgba8().enumerate_pixels().flat_map(
                |(_, _, col)| {
                    let [r, g, b, a] = col.0;
                    [r, g, b, a].into_iter()
                },
            );

            // let staging = img.fill_from_pixels(
            //     device,
            //     ctx,
            //     alloc,
            //     pixel_bytes,
            //     4,
            //     vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            //     cmd,
            // )?;

            Ok(())
        })?;

        img
    };
    */

    let mut compositor = Compositor::init(
        &mut engine,
        &swapchain_dims,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
    )?;

    game_raving::sublayers::add_sublayer_defs(&mut engine, &mut compositor)?;

    {
        compositor.new_layer("main_layer", 0, true);

        compositor.sublayer_alloc_tx.send(SublayerAllocMsg::new(
            "main_layer",
            "rects",
            "rect-rgb",
            &[],
        ))?;

        compositor.allocate_sublayers(&mut engine)?;

        compositor.with_layer("main_layer", |layer| {
            if let Some(sublayer) = layer.get_sublayer_mut("rects") {
                let mut vert = [0u8; 4 * 8];
                vert.clone_from_slice(bytemuck::cast_slice(&[
                    100.0f32, 100.0, 200.0, 100.0, 1.0, 0.0, 0.0, 1.0,
                ]));

                sublayer.update_vertices_array(Some(vert))?;
            }

            Ok(())
        })?;
    }
    // let mut main_layer = compositor.new_layer(name, depth, enabled)

    let mut recreate_swapchain = false;
    // let mut sync_objs: Option<(FenceIx, SemaphoreIx)> = None;
    let mut sync_objs: Option<FenceIx> = None;

    let mut clear_queue: Vec<Box<dyn std::any::Any>> = Vec::new();

    let should_exit = Arc::new(AtomicCell::new(false));

    {
        let exit = should_exit.clone();
        ctrlc::set_handler(move || {
            exit.store(true);
        })?;
    }

    println!("Hello, world!");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                for val in clear_queue.drain(..) {
                    if val.type_id() == std::any::TypeId::of::<ImageViewIx>() {
                        let ix = *val.downcast::<ImageViewIx>().unwrap();
                        engine
                            .resources
                            .destroy_image_view(&engine.context, ix);
                    } else if val.type_id() == std::any::TypeId::of::<ImageIx>()
                    {
                        let ix = *val.downcast::<ImageIx>().unwrap();
                        engine
                            .resources
                            .destroy_image(
                                &engine.context,
                                &mut engine.allocator,
                                ix,
                            )
                            .unwrap();
                    }
                }

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

                        recreate_swapchain = false;
                    }
                }

                let [width, height] = swapchain_dims.load();

                if let Ok((img, view)) = engine.draw_compositor(
                    &compositor,
                    [0.3, 0.3, 0.3],
                    width,
                    height,
                ) {
                    match engine.display_image(
                        sync_objs,
                        img,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    ) {
                        Ok(Some(objs)) => {
                            sync_objs = Some(objs);
                        }
                        Ok(None) => {
                            recreate_swapchain = true;
                        }
                        Err(e) => {
                            log::error!("display_image error: {:?}", e);
                            recreate_swapchain = true;
                        }
                    }

                    // clear_queue.push(Box::new(view));
                    // clear_queue.push(Box::new(img));
                }
            }
            Event::RedrawEventsCleared => {}
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
                        // waragraph::input::set_mouse_pos(position.x, position.y);
                    }
                    /*
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(kc) = input.virtual_keycode {
                            use VirtualKeyCode as VK;

                            if input.state == winit::event::ElementState::Pressed {
                                if matches!(kc, VK::Space) {
                                    if let Some(labels) = label_stacks.as_mut() {
                                        let [width, _] = swapchain_dims.load();
                                        let [slot_offset, slot_width] =
                                            viewer.slot_x_offsets(width);

                                        labels
                                            .update_layer(
                                                &mut compositor,
                                                &graph,
                                                viewer.view.load(),
                                                slot_offset,
                                                slot_width,
                                            )
                                            .unwrap();
                                    }
                                }
                                if matches!(kc, VK::Return) {
                                    if let Err(e) =
                                        console.handle_input(&db, &buffers, ConsoleInput::Submit)
                                    {
                                        log::error!("Console error: {:?}", e);
                                    }
                                } else if matches!(kc, VK::Back) {
                                    console
                                        .handle_input(&db, &buffers, ConsoleInput::Backspace)
                                        .unwrap();
                                }
                            }
                        }
                    }
                    */
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
