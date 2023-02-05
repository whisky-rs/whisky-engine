use bytemuck::{Pod, Zeroable};
use rusttype::gpu_cache::Cache;
use rusttype::{point, Font, PositionedGlyph, Rect, Scale};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CopyBufferToImageInfo, CopyImageToBufferInfo,
    PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::image::sys::ImageCreateInfo;
use vulkano::image::view::{ImageView, ImageViewCreateInfo};
use vulkano::image::{
    ImageCreateFlags, ImageDimensions, ImageLayout, ImageUsage, ImmutableImage, SwapchainImage, AttachmentImage, ImageAccess, SampleCount,
};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::vertex_input::{VertexInputState, BuffersDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, RenderPass, Subpass, FramebufferCreateInfo};
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::swapchain::Swapchain;

use std::iter;
use std::sync::Arc;
use vulkano::memory::allocator::MemoryAllocator;

use super::Pipelines;
use super::vertex::Vertex;


mod chars_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vertex/vertex.glsl",
    }
}

mod chars_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fragment/fragment.glsl",
    }
}

struct TextData {
    glyphs: Vec<PositionedGlyph<'static>>,
    color: [f32; 4],
}

pub struct DrawText {
    device: Arc<Device>,
    queue: Arc<Queue>,
    font: Font<'static>,
    cache: Cache<'static>,
    cache_pixel_buffer: Vec<u8>,
    framebuffers: Vec<Arc<Framebuffer>>,
    texts: Vec<TextData>,
    pipeline: Arc<GraphicsPipeline>
}

const CACHE_WIDTH: usize = 1000;
const CACHE_HEIGHT: usize = 1000;

impl DrawText {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        swapchain: Arc<Swapchain>,
        images: &[Arc<SwapchainImage>],
        memory_allocator: &impl MemoryAllocator,
        dimentions: [u32; 2],
        max_sample_count: SampleCount,
    ) -> DrawText {
        let font_data = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
        let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
        let cache = Cache::builder()
            .dimensions(CACHE_WIDTH as u32, CACHE_HEIGHT as u32)
            .build();
        let cache_pixel_buffer = vec![0; CACHE_WIDTH * CACHE_HEIGHT];

        let render_pass = vulkano::single_pass_renderpass!(device.clone(),
        attachments: {
            intermediary: {
                load: Load,
                store: DontCare,
                format: swapchain.image_format(),
                samples: max_sample_count,
            },
            color: {
                load: DontCare,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            }
        },
        pass: {
            color: [intermediary],
            depth_stencil: {}
            resolve: [color],
        }
        )
        .unwrap() as Arc<RenderPass>;

        let c_vs = chars_vs::load(device.clone()).unwrap();
        let c_fs = chars_fs::load(device.clone()).unwrap();


        let pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .vertex_shader(c_vs.entry_point("main").unwrap(), ())
        .input_assembly_state(
            InputAssemblyState::new(),
        )
        .fragment_shader(c_fs.entry_point("main").unwrap(), ())
        .multisample_state(MultisampleState {
            rasterization_samples: Subpass::from(render_pass.clone(), 0).unwrap().num_samples().unwrap(),
            ..Default::default()
        })
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap();

        let framebuffers = images
        .iter()
        .map(|image| {

            let intermediary = ImageView::new_default(
                AttachmentImage::transient_multisampled(
                    memory_allocator,
                    dimentions,
                    max_sample_count,
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
        .collect::<Vec<_>>();


        DrawText {
            device,
            queue,
            font,
            cache,
            cache_pixel_buffer,
            framebuffers,
            texts: vec![],
            pipeline
        }
    }

    pub fn queue_text(&mut self, x: f32, y: f32, size: f32, color: [f32; 4], text: &str) {
        let glyphs: Vec<PositionedGlyph> = self
            .font
            .layout(text, Scale::uniform(size), point(x, y))
            .map(|x| x.clone())
            .collect();
        for glyph in &glyphs {
            self.cache.queue_glyph(0, glyph.clone());
        }
        self.texts.push(TextData {
            glyphs: glyphs.clone(),
            color,
        });
    }

    pub fn draw_text<'a>(
        &mut self,
        command_buffer: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        image_num: usize,
        dimentions: [usize; 2],
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        memory_allocator: &impl MemoryAllocator,
    ) -> &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
        let cache_pixel_buffer = &mut self.cache_pixel_buffer;
        let cache = &mut self.cache;

        // update texture cache
        cache
            .cache_queued(|rect, src_data| {
                let width = (rect.max.x - rect.min.x) as usize;
                let height = (rect.max.y - rect.min.y) as usize;
                let mut dst_index = rect.min.y as usize * CACHE_WIDTH + rect.min.x as usize;
                let mut src_index = 0;

                for _ in 0..height {
                    let dst_slice = &mut cache_pixel_buffer[dst_index..dst_index + width];
                    let src_slice = &src_data[src_index..src_index + width];
                    dst_slice.copy_from_slice(src_slice);

                    dst_index += CACHE_WIDTH;
                    src_index += width;
                }
            })
            .unwrap();

        let buffer = CpuAccessibleBuffer::<[u8]>::from_iter(
            memory_allocator,
            BufferUsage {
                transfer_src: true,
                transfer_dst: true,
                uniform_texel_buffer: true,
                storage_texel_buffer: true,
                uniform_buffer: true,
                storage_buffer: true,
                index_buffer: true,
                vertex_buffer: true,
                indirect_buffer: true,
                shader_device_address: true,
                ..Default::default()
            },
            false,
            cache_pixel_buffer.iter().cloned(),
        )
        .unwrap();

        let (cache_texture, cache_texture_write) = ImmutableImage::uninitialized(
            memory_allocator,
            ImageDimensions::Dim2d {
                width: CACHE_WIDTH as u32,
                height: CACHE_HEIGHT as u32,
                array_layers: 1,
            },
            Format::R8_UNORM,
            1,
            ImageUsage {
                sampled: true,
                transfer_dst: true,
                ..ImageUsage::empty()
            },
          ImageCreateFlags::empty(),
            ImageLayout::General,
            Some(self.queue.queue_family_index()),
        )
        .unwrap();

        let sampler = Sampler::new(
            self.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                ..Default::default()
            },
        )
        .unwrap();

        let cache_texture_view = ImageView::new_default(cache_texture.clone())
        .unwrap();

        let set = PersistentDescriptorSet::new(
            descriptor_set_allocator,
            self.pipeline.layout().set_layouts().get(0).unwrap().clone(),
            [WriteDescriptorSet::image_view_sampler(
                0,
                cache_texture_view,
                sampler,
            )],
        );

        let mut command_buffer = command_buffer
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                buffer,
                cache_texture_write,
            ))
            .unwrap()
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([1.0, 1.0, 1.0, 1.0].into()), None],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_num as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .unwrap();

        // draw
        for text in &mut self.texts.drain(..) {
            let vertices: Vec<Vertex> = text
                .glyphs
                .iter()
                .flat_map(|g| {
                    if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(0, g) {
                        let gl_rect = Rect {
                            min: point(
                                (screen_rect.min.x as f32 / dimentions[0] as f32 - 0.5) * 2.0,
                                (screen_rect.min.y as f32 / dimentions[1] as f32 - 0.5) * 2.0,
                            ),
                            max: point(
                                (screen_rect.max.x as f32 / dimentions[0] as f32 - 0.5) * 2.0,
                                (screen_rect.max.y as f32 / dimentions[1] as f32 - 0.5) * 2.0,
                            ),
                        };
                        vec![
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.max.y],
                                tex_position: [uv_rect.min.x, uv_rect.max.y],
                                color: [text.color[0], text.color[1], text.color[2]],
                                ..Default::default()
                            },
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.min.y],
                                tex_position: [uv_rect.min.x, uv_rect.min.y],
                                color: [text.color[0], text.color[1], text.color[2]],
                                // color: text.color,
                                ..Default::default()
                            },
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.min.y],
                                tex_position: [uv_rect.max.x, uv_rect.min.y],
                                color: [text.color[0], text.color[1], text.color[2]],
                                // color: text.color,
                                ..Default::default()
                            },
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.min.y],
                                tex_position: [uv_rect.max.x, uv_rect.min.y],
                                color: [text.color[0], text.color[1], text.color[2]],
                                // color: text.color,
                                ..Default::default()
                            },
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.max.y],
                                tex_position: [uv_rect.max.x, uv_rect.max.y],
                                color: [text.color[0], text.color[1], text.color[2]],
                                // color: text.color,
                                ..Default::default()
                            },
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.max.y],
                                tex_position: [uv_rect.min.x, uv_rect.max.y],
                                color: [text.color[0], text.color[1], text.color[2]],
                                // color: text.color,
                                ..Default::default()
                            },
                        ]
                        .into_iter()
                    } else {
                        vec![].into_iter()
                    }
                })
                .collect();

            let vertex_buffer = CpuAccessibleBuffer::from_iter(
                memory_allocator,
                BufferUsage {
                    vertex_buffer: true,
                    transfer_dst: true,
                    ..Default::default()
                },
                false,
                vertices.into_iter(),
            )
            .unwrap();
            command_buffer = command_buffer
            .bind_vertex_buffers(0, vertex_buffer.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    0,
                    set.clone().unwrap(),
                )
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap();
        }

        command_buffer.end_render_pass().unwrap()
    }
}

impl DrawTextTrait for AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
    fn draw_text(
        &mut self,
        data: &mut DrawText,
        image_num: usize,
        dimensions: [usize; 2],
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        memory_allocator: &impl MemoryAllocator,
    ) -> &mut Self {
        data.draw_text(
            self,
            image_num,
            dimensions,
            descriptor_set_allocator,
            memory_allocator,
        )
    }
}

pub trait DrawTextTrait {
    fn draw_text(
        &mut self,
        data: &mut DrawText,
        image_num: usize,
        dimensions: [usize; 2],
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        memory_allocator: &impl MemoryAllocator,
    ) -> &mut Self;
}
