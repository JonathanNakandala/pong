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
use winit::{
    ElementState, Event, EventsLoop, KeyboardInput, VirtualKeyCode, Window, WindowBuilder,
    WindowEvent,
};

use std::sync::Arc;
use std::time::{Duration, Instant};
use vulkano_text::{DrawText, DrawTextTrait};

fn main() {
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
    mod vs_ball {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "src/shaders/ball.vs"
        }
    }

    mod fs_ball {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "src/shaders/ball.fs"
        }
    }
    let vs_player1 = vs_player1::Shader::load(device.clone()).unwrap();
    let fs_player1 = fs_player1::Shader::load(device.clone()).unwrap();
    let vs_player2 = vs_player2::Shader::load(device.clone()).unwrap();
    let fs_player2 = fs_player2::Shader::load(device.clone()).unwrap();
    let vs_net = vs_net::Shader::load(device.clone()).unwrap();
    let fs_net = fs_net::Shader::load(device.clone()).unwrap();
    let vs_ball = vs_ball::Shader::load(device.clone()).unwrap();
    let fs_ball = fs_ball::Shader::load(device.clone()).unwrap();
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

    let mut draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);

    let (width, _): (u32, u32) = surface.window().get_inner_size().unwrap().into();
    let mut x = -200.0;

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

    let pipeline_ball = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs_ball.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs_ball.main_entry_point(), ())
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
    let mut player_1_displacement = 0;
    let mut player_2_displacement = 0;
    let mut player_1_displacement_velocity: i32 = 0;
    let mut player_2_displacement_velocity: i32 = 0;
    let mut displacement_increment = false;
    let mut ball_displacement_x_increment = false;
    let mut ball_displacement_y_increment = false;
    let displacement_x_constant = 1;
    let displacement_y_constant = 1;

    let mut ball_displacement: [i32; 2] = [0; 2];
    /*     fn displace_player(direction: String, displacement: i32) -> i32 {
        if (displacement == 150 && direction == "Down") || (displacement == 0 && direction == "Up")
        {
            return displacement;
        }
        if direction == "Up" {
            return displacement - 10;
        } else if direction == "Down" {
            return displacement + 10;
        };
        displacement
    } */

    let mut score_player1: u8 = 0;
    let mut score_player2: u8 = 0;
    // Just experimentally found the position for the ball displacement so that it intersects with the paddle
    let paddle_y_player1 = -77;
    let paddle_y_player2 = 77;
    let wall_y_top = -97;
    let wall_y_bottom = 97;
    let single_player = false;
    // How long to hold the text of the winner for
    let mut hold_text_time = 0;
    let mut theres_a_winner = false;
    let mut time_is_set = false;
    let mut time: Instant = Instant::now();

    loop {
        // Ball movement x axis
        if ball_displacement[0] == -100 || ball_displacement[0] == 100 {
            ball_displacement_x_increment = !ball_displacement_x_increment;
        }

        if ball_displacement_x_increment {
            ball_displacement[0] += displacement_x_constant;
        }
        if !ball_displacement_x_increment {
            ball_displacement[0] -= displacement_x_constant;
        }
        // Ball Movement y axis
        if ball_displacement[1] == wall_y_top || ball_displacement[1] == wall_y_bottom {
            ball_displacement_y_increment = !ball_displacement_y_increment;
        }

        if ball_displacement_y_increment {
            ball_displacement[1] += displacement_y_constant;
        }
        if !ball_displacement_y_increment {
            ball_displacement[1] -= displacement_y_constant;
        }

        // Point Scoring
        if ball_displacement[0] == -100 {
            score_player2 += 1;
            ball_displacement = [0, 0];
        }
        if ball_displacement[0] == 100 {
            score_player1 += 1;
            ball_displacement = [0, 0];
        }
        //

        // Auto Move Player
        if single_player {
            if player_2_displacement == 150 || player_2_displacement == 0 {
                displacement_increment = !displacement_increment;
            }
            if displacement_increment {
                player_2_displacement += displacement_x_constant;
            }
            if !displacement_increment {
                player_2_displacement -= displacement_x_constant;
            }
        }
        // Smooth Paddle Movement
        if player_1_displacement_velocity != 0 {
            if player_1_displacement_velocity.is_positive() {
                player_1_displacement += player_1_displacement_velocity;
                player_1_displacement_velocity -= 1;
                if player_1_displacement > 150 {
                    player_1_displacement = 150;
                }
            }
            if player_1_displacement_velocity.is_negative() {
                player_1_displacement += player_1_displacement_velocity;
                player_1_displacement_velocity += 1;
                if player_1_displacement < 0 {
                    player_1_displacement = 0;
                }
            }
        }
        if player_2_displacement_velocity != 0 {
            if player_2_displacement_velocity.is_positive() {
                player_2_displacement += player_2_displacement_velocity;
                player_2_displacement_velocity -= 1;
                if player_2_displacement > 150 {
                    player_2_displacement = 150;
                }
            }
            if player_2_displacement_velocity.is_negative() {
                player_2_displacement += player_2_displacement_velocity;
                player_2_displacement_velocity += 1;
                if player_2_displacement < 0 {
                    player_2_displacement = 0;
                }
            }
        }

        let paddle_surface_player1: [f32; 2] = [-1.0, -0.5];
        let paddle_surface_player2: [f32; 2] = [1.0, 0.5];
        let paddle_surface_position_player1: [f32; 2] = [
            paddle_surface_player1[0] + player_1_displacement as f32 / 100.0,
            paddle_surface_player1[1] + player_1_displacement as f32 / 100.0,
        ];
        let paddle_surface_position_player2: [f32; 2] = [
            paddle_surface_player2[0] - player_2_displacement as f32 / 100.0,
            paddle_surface_player2[1] - player_2_displacement as f32 / 100.0,
        ];
        //Ball Bouncing off Paddle Logic
        if ball_displacement[0] == paddle_y_player1 {
            if ball_displacement[1] as f32 / 100.0 >= paddle_surface_position_player1[0]
                && ball_displacement[1] as f32 / 100.0 <= paddle_surface_position_player1[1]
            {
                ball_displacement_x_increment = !ball_displacement_x_increment;
            }
        }
        if ball_displacement[0] == paddle_y_player2 {
            if ball_displacement[1] as f32 / 100.0 >= paddle_surface_position_player2[1]
                && ball_displacement[1] as f32 / 100.0 <= paddle_surface_position_player2[0]
            {
                ball_displacement_x_increment = !ball_displacement_x_increment;
            }
        }
        //
        // GPU Push Constants
        let pc_player1 = vs_player1::ty::Displacement {
            displacement: player_1_displacement as f32 / 100.0,
        };

        let pc_player2 = vs_player2::ty::Displacement {
            displacement: -player_2_displacement as f32 / 100.0,
        };

        let pc_ball = vs_ball::ty::BallPosition {
            vector: [
                ball_displacement[0] as f32 / 100.0,
                ball_displacement[1] as f32 / 100.0,
            ],
        };

        if pc_ball.vector[0] < -1.0 {
            println!("{}", pc_ball.vector[0]);
        }

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
                        position: [-0.8, paddle_surface_player1[0]],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.9, paddle_surface_player1[1]],
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
                        position: [0.8, paddle_surface_player2[0]],
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
                        position: [0.8, paddle_surface_player2[1]],
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

        let vertex_buffer_ball = {
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
                        position: [-0.03, -0.03],
                        color: [1.0, 0.0, 1.0],
                    },
                    Vertex {
                        position: [0.03, -0.03],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.03, 0.03],
                        color: [0.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [-0.03, 0.03],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.03, 0.03],
                        color: [1.0, 1.0, 1.0],
                    },
                    Vertex {
                        position: [0.03, -0.03],
                        color: [0.0, 1.0, 1.0],
                    },
                ]
                .iter()
                .cloned(),
            )
            .unwrap()
        };

        if x > width as f32 {
            x = 0.0;
        } else {
            x += 0.4;
        }
        draw_text.queue_text(
            630.0,
            200.0,
            190.0,
            [0.0, 1.0, 1.0, 1.0],
            &score_player1.to_string(),
        );
        draw_text.queue_text(
            800.0,
            200.0,
            190.0,
            [0.0, 1.0, 1.0, 1.0],
            &score_player2.to_string(),
        );
        let score_to_win = 9;
        // Player Wins, Reset Score and let them know they won for a bit
        if (score_player1 == score_to_win || score_player2 == score_to_win) && !time_is_set {
            theres_a_winner = true;
            time = Instant::now();
            time_is_set = true;
        }
        if theres_a_winner {
            if score_player1 == score_to_win {
                draw_text.queue_text(800.0, 400.0, 150.0, [0.0, 1.0, 1.0, 1.0], "Player 1 Wins!");
            }
            if score_player2 == score_to_win {
                draw_text.queue_text(80.0, 400.0, 150.0, [0.0, 1.0, 1.0, 1.0], "Player 2 Wins!");
            }
            ball_displacement[0] = 0;
            ball_displacement[1] = 0;

            if time.elapsed() > Duration::from_secs(3) {
                score_player1 = 0;
                score_player2 = 0;
                theres_a_winner = false;
                time_is_set = false;
            }
        }
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
            draw_text = DrawText::new(
                device.clone(),
                queue.clone(),
                swapchain.clone(),
                &new_images,
            );

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
                // The subpasses of the render pass are each of these
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
                .draw(
                    pipeline_ball.clone(),
                    &dynamic_state,
                    vertex_buffer_ball.clone(),
                    (),
                    pc_ball,
                )
                .unwrap();
        let command_buffer = command_buffer
            .end_render_pass()
            .unwrap()
            .draw_text(&mut draw_text, image_num)
            .build()
            .unwrap();

        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
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
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::W),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                //player_1_displacement = displace_player("Up".to_owned(), player_1_displacement);
                player_1_displacement_velocity -= 2;
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::W),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                player_1_displacement_velocity = -4;
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::S),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                //player_1_displacement = displace_player("Down".to_owned(), player_1_displacement);
                player_1_displacement_velocity += 2;
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::S),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                player_1_displacement_velocity = 4;
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Up),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if !single_player {
                    //player_2_displacement =
                    //displace_player("Down".to_owned(), player_2_displacement);
                    player_2_displacement_velocity += 2;
                }
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::Up),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if !single_player {
                    player_2_displacement_velocity = 4;
                }
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Down),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if !single_player {
                    //player_2_displacement = displace_player("Up".to_owned(), player_2_displacement);
                    player_2_displacement_velocity -= 2;
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::Down),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if !single_player {
                    player_2_displacement_velocity = -4;
                }
            }
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
    println!(
        "Dimensions: {}x{}",
        dimensions[0] as f32, dimensions[1] as f32
    );

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
