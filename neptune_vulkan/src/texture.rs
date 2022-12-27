use crate::{AshDevice, Error};
use ash::vk;
use bitflags::bitflags;
use std::sync::{Arc, Mutex};

bitflags! {
    pub struct TextureUsage: u32 {
        const ATTACHMENT = 1 << 0;
    }
}

bitflags! {
    pub struct TextureBindingType: u32 {
        const SAMPLED = 1 << 0;
        const STORAGE = 1 << 0;
    }
}

pub(crate) fn get_vk_texture_2d_create_info(
    usage: TextureUsage,
    bindings: TextureBindingType,
    format: vk::Format,
    size: [u32; 2],
) -> vk::ImageCreateInfo {
    let mut vk_usage = vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST;

    if bindings.contains(TextureBindingType::SAMPLED) {
        vk_usage |= vk::ImageUsageFlags::SAMPLED;
    }

    if bindings.contains(TextureBindingType::STORAGE) {
        vk_usage |= vk::ImageUsageFlags::STORAGE;
    }

    let is_color_format = true;
    if usage.contains(TextureUsage::ATTACHMENT) {
        vk_usage |= match is_color_format {
            true => vk::ImageUsageFlags::COLOR_ATTACHMENT,
            false => vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        };
    }

    vk::ImageCreateInfo::builder()
        .format(format)
        .image_type(vk::ImageType::TYPE_2D)
        .usage(vk_usage)
        .extent(vk::Extent3D {
            width: size[0],
            height: size[1],
            depth: 1,
        })
        .array_layers(1)
        .mip_levels(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .build()
}

#[derive(Default, Debug)]
pub struct AshImage {
    pub handle: vk::Image,
    pub allocation: gpu_allocator::vulkan::Allocation,
    //TODO: Bindings
}

impl AshImage {
    pub(crate) fn new(
        device: &Arc<AshDevice>,
        allocator: &Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        create_info: &vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> crate::Result<Self> {
        let handle = match unsafe { device.create_image(create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(Error::VkError(e)),
        };

        let requirements = unsafe { device.get_image_memory_requirements(handle) };

        let allocation =
            match allocator
                .lock()
                .unwrap()
                .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                    name: "Image Allocation",
                    requirements,
                    location: memory_location,
                    linear: true,
                }) {
                Ok(allocation) => allocation,
                Err(e) => {
                    unsafe { device.destroy_image(handle, None) };
                    return Err(Error::GpuAllocError(e));
                }
            };

        if let Err(e) =
            unsafe { device.bind_image_memory(handle, allocation.memory(), allocation.offset()) }
        {
            unsafe { device.destroy_image(handle, None) };
            let _ = allocator.lock().unwrap().free(allocation);
            return Err(Error::VkError(e));
        }

        Ok(Self { allocation, handle })
    }

    pub(crate) fn destroy(
        &mut self,
        device: &Arc<AshDevice>,
        allocator: &Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    ) {
        unsafe { device.destroy_image(self.handle, None) };
        let _ = allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
        trace!("Destroy Texture");
    }
}
