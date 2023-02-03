use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use png::Info;
use vulkano::command_buffer::allocator::CommandBufferAllocator;
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::device::Device;
use vulkano::memory::allocator::MemoryAllocator;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::{
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    format::Format,
    image::{view::ImageView, ImageDimensions, ImmutableImage, MipmapsCount},
    pipeline::Pipeline,
    sampler::{Filter, Sampler, SamplerCreateInfo},
};

pub struct Texture(pub Arc<PersistentDescriptorSet>);
impl Texture {
    pub fn new<L, A: CommandBufferAllocator>(
        device: Arc<Device>,
        paths: &[impl AsRef<Path>],
        memory_allocator: &(impl MemoryAllocator + ?Sized),
        command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        mip_levels: MipmapsCount,
        pipeline: Arc<GraphicsPipeline>,
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
    ) -> Self {
        let image = Self::load(paths, memory_allocator, command_buffer, mip_levels);
        let sampler = Sampler::new(
            device,
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                ..Default::default()
            },
        )
        .unwrap();
        let layout = pipeline.layout().set_layouts().get(0).unwrap();
        Texture(Self::create_descriptor_set(
            descriptor_set_allocator,
            layout,
            image,
            sampler,
        ))
    }

    fn load<L, A>(
        paths: &[impl AsRef<Path>],
        memory_allocator: &(impl MemoryAllocator + ?Sized),
        command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        mip_levels: MipmapsCount,
    ) -> Arc<ImageView<ImmutableImage>>
    where
        A: CommandBufferAllocator,
    {
        let mut dimensions = (0, 0);

        let files_data: Vec<_> = paths
            .iter()
            .map(|path| File::open(path).unwrap())
            .flat_map(|file| {
                let mut decoder = png::Decoder::new(file);
                let &Info { width, height, .. } = decoder.read_header_info().unwrap();
                dimensions = (width, height);
                let mut reader = decoder.read_info().unwrap();
                let mut image_data = Vec::new();
                image_data.resize((width * height * 4) as usize, 0);
                reader.next_frame(&mut image_data).unwrap();
                image_data
            })
            .collect();

        let dimensions = ImageDimensions::Dim2d {
            width: dimensions.0,
            height: dimensions.1,
            array_layers: paths.len() as u32,
        };

        let image = ImmutableImage::from_iter(
            memory_allocator,
            files_data,
            dimensions,
            mip_levels,
            Format::R8G8B8A8_SRGB,
            command_buffer,
        )
        .unwrap();
        ImageView::new_default(image).unwrap()
    }

    fn create_descriptor_set(
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        layout: &Arc<DescriptorSetLayout>,
        drawing: Arc<ImageView<ImmutableImage>>,
        sampler: Arc<Sampler>,
    ) -> Arc<PersistentDescriptorSet> {
        PersistentDescriptorSet::new(
            descriptor_set_allocator,
            layout.clone(),
            [WriteDescriptorSet::image_view_sampler(0, drawing, sampler)],
        )
        .unwrap()
    }
}
