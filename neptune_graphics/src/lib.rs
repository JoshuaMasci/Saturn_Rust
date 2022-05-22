mod buffer;
mod id_pool;
mod interface;
mod pipeline;
mod render_graph;
mod resource;
mod texture;
pub mod vulkan;

use crate::render_graph::{
    ColorAttachment, DepthStencilAttachment, RasterPassBuilder, RenderGraphBuilder,
};
pub use buffer::BufferDescription;
pub use buffer::BufferUsages;
pub use texture::TextureDescription;
pub use texture::TextureDimensions;
pub use texture::TextureFormat;
pub use texture::TextureUsages;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum IndexSize {
    U16,
    U32,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum MemoryType {
    GpuOnly,
    CpuToGpu,
    GpuToCpu,
}

impl MemoryType {
    pub fn to_gpu_alloc(self) -> gpu_allocator::MemoryLocation {
        match self {
            MemoryType::GpuOnly => gpu_allocator::MemoryLocation::GpuOnly,
            MemoryType::CpuToGpu => gpu_allocator::MemoryLocation::CpuToGpu,
            MemoryType::GpuToCpu => gpu_allocator::MemoryLocation::GpuToCpu,
        }
    }
}

pub fn render_graph_test(render_graph: &mut RenderGraphBuilder) {
    let some_buffer = render_graph.create_buffer(BufferDescription {
        size: 16,
        usage: BufferUsages::STORAGE,
        memory_type: MemoryType::GpuOnly,
    });

    let swapchain_image = render_graph.get_swapchain_image();

    // let some_texture = render_graph.create_texture(TextureDescription {
    //     format: TextureFormat::Rgba8Unorm,
    //     size: TextureDimensions::D2(128, 128),
    //     usage: TextureUsages::COLOR_ATTACHMENT,
    //     memory_type: MemoryType::GpuOnly,
    // });

    let some_depth_texture = render_graph.create_texture(TextureDescription {
        format: TextureFormat::D32Float,
        size: TextureDimensions::D2(swapchain_image.1[0], swapchain_image.1[1]),
        usage: TextureUsages::DEPTH_STENCIL_ATTACHMENT,
        memory_type: MemoryType::GpuOnly,
    });

    render_graph.add_raster_pass(
        RasterPassBuilder::new("Test")
            .attachments(
                &[ColorAttachment {
                    id: swapchain_image.0,
                    clear: Some([1.0, 0.5, 0.25, 0.0]),
                }],
                Some(DepthStencilAttachment {
                    id: some_depth_texture,
                    clear: Some((1.0, 0)),
                }),
            )
            .vertex_buffer(some_buffer)
            .raster_fn(move |_, _, _, _| {}),
    );
}
