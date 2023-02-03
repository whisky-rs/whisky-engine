use bytemuck::{Pod, Zeroable};
use vulkano::impl_vertex;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 2],
    pub texture_id: u32,
    pub radius: f32,
    pub dist: f32,
    pub center: [f32; 2],
    pub color: [f32; 3],
}

impl_vertex!(Vertex, position, texture_id, radius, dist, center, color);
