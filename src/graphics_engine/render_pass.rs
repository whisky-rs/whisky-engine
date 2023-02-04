use std::sync::Arc;

use vulkano::{
    buffer::TypedBufferAccess,
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents,
    },
    device::Device,
    image::SampleCount,
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            multisample::MultisampleState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{Framebuffer, RenderPass, Subpass},
    shader::ShaderModule,
    swapchain::Swapchain,
};

use super::{vertex::Vertex, Pipelines, Textures, VertexBuffers};

pub struct SimpleShapes {
    pub command_buffer_allocator: StandardCommandBufferAllocator,
    pub render_pass: Arc<RenderPass>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub circle_pipeline: Arc<GraphicsPipeline>,
    pub texture_pipeline: Arc<GraphicsPipeline>,
    pub texture_array_pipeline: Arc<GraphicsPipeline>,
}

impl SimpleShapes {
    fn create_pipeline(
        device: &Arc<Device>,
        subpass: Subpass,
        vertex_shader: Arc<ShaderModule>,
        fragment_shader: Arc<ShaderModule>,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .multisample_state(MultisampleState {
                rasterization_samples: subpass.num_samples().unwrap(),
                ..Default::default()
            })
            .color_blend_state(ColorBlendState::new(subpass.num_color_attachments()).blend_alpha())
            .render_pass(subpass)
            .build(device.clone())
            .unwrap()
    }

    fn create_pipeline_trg_strip(
        device: &Arc<Device>,
        subpass: Subpass,
        vertex_shader: Arc<ShaderModule>,
        fragment_shader: Arc<ShaderModule>,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .input_assembly_state(
                InputAssemblyState::new().topology(PrimitiveTopology::TriangleStrip),
            )
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .multisample_state(MultisampleState {
                rasterization_samples: subpass.num_samples().unwrap(),
                ..Default::default()
            })
            .color_blend_state(ColorBlendState::new(subpass.num_color_attachments()).blend_alpha())
            .render_pass(subpass)
            .build(device.clone())
            .unwrap()
    }

    /// Creates new render pass
    pub fn new(
        device: &Arc<Device>,
        swapchain: Arc<Swapchain>,
        max_sample_count: SampleCount,
    ) -> SimpleShapes {
        //loading shader files
        let vs = polygon_vs::load(device.clone()).unwrap();
        let fs = polygon_fs::load(device.clone()).unwrap();
        let circle_vs = circle_vs::load(device.clone()).unwrap();
        let circle_fs = circle_fs::load(device.clone()).unwrap();
        let text_fs = tex_fs::load(device.clone()).unwrap();
        let text_vs = tex_vs::load(device.clone()).unwrap();
        let text_array_vs = tex_array_vs::load(device.clone()).unwrap();
        let text_array_fs = tex_array_fs::load(device.clone()).unwrap();

        //creation of render pass
        let render_pass = vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                intermediary: {
                    load: Clear,
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
        .unwrap();

        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let circle_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let texture_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let texture_array_subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        //creation of graphics pipelines
        let pipeline = SimpleShapes::create_pipeline_trg_strip(device, subpass, vs, fs);

        let circle_pipeline =
            SimpleShapes::create_pipeline(device, circle_subpass, circle_vs, circle_fs);

        let texture_pipeline =
            SimpleShapes::create_pipeline_trg_strip(device, texture_subpass, text_vs, text_fs);

        let texture_array_pipeline = SimpleShapes::create_pipeline_trg_strip(
            device,
            texture_array_subpass,
            text_array_vs,
            text_array_fs,
        );

        SimpleShapes {
            command_buffer_allocator,
            render_pass,
            pipeline,
            circle_pipeline,
            texture_pipeline,
            texture_array_pipeline,
        }
    }

    pub fn render(
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        framebuffers: &mut [Arc<Framebuffer>],
        image_index: u32,
        viewport: &mut Viewport,
        textures: &Textures,
        pipelines: &Pipelines,
        buffers: VertexBuffers,
    ) {
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([1.0, 1.0, 1.0, 1.0].into()), None],
                    ..RenderPassBeginInfo::framebuffer(framebuffers[image_index as usize].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap()
            .set_viewport(0, [viewport.clone()])
            .bind_pipeline_graphics(pipelines.texture_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipelines.texture_pipeline.layout().clone(),
                0,
                textures.background.0.clone(),
            )
            .bind_vertex_buffers(0, buffers.background.clone())
            .draw(buffers.background.len() as u32, 1, 0, 0)
            .unwrap()
            .bind_pipeline_graphics(pipelines.polygon_pipeline.clone())
            // .bind_descriptor_sets(
            //     PipelineBindPoint::Graphics,
            //     pipelines.texture_pipeline.layout().clone(),
            //     0,
            //     textures.test_set.0.clone(),
            // )
            .bind_vertex_buffers(0, buffers.polygons.clone())
            .draw(buffers.polygons.len() as u32, 1, 0, 0)
            .unwrap()
            .bind_pipeline_graphics(pipelines.circle_pipeline.clone())
            .bind_vertex_buffers(0, buffers.circles.clone())
            .draw(buffers.circles.len() as u32, 1, 0, 0)
            .unwrap()


            .end_render_pass()
            .unwrap();
    }
}

mod polygon_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        vulkan_version: "1.2",
        spirv_version: "1.5",
        path: "shaders/vertex/polygon.glsl"
    }
}

mod polygon_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        vulkan_version: "1.2",
        spirv_version: "1.5",
        path: "shaders/fragment/polygon_frag.glsl"
    }
}

mod circle_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        vulkan_version: "1.2",
        spirv_version: "1.5",
        path: "shaders/vertex/circle.glsl"
    }
}

mod circle_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        vulkan_version: "1.2",
        spirv_version: "1.5",
        path: "shaders/fragment/circle_frag.glsl"
    }
}

mod tex_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vertex/texture.glsl"
    }
}

mod tex_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fragment/texture_frag.glsl"
    }
}

mod tex_array_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vertex/texture_array.glsl"
    }
}

mod tex_array_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fragment/texture_array_frag.glsl"
    }
}
