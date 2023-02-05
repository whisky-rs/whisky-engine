use crossbeam::channel;
use std::sync::Arc;
use std::vec;
use vulkano::image::{AttachmentImage, ImageUsage, SampleCount};
use vulkano::memory::allocator::MemoryAllocator;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBufferAbstract},
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    image::{view::ImageView, ImageAccess, MipmapsCount, SwapchainImage},
    memory::allocator::StandardMemoryAllocator,
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    swapchain::{
        acquire_next_image, AcquireError, SwapchainCreateInfo, SwapchainCreationError,
        SwapchainPresentInfo,
    },
    sync::{self, FlushError, GpuFuture},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use vertex::Vertex;

use crate::game_logic::GameState;
use crate::geometry::Circle;
use crate::graphics_engine::render_pass::SimpleShapes;
use crate::physics::{DisplayMessage, WithColor};
use crate::InputMessage;

use self::draw_text::DrawText;

use super::geometry::Polygon;

mod draw_text;
mod render_pass;
mod setup;
mod texture;
mod vertex;

pub struct VertexBuffers {
    background: Arc<CpuAccessibleBuffer<[Vertex]>>,
    polygons: Arc<CpuAccessibleBuffer<[Vertex]>>,
    circles: Arc<CpuAccessibleBuffer<[Vertex]>>,
}

pub struct Textures {
    background: texture::Texture,
    test_set: texture::Texture,
}

pub struct Pipelines {
    texture_array_pipeline: Arc<GraphicsPipeline>,
    texture_pipeline: Arc<GraphicsPipeline>,
    polygon_pipeline: Arc<GraphicsPipeline>,
    circle_pipeline: Arc<GraphicsPipeline>,
}

/// Runs simple graphics engine, as argument takes channel providing Polygon data to be drawn
pub fn run(
    channel: channel::Receiver<DisplayMessage>,
    mut messages: channel::Sender<InputMessage>,
    mut game_state: GameState,
) {
    let setup::Init {
        device,
        queue,
        surface,
        event_loop,
        mut swapchain,
        images,
        max_sample_count,
    } = setup::init();

    let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

    let render_pass::SimpleShapes {
        command_buffer_allocator,
        render_pass,
        pipeline,
        circle_pipeline,
        texture_pipeline,
        texture_array_pipeline,
    } = render_pass::SimpleShapes::new(&device, swapchain.clone(), max_sample_count);

    let pipelines = Pipelines {
        circle_pipeline,
        polygon_pipeline: pipeline,
        texture_array_pipeline,
        texture_pipeline,
    };
    let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();

    let dimensions = window.inner_size();
    let mut draw_text = DrawText::new(
        device.clone(),
        queue.clone(),
        swapchain.clone(),
        &images,
        &memory_allocator,
        [dimensions.width as u32, dimensions.height as u32],
        max_sample_count,
    );

    let mut first_frame = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    let texture_buffer = create_vertex_buffer(
        &memory_allocator,
        create_positioned_vertexes(vec![[-1.0, -1.0], [-1.0, 1.0], [1.0, -1.0], [1.0, 1.0]])
            .iter()
            .cloned(),
    );
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());

    println!("Loading Textures Files...");

    let background_set = texture::Texture::new(
        device.clone(),
        &["assets/images/magic_pen_bg.png"],
        &memory_allocator,
        &mut first_frame,
        MipmapsCount::One,
        pipelines.texture_pipeline.clone(),
        &descriptor_set_allocator,
    );

    let test_set = texture::Texture::new(
        device.clone(),
        &["assets/images/pineapple.png"],
        &memory_allocator,
        &mut first_frame,
        MipmapsCount::One,
        pipelines.texture_pipeline.clone(),
        &descriptor_set_allocator,
    );

    let game_textures = Textures {
        background: background_set,
        test_set,
    };

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };
    let mut framebuffers = window_size_dependent_setup(
        &images,
        render_pass.clone(),
        &mut viewport,
        &memory_allocator,
        max_sample_count,
    );

    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(
        first_frame
            .build()
            .unwrap()
            .execute(queue.clone())
            .unwrap()
            .boxed(),
        );

    let mut is_first_run = true;
    let mut circles_vertices = vec![];
    let mut polygons_vertices = vec![];



    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::WindowEvent {
            event: WindowEvent::MouseInput { state, button, .. },
            ..
        } => {
            game_state.handle_mouse_input(state, button, &mut messages);
        }
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            game_state.handle_mouse_moved(position, dimensions);
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            game_state.handle_keyboard_input(input);
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            recreate_swapchain = true;
        }
        Event::RedrawEventsCleared => {
            if is_first_run {
                println!("texture loaded");
                is_first_run = false;
            }

            // window section
            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
            let dimensions = window.inner_size();
            if dimensions.width == 0 || dimensions.height == 0 {
                return;
            }

            previous_frame_end.as_mut().unwrap().cleanup_finished();

            if recreate_swapchain {
                let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage {
                        transfer_src: false,
                        transfer_dst: true,
                        sampled: true,
                        storage: false,
                        color_attachment: true,
                        depth_stencil_attachment: false,
                        transient_attachment: false,
                        input_attachment: false,
                        ..Default::default()
                    },
                    ..swapchain.create_info()
                }) {
                    Ok(r) => r,
                    Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

                swapchain = new_swapchain;
                framebuffers = window_size_dependent_setup(
                    &new_images,
                    render_pass.clone(),
                    &mut viewport,
                    &memory_allocator,
                    max_sample_count,
                );

                draw_text = DrawText::new(
                    device.clone(),
                    queue.clone(),
                    swapchain.clone(),
                    &new_images,
                    &memory_allocator,
                    [dimensions.width as u32, dimensions.height as u32],
                    max_sample_count,
                );

                recreate_swapchain = false;
            }

            let (image_index, suboptimal, acquire_future) =
                match acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            match channel.try_recv() {
                Ok(received) => {
                    (polygons_vertices, circles_vertices) =
                        format_data((received.polygons, received.circles));
                }
                Err(channel::TryRecvError::Disconnected) => *control_flow = ControlFlow::Exit,
                _ => {}
            }

            let vertex_buffer_polygons =
                create_vertex_buffer(&memory_allocator, polygons_vertices.clone());

            let vertex_buffer_circles = if !circles_vertices.is_empty() {
                create_vertex_buffer(&memory_allocator, circles_vertices.clone())
            } else {
                create_vertex_buffer(&memory_allocator, [Vertex::default(); 3])
            };

            let mut builder = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            SimpleShapes::render(
                &mut builder,
                &mut framebuffers,
                image_index,
                &mut viewport,
                &game_textures,
                &pipelines,
                VertexBuffers {
                    background: texture_buffer.clone(),
                    polygons: vertex_buffer_polygons,
                    circles: vertex_buffer_circles,
                },
                &mut draw_text,
                image_index as usize,
                [dimensions.width as usize, dimensions.height as usize],
                &descriptor_set_allocator,
                &memory_allocator,
            );
            let command_buffer = builder.build().unwrap();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(
                    queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_index),
                )
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(future.boxed());
                }
                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
            }
        }
        _ => (),
    });
}

fn create_vertex_buffer(
    memory_allocator: &(impl MemoryAllocator + ?Sized),
    vertexes: impl IntoIterator<Item = Vertex, IntoIter = impl ExactSizeIterator<Item = Vertex>>,
) -> Arc<CpuAccessibleBuffer<[Vertex]>> {
    CpuAccessibleBuffer::<[Vertex]>::from_iter(
        memory_allocator,
        BufferUsage {
            vertex_buffer: true,
            ..BufferUsage::empty()
        },
        false,
        vertexes,
    )
    .unwrap()
}

/// Changes Polygon to correct order of Vertexes, also creates quads needed to draw cricles
fn format_data(
    (polygons, circles): (Vec<WithColor<Polygon>>, Vec<WithColor<Circle>>),
) -> (Vec<Vertex>, Vec<Vertex>) {
    let polygons_vertexes = polygons
        .into_iter()
        .enumerate()
        .flat_map(|(i, pol)| {
            std::iter::once(Vertex {
                texture_id: i as u32,
                position: [
                    pol.shape.vertices.last().unwrap().0 as f32,
                    -pol.shape.vertices.last().unwrap().1 as f32,
                ],
                ..Default::default()
            })
            .chain(if pol.shape.vertices.len() == 4 {
                vec![
                    Vertex {
                        texture_id: i as u32,
                        position: [
                            pol.shape.vertices[3].0 as f32,
                            -pol.shape.vertices[3].1 as f32,
                        ],
                        color: pol.color,
                        tex_position: [0.0, 0.0],
                        ..Default::default()
                    },
                    Vertex {
                        texture_id: i as u32,
                        position: [
                            pol.shape.vertices[0].0 as f32,
                            -pol.shape.vertices[0].1 as f32,
                        ],
                        color: pol.color,
                        tex_position: [0.0, 1.0],
                        ..Default::default()
                    },
                    Vertex {
                        texture_id: i as u32,
                        position: [
                            pol.shape.vertices[2].0 as f32,
                            -pol.shape.vertices[2].1 as f32,
                        ],
                        color: pol.color,
                        tex_position: [1.0, 0.0],
                        ..Default::default()
                    },
                    Vertex {
                        texture_id: i as u32,
                        position: [
                            pol.shape.vertices[1].0 as f32,
                            -pol.shape.vertices[1].1 as f32,
                        ],
                        color: pol.color,
                        tex_position: [0.0, 0.0],
                        ..Default::default()
                    },
                ]
                .into_iter()
            } else {
                vec![
                    Vertex {
                        texture_id: i as u32,
                        position: [
                            pol.shape.vertices[2].0 as f32,
                            -pol.shape.vertices[2].1 as f32,
                        ],
                        color: pol.color,
                        tex_position: [0.0, 0.0],
                        ..Default::default()
                    },
                    Vertex {
                        texture_id: i as u32,
                        position: [
                            pol.shape.vertices[0].0 as f32,
                            -pol.shape.vertices[0].1 as f32,
                        ],
                        color: pol.color,
                        tex_position: [1.0, 0.0],
                        ..Default::default()
                    },
                    Vertex {
                        texture_id: i as u32,
                        position: [
                            pol.shape.vertices[1].0 as f32,
                            -pol.shape.vertices[1].1 as f32,
                        ],
                        color: pol.color,
                        tex_position: [1.0, 0.0],
                        ..Default::default()
                    },
                ]
                .into_iter()
            })
            .chain(std::iter::once(Vertex {
                texture_id: i as u32,
                position: [
                    pol.shape.vertices[1].0 as f32,
                    -pol.shape.vertices[1].1 as f32,
                ],
                ..Default::default()
            }))
            .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let circles_vertexes = circles
        .into_iter()
        .flat_map(|circle| {
            let color = circle.color;
            let center = [circle.shape.center.0 as f32, -circle.shape.center.1 as f32];
            let radius = circle.shape.radius as f32;
            let positions = [
                [
                    circle.shape.center.0 as f32,
                    (-(circle.shape.center.1 - circle.shape.radius * 2.0_f64.sqrt())) as f32,
                ],
                [
                    (circle.shape.center.0 - circle.shape.radius * 2.0_f64.sqrt()) as f32,
                    -circle.shape.center.1 as f32,
                ],
                [
                    circle.shape.center.0 as f32,
                    (-(circle.shape.center.1 + circle.shape.radius * 2.0_f64.sqrt())) as f32,
                ],
                [
                    circle.shape.center.0 as f32,
                    (-(circle.shape.center.1 + circle.shape.radius * 2.0_f64.sqrt())) as f32,
                ],
                [
                    (circle.shape.center.0 + circle.shape.radius * 2.0_f64.sqrt()) as f32,
                    -circle.shape.center.1 as f32,
                ],
                [
                    circle.shape.center.0 as f32,
                    (-(circle.shape.center.1 - circle.shape.radius * 2.0_f64.sqrt())) as f32,
                ],
            ];
            create_circle_vertices(positions, radius, center, color)
        })
        .collect::<Vec<_>>();

    (polygons_vertexes, circles_vertexes)
}

fn create_circle_vertices(
    positions: [[f32; 2]; 6],
    radius: f32,
    center: [f32; 2],
    color: [f32; 3],
) -> Vec<Vertex> {
    positions
        .into_iter()
        .map(|position| Vertex {
            position,
            radius,
            center,
            color,
            ..Default::default()
        })
        .collect()
}

fn calculate_vertex_distance(pos0: [f32; 2], pos1: [f32; 2]) -> f32 {
    ((pos0[0] - pos1[0]).powi(2) + (pos0[1] - pos1[1]).powi(2)).sqrt()
}

fn create_positioned_vertexes(positions: Vec<[f32; 2]>) -> Vec<Vertex> {
    positions
        .into_iter()
        .map(|position| Vertex {
            position,
            ..Default::default()
        })
        .collect()
}

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
    memory_allocator: &(impl MemoryAllocator + ?Sized),
    sample_count: SampleCount,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| {
            let intermediary = ImageView::new_default(
                AttachmentImage::transient_multisampled(
                    memory_allocator,
                    dimensions,
                    sample_count,
                    image.format(),
                )
                .unwrap(),
            )
            .unwrap();

            let view = ImageView::new_default(image.clone()).unwrap();

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![intermediary, view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}
