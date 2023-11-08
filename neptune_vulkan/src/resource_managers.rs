use crate::buffer::{AshBuffer, Buffer};
use crate::descriptor_set::{DescriptorCount, DescriptorSet};
use crate::device::AshDevice;
use crate::image::{AshImage, Image, TransientImageSize};
use crate::render_graph::{
    BufferGraphResource, BufferResourceDescription, ImageGraphResource, ImageResourceDescription,
};
use crate::sampler::Sampler;
use crate::swapchain::AcquiredSwapchainImage;
use crate::{BufferKey, ImageHandle, ImageKey, SamplerKey, VulkanError};
use ash::vk;
use log::{error, warn};
use slotmap::SlotMap;
use std::sync::Arc;

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub enum Queue {
    #[default]
    Graphics,
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub struct BufferBarrierFlags {
    pub stage_mask: vk::PipelineStageFlags2,
    pub access_flags: vk::AccessFlags2,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferResourceAccess {
    #[default]
    None,
    TransferRead,
    TransferWrite,
    VertexRead,
    IndexRead,
    IndirectRead,
    UniformRead,
    StorageRead,
    StorageWrite,
}

impl BufferResourceAccess {
    pub fn get_barrier_flags(&self) -> BufferBarrierFlags {
        let shader_all: vk::PipelineStageFlags2 = vk::PipelineStageFlags2::VERTEX_SHADER
            | vk::PipelineStageFlags2::FRAGMENT_SHADER
            | vk::PipelineStageFlags2::COMPUTE_SHADER
            | vk::PipelineStageFlags2::TASK_SHADER_EXT
            | vk::PipelineStageFlags2::MESH_SHADER_EXT
            | vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR;

        match self {
            Self::None => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::NONE,
                access_flags: vk::AccessFlags2::NONE,
            },
            Self::TransferRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_flags: vk::AccessFlags2::MEMORY_READ,
            },
            Self::TransferWrite => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_flags: vk::AccessFlags2::MEMORY_WRITE,
            },
            Self::VertexRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::VERTEX_INPUT,
                access_flags: vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
            },
            Self::IndexRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::VERTEX_INPUT,
                access_flags: vk::AccessFlags2::INDEX_READ,
            },
            Self::IndirectRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::DRAW_INDIRECT,
                access_flags: vk::AccessFlags2::INDIRECT_COMMAND_READ,
            },
            Self::UniformRead => BufferBarrierFlags {
                stage_mask: shader_all,
                access_flags: vk::AccessFlags2::UNIFORM_READ,
            },
            Self::StorageRead => BufferBarrierFlags {
                stage_mask: shader_all,
                access_flags: vk::AccessFlags2::SHADER_STORAGE_READ,
            },
            Self::StorageWrite => BufferBarrierFlags {
                stage_mask: shader_all,
                access_flags: vk::AccessFlags2::SHADER_WRITE,
            },
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub struct ImageBarrierFlags {
    pub stage_mask: vk::PipelineStageFlags2,
    pub access_flags: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageResourceAccess {
    #[default]
    None,
    TransferRead,
    TransferWrite,
    AttachmentWrite,
    SampledRead,
    StorageRead,
    StorageWrite,
}

impl ImageResourceAccess {
    pub fn get_barrier_flags(&self, is_color_image: bool) -> ImageBarrierFlags {
        let shader_all: vk::PipelineStageFlags2 = vk::PipelineStageFlags2::VERTEX_SHADER
            | vk::PipelineStageFlags2::FRAGMENT_SHADER
            | vk::PipelineStageFlags2::COMPUTE_SHADER
            | vk::PipelineStageFlags2::TASK_SHADER_EXT
            | vk::PipelineStageFlags2::MESH_SHADER_EXT
            | vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR;

        match self {
            Self::None => ImageBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::NONE,
                access_flags: vk::AccessFlags2::NONE,
                layout: vk::ImageLayout::UNDEFINED,
            },
            Self::TransferRead => ImageBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_flags: vk::AccessFlags2::TRANSFER_READ,
                layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            },
            Self::TransferWrite => ImageBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_flags: vk::AccessFlags2::TRANSFER_WRITE,
                layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            },
            Self::AttachmentWrite => {
                if is_color_image {
                    ImageBarrierFlags {
                        stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        access_flags: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    }
                } else {
                    ImageBarrierFlags {
                        stage_mask: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                        access_flags: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
                    }
                }
            }
            Self::SampledRead => ImageBarrierFlags {
                stage_mask: shader_all,
                access_flags: vk::AccessFlags2::SHADER_READ,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            Self::StorageRead => ImageBarrierFlags {
                stage_mask: shader_all,
                access_flags: vk::AccessFlags2::SHADER_READ,
                layout: vk::ImageLayout::GENERAL,
            },
            Self::StorageWrite => ImageBarrierFlags {
                stage_mask: shader_all,
                access_flags: vk::AccessFlags2::SHADER_WRITE,
                layout: vk::ImageLayout::GENERAL,
            },
        }
    }
}

pub struct BufferResource {
    pub buffer: Buffer,
    pub last_access: BufferResourceAccess,
}

pub struct BufferTempResource {
    pub buffer: AshBuffer,
    pub last_access: BufferResourceAccess,
}

pub struct ImageResource {
    image: Image,
}

pub struct ImageTempResource {
    pub image: AshImage,
    pub last_usage: ImageResourceAccess,
}

pub struct ResourceManager {
    #[allow(unused)]
    device: Arc<AshDevice>,

    buffers: SlotMap<BufferKey, BufferResource>,
    freed_buffers: Vec<BufferKey>,

    images: SlotMap<ImageKey, ImageResource>,
    freed_images: Vec<ImageKey>,

    samplers: SlotMap<SamplerKey, Arc<Sampler>>,

    pub(crate) descriptor_set: DescriptorSet,

    //TODO: rework this use multiple frames in flight
    freed_buffers2: Vec<BufferKey>,
    freed_images2: Vec<ImageKey>,
    pub(crate) transient_buffers: Vec<Buffer>,
    pub(crate) transient_images: Vec<Image>,
}

impl ResourceManager {
    pub fn new(device: Arc<AshDevice>) -> Self {
        let descriptor_set = DescriptorSet::new(
            device.clone(),
            DescriptorCount {
                storage_buffers: 1024,
                storage_images: 1024,
                sampled_images: 1024,
                samplers: 128,
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            device,
            buffers: SlotMap::with_key(),
            freed_buffers: Vec::new(),
            images: SlotMap::with_key(),
            freed_images: Vec::new(),
            samplers: SlotMap::with_key(),
            descriptor_set,
            freed_buffers2: Vec::new(),
            freed_images2: Vec::new(),
            transient_buffers: Vec::new(),
            transient_images: Vec::new(),
        }
    }

    pub fn flush_frame(&mut self) {
        //TODO: fix this when multiple frames in flight implemented
        for key in self.freed_buffers2.drain(..) {
            if self.buffers.remove(key).is_none() {
                warn!("BufferKey({:?}) was invalid on deletion", key);
            }
        }
        for key in self.freed_images2.drain(..) {
            if self.images.remove(key).is_none() {
                warn!("ImageKey({:?}) was invalid on deletion", key);
            }
        }
        self.freed_buffers2 = std::mem::take(&mut self.freed_buffers);
        self.freed_images2 = std::mem::take(&mut self.freed_images);

        self.transient_buffers.clear();
        self.transient_images.clear();
    }

    //Buffers
    pub fn add_buffer(&mut self, mut buffer: Buffer) -> BufferKey {
        if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
            buffer.storage_binding = Some(self.descriptor_set.bind_storage_buffer(&buffer));
        }

        self.buffers.insert(BufferResource {
            buffer,
            last_access: Default::default(),
        })
    }
    pub fn get_buffer(&self, key: BufferKey) -> Option<&Buffer> {
        self.buffers.get(key).map(|resource| &resource.buffer)
    }
    pub fn get_and_update_buffer_resource(
        &mut self,
        key: BufferKey,
        new_last_access: BufferResourceAccess,
    ) -> Option<BufferTempResource> {
        self.buffers
            .get_mut(key)
            .map(|resource| BufferTempResource {
                buffer: resource.buffer.get_copy(),
                last_access: std::mem::replace(&mut resource.last_access, new_last_access),
            })
    }

    pub fn remove_buffer(&mut self, key: BufferKey) {
        self.freed_buffers.push(key);
    }

    //Images
    pub fn add_image(&mut self, mut image: Image) -> ImageKey {
        if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
            image.storage_binding = Some(self.descriptor_set.bind_storage_image(&image));
        }

        if image.usage.contains(vk::ImageUsageFlags::SAMPLED) {
            image.sampled_binding = Some(self.descriptor_set.bind_sampled_image(&image));
        }

        self.images.insert(ImageResource { image })
    }
    pub fn get_image(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key).map(|resource| &resource.image)
    }
    pub fn remove_image(&mut self, key: ImageKey) {
        self.freed_images.push(key);
    }

    //Samplers
    pub fn add_sampler(&mut self, mut sampler: Sampler) -> SamplerKey {
        sampler.binding = Some(self.descriptor_set.bind_sampler(&sampler));
        self.samplers.insert(Arc::new(sampler))
    }
    pub fn get_sampler(&self, key: SamplerKey) -> Option<Arc<Sampler>> {
        self.samplers.get(key).cloned()
    }
    pub fn remove_sampler(&mut self, key: SamplerKey) {
        if self.samplers.remove(key).is_none() {
            warn!("Tried to remove invalid SamplerKey({:?})", key);
        }
    }

    //Graph Functions
    //TODO: take in vector to reuse memory?
    /// Get the buffer resources and update the last usages
    pub fn get_buffer_resources(
        &mut self,
        buffers: &[BufferGraphResource],
    ) -> Result<Vec<BufferTempResource>, VulkanError> {
        let mut buffer_resources = Vec::with_capacity(buffers.len());
        for buffer in buffers {
            buffer_resources.push(match &buffer.description {
                BufferResourceDescription::Persistent(key) => {
                    let buffer = &self.buffers[*key];
                    //TODO: get usages with multiple frames in flight
                    //TODO: write last usages + queue
                    BufferTempResource {
                        buffer: buffer.buffer.get_copy(),
                        last_access: buffer.last_access,
                    }
                }
                BufferResourceDescription::Transient(buffer_description) => {
                    let mut buffer =
                        Buffer::new(self.device.clone(), "Transient Buffer", buffer_description)?;
                    if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                        buffer.storage_binding =
                            Some(self.descriptor_set.bind_storage_buffer(&buffer));
                    }
                    let resource = BufferTempResource {
                        buffer: buffer.get_copy(),
                        last_access: BufferResourceAccess::None, //Never used before
                    };
                    self.transient_buffers.push(buffer);
                    resource
                }
            });
        }

        Ok(buffer_resources)
    }

    //TODO: take in vector to reuse memory?
    /// Get the image resources and update the last usages
    pub fn get_image_resources(
        &mut self,
        swapchain_images: &[AcquiredSwapchainImage],
        images: &[ImageGraphResource],
    ) -> Result<Vec<ImageTempResource>, VulkanError> {
        let mut image_resources = Vec::with_capacity(images.len());
        for image in images {
            image_resources.push(match &image.description {
                ImageResourceDescription::Persistent(key) => {
                    let image = &self.images[*key];
                    //TODO: get usages with multiple frames in flight
                    //TODO: write last usages + queue + layout
                    ImageTempResource {
                        image: image.image.get_copy(),
                        last_usage: ImageResourceAccess::None,
                    }
                }
                ImageResourceDescription::Transient(transient_image_description) => {
                    let image_size = get_transient_image_size(
                        transient_image_description.size.clone(),
                        self,
                        images,
                        swapchain_images,
                    );
                    let image_description = transient_image_description
                        .to_image_description([image_size.width, image_size.height]);
                    let mut image =
                        Image::new_2d(self.device.clone(), "Transient Image", &image_description)?;

                    if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
                        image.storage_binding =
                            Some(self.descriptor_set.bind_storage_image(&image));
                    }

                    if image.usage.contains(vk::ImageUsageFlags::SAMPLED) {
                        image.sampled_binding =
                            Some(self.descriptor_set.bind_sampled_image(&image));
                    }

                    let resource = ImageTempResource {
                        image: image.get_copy(),
                        last_usage: ImageResourceAccess::None, //Never used before
                    };
                    self.transient_images.push(image);
                    resource
                }
                ImageResourceDescription::Swapchain(index) => {
                    //Swapchain always starts out unused
                    ImageTempResource {
                        image: swapchain_images[*index].image,
                        last_usage: ImageResourceAccess::None,
                    }
                }
            });
        }

        Ok(image_resources)
    }
}

fn get_transient_image_size(
    size: TransientImageSize,
    persistent: &ResourceManager,
    images: &[ImageGraphResource],
    swapchain_images: &[AcquiredSwapchainImage],
) -> vk::Extent2D {
    match size {
        TransientImageSize::Exact(extent) => extent,
        TransientImageSize::Relative(scale, target) => {
            let mut extent = match target {
                ImageHandle::Persistent(image_key) => {
                    persistent.get_image(image_key).as_ref().unwrap().size
                }
                ImageHandle::Transient(index) => match &images[index].description {
                    ImageResourceDescription::Persistent(image_key) => {
                        error!("Found a Persistent image when looking up a transient image size, this shouldn't happened (but I won't crash)");
                        persistent.get_image(*image_key).as_ref().unwrap().size
                    }
                    ImageResourceDescription::Transient(desc) => get_transient_image_size(
                        desc.size.clone(),
                        persistent,
                        images,
                        swapchain_images,
                    ),
                    ImageResourceDescription::Swapchain(swapchain_index) => {
                        swapchain_images[*swapchain_index].image.size
                    }
                },
            };
            extent.width = ((extent.width as f32) * scale[0]) as u32;
            extent.height = ((extent.height as f32) * scale[1]) as u32;

            extent
        }
    }
}
