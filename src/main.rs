use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::device::{Device, DeviceExtensions};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain;
use vulkano::swapchain::{
    AcquireError, PresentMode, SurfaceTransform, Swapchain, SwapchainCreationError,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};

use vulkano_win::VkSurfaceBuild;

use winit::{Event, EventsLoop, Window, WindowBuilder, WindowEvent};

use rand::Rng;
use std::sync::Arc;

fn main() {
    let mut rng = rand::thread_rng();
    println!("Float: {}", rng.gen_range(-1.0, 1.0));
    println!("Float: {}", rng.gen_range(-1.0, 1.0));
    // Create a Vulkan Instance and selecting extensions to enable
    let extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, &extensions, None).expect("failed to create instance");
    for (i, physical_device) in PhysicalDevice::enumerate(&instance).enumerate() {
        println!(
            "Device {}: Name: {}, type: {:?}",
            i,
            physical_device.name(),
            physical_device.ty()
        );
    }
    // Chose Physical Device to use
    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device available");

    println!(
        "Using device: {} (type: {:?})",
        physical.name(),
        physical.ty()
    );
    println!("Vulkan Version: {}", physical.api_version());
    // Create Event loop, Swapchain Surface and Window
    let mut events_loop = EventsLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&events_loop, instance.clone())
        .unwrap();

    let window = surface.window();
    window.set_title("J-Pong");
    // Choose GPU Queue to execute draw commands
    for family in physical.queue_families() {
        println!(
            "Found a queue family with {:?} queue(s)",
            family.queues_count()
        );
    }
    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    // Setting up the device, extensions are optional features and get the first queue
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = {
        Device::new(
            physical,
            physical.supported_features(),
            &device_extensions,
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("failed to create device")
    };

    let queue = queues.next().unwrap();
    let (mut swapchain, images) = {
        let caps = surface
            .capabilities(physical)
            .expect("failed to get surface capabilities");
        let usage = caps.supported_usage_flags;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        let initial_dimensions = if let Some(dimensions) = window.get_inner_size() {
            // convert to physical pixels
            let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            // The window no longer exists so exit the application.
            return;
        };
        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            initial_dimensions,
            1,
            usage,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            true,
            None,
        )
        .unwrap()
    };

    // Create the Vertex and Fragment Shaders

    mod vs_player1 {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "src/shaders/player1.vs"
        }
    }

    mod fs_player1 {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "src/shaders/player1.fs"
        }
    }

    mod vs_player2 {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "src/shaders/player2.vs"
        }
    }

    mod fs_player2 {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "src/shaders/player2.fs"
        }
    }

    mod vs_net {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "src/shaders/net.vs"
        }
    }

    mod fs_net {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "src/shaders/net.fs"
        }
    }

    let vs_player1 = vs_player1::Shader::load(device.clone()).unwrap();
    let fs_player1 = fs_player1::Shader::load(device.clone()).unwrap();
    let vs_player2 = vs_player2::Shader::load(device.clone()).unwrap();
    let fs_player2 = fs_player2::Shader::load(device.clone()).unwrap();
    let vs_net = vs_net::Shader::load(device.clone()).unwrap();
    let fs_net = fs_net::Shader::load(device.clone()).unwrap();
    // Create Render Pass
    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
    );
    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs_player1.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs_player1.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let pipeline_player2 = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs_player2.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs_player2.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let pipeline_net = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs_net.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs_net.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
        compare_mask: None,
        write_mask: None,
        reference: None,
    };
    let mut framebuffers =
        window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>;
    let mut displacement_amount = 0;
    let mut displacement_increment = false;
    let displacement_constant = 1;

    loop {
        if displacement_amount == 150 || displacement_amount == 0 {
            displacement_increment = !displacement_increment;
        }
        if displacement_increment == true {
            displacement_amount += displacement_constant;
        }
        if displacement_increment == false {
            displacement_amount -= displacement_constant;
        }

        let pc_player1 = vs_player1::ty::Displacement {
            displacement: displacement_amount as f32 / 100.0,
        };

        let pc_player2 = vs_player2::ty::Displacement {
            displacement: -displacement_amount as f32 / 100.0,
        };
        let vertex_buffer_player1 = {
            #[derive(Default, Debug, Clone)]
            struct Vertex {
                position: [f32; 2],
                color: [f32; 3],
            }

            vulkano::impl_vertex!(Vertex, position, color);

            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                [
                    Vertex {
                        position: [-0.9, -1.0],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.8, -1.0],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.9, -0.5],
                        color: [0.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.8, -1.0],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.8, -0.5],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.9, -0.5],
                        color: [0.0, 1.0, 1.0],
                    },
                ]
                .iter()
                .cloned(),
            )
            .unwrap()
        };

        let vertex_buffer_player2 = {
            #[derive(Default, Debug, Clone)]
            struct Vertex {
                position: [f32; 2],
                colour: [f32; 3],
            }

            vulkano::impl_vertex!(Vertex, position, colour);

            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                [
                    Vertex {
                        position: [0.9, 1.0],
                        colour: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.8, 1.0],
                        colour: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.9, 0.5],
                        colour: [0.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.8, 1.0],
                        colour: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.8, 0.5],
                        colour: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.9, 0.5],
                        colour: [0.0, 1.0, 1.0],
                    },
                ]
                .iter()
                .cloned(),
            )
            .unwrap()
        };

        let vertex_buffer_net = {
            #[derive(Default, Debug, Clone)]
            struct Vertex {
                position: [f32; 2],
                color: [f32; 3],
            }

            vulkano::impl_vertex!(Vertex, position, color);

            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                [
                    Vertex {
                        position: [-0.005, -1.0],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.005, -1.0],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.005, 1.0],
                        color: [0.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.005, 1.0],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.005, 1.0],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.005, -1.0],
                        color: [0.0, 1.0, 1.0],
                    },
                ]
                .iter()
                .cloned(),
            )
            .unwrap()
        };

        let offset_buffer = {
            #[derive(Default, Debug, Clone)]
            struct Offset {
                position: [f32; 2],
            }

            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                [Offset {
                    position: [rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)],
                }]
                .iter()
                .cloned(),
            )
            .unwrap()
        };

        // Frees no longer needed resources
        previous_frame_end.cleanup_finished();
        // Window Resize: Recreate swapchain, framebuffer and viewport
        if recreate_swapchain {
            let dimensions = if let Some(dimensions) = window.get_inner_size() {
                let dimensions: (u32, u32) =
                    dimensions.to_physical(window.get_hidpi_factor()).into();
                [dimensions.0, dimensions.1]
            } else {
                return;
            };

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => continue,
                Err(err) => panic!("{:?}", err),
            };

            swapchain = new_swapchain;
            framebuffers =
                window_size_dependent_setup(&new_images, render_pass.clone(), &mut dynamic_state);

            recreate_swapchain = false;
        }
        // Aquire image from swapchain, blocks if no image available
        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    recreate_swapchain = true;
                    continue;
                }
                Err(err) => panic!("{:?}", err),
            };
        // Clear the screen with a colour
        let clear_values = vec![[0.0, 0.0, 0.0, 0.0].into()];

        // In order to draw, we have to build a *command buffer*.
        let command_buffer =
            AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                .unwrap()
                // Before we can draw, we have to *enter a render pass*.
                .begin_render_pass(framebuffers[image_num].clone(), false, clear_values)
                .unwrap()
                // The first subpass of the render pass: We add a draw command.
                .draw(
                    pipeline_net.clone(),
                    &dynamic_state,
                    vertex_buffer_net.clone(),
                    (),
                    (),
                )
                .unwrap()
                .draw(
                    pipeline.clone(),
                    &dynamic_state,
                    vertex_buffer_player1.clone(),
                    (),
                    pc_player1,
                )
                .unwrap()
                .draw(
                    pipeline_player2.clone(),
                    &dynamic_state,
                    vertex_buffer_player2.clone(),
                    (),
                    pc_player2,
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                // Finish building the command buffer by calling `build`.
                .build()
                .unwrap();

        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            // The color output is now expected to contain our triangle. But in order to show it on
            // the screen, we have to *present* the image by calling `present`.
            // This function does not actually present the image immediately. Instead it submits a
            // present command at the end of the queue. This means that it will only be presented once
            // the GPU has finished executing the command buffer that draws the triangle.
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
        }

        let mut done = false;
        events_loop.poll_events(|ev| match ev {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => done = true,
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => recreate_swapchain = true,
            _ => (),
        });
        if done {
            return;
        }
    }
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
