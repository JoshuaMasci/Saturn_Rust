use crate::resource_manager::{BufferHandle, ResourceManager, SamplerHandle, TextureHandle};
use crate::sampler::SamplerCreateInfo;
use crate::texture::{TextureBindingType, TextureUsage};
use crate::{BufferBindingType, BufferUsage};
use crate::{Error, PhysicalDevice};
use ash::vk;
use std::ffi::CStr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub struct Buffer {
    pub(crate) handle: BufferHandle,
    resource_manager: Arc<Mutex<ResourceManager>>,
}
impl Drop for Buffer {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_buffer(self.handle);
    }
}

pub struct Texture {
    pub(crate) handle: TextureHandle,
    resource_manager: Arc<Mutex<ResourceManager>>,
}
impl Drop for Texture {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_texture(self.handle);
    }
}

pub struct Sampler {
    pub(crate) handle: SamplerHandle,
    resource_manager: Arc<Mutex<ResourceManager>>,
}
impl Drop for Sampler {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_sampler(self.handle);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Integrated,
    Discrete,
    Unknown,
}

impl DeviceType {
    fn from_vk(device_type: vk::PhysicalDeviceType) -> Self {
        match device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => Self::Discrete,
            vk::PhysicalDeviceType::INTEGRATED_GPU => Self::Integrated,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceVendor {
    Amd,
    Arm,
    ImgTec,
    Intel,
    Nvidia,
    Qualcomm,
    Unknown(u32),
}

impl DeviceVendor {
    fn from_vk(vendor_id: u32) -> Self {
        match vendor_id {
            0x1002 => DeviceVendor::Amd,
            0x10DE => DeviceVendor::Nvidia,
            0x8086 => DeviceVendor::Intel,
            0x1010 => DeviceVendor::ImgTec,
            0x13B5 => DeviceVendor::Arm,
            0x5132 => DeviceVendor::Qualcomm,
            x => DeviceVendor::Unknown(x),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub vendor: DeviceVendor,
    pub device_type: DeviceType,
}

impl DeviceInfo {
    pub(crate) fn new(physical_device_properties: vk::PhysicalDeviceProperties) -> Self {
        Self {
            name: String::from(
                unsafe { CStr::from_ptr(physical_device_properties.device_name.as_ptr()) }
                    .to_str()
                    .expect("Failed to convert CStr to string"),
            ),
            vendor: DeviceVendor::from_vk(physical_device_properties.vendor_id),
            device_type: DeviceType::from_vk(physical_device_properties.device_type),
        }
    }
}

#[derive(Clone)]
pub struct AshDevice(ash::Device);
impl AshDevice {
    fn new(device: ash::Device) -> Self {
        Self(device)
    }
}

impl Deref for AshDevice {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for AshDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
            trace!("Drop Device");
        }
    }
}

pub struct Device {
    resource_manager: Arc<Mutex<ResourceManager>>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    device: Arc<AshDevice>,

    info: DeviceInfo,
    physical_device: vk::PhysicalDevice,

    graphics_queue: vk::Queue,
}

impl Device {
    pub(crate) fn new(
        instance: &ash::Instance,
        physical_device: &PhysicalDevice,
    ) -> crate::Result<Self> {
        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let mut robustness2_features = vk::PhysicalDeviceRobustness2FeaturesEXT::builder()
            .null_descriptor(true)
            .build();
        let mut vulkan1_2_features = vk::PhysicalDeviceVulkan12Features::builder()
            .descriptor_indexing(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_uniform_buffer_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .runtime_descriptor_array(true)
            .build();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(physical_device.graphics_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        let device = match unsafe {
            instance.create_device(
                physical_device.handle,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_info)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut synchronization2_features)
                    .push_next(&mut robustness2_features)
                    .push_next(&mut vulkan1_2_features),
                None,
            )
        } {
            Ok(device) => device,
            Err(e) => return Err(Error::VkError(e)),
        };

        let graphics_queue =
            unsafe { device.get_device_queue(physical_device.graphics_queue_family_index, 0) };

        let device = Arc::new(AshDevice::new(device));

        let allocator = match gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: (**device).clone(),
                physical_device: physical_device.handle,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: false,
            },
        ) {
            Ok(allocator) => Arc::new(Mutex::new(allocator)),
            Err(e) => return Err(Error::GpuAllocError(e)),
        };

        const FRAMES_IN_FLIGHT_COUNT: usize = 3;
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new(
            FRAMES_IN_FLIGHT_COUNT,
            device.clone(),
            allocator.clone(),
        )?));

        Ok(Self {
            info: physical_device.device_info.clone(),
            physical_device: physical_device.handle,
            device,
            allocator,
            resource_manager,
            graphics_queue,
        })
    }

    pub fn info(&self) -> DeviceInfo {
        self.info.clone()
    }

    pub fn create_buffer(
        &self,
        name: &str,
        usage: BufferUsage,
        binding: BufferBindingType,
        size: u64,
    ) -> crate::Result<Buffer> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_buffer(name, usage, binding, size)
            .map(|handle| Buffer {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_buffer_with_data(
        &self,
        name: &str,
        usage: BufferUsage,
        binding: BufferBindingType,
        data: &[u8],
    ) -> crate::Result<Buffer> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_buffer(name, usage, binding, data.len() as u64)
            .map(|handle| Buffer {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_texture(
        &self,
        name: &str,
        usage: TextureUsage,
        bindings: TextureBindingType,
        format: vk::Format,
        size: [u32; 2],
    ) -> crate::Result<Texture> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_texture(name, usage, bindings, format, size)
            .map(|handle| Texture {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_texture_with_data(
        &self,
        name: &str,
        usage: TextureUsage,
        bindings: TextureBindingType,
        format: vk::Format,
        size: [u32; 2],
        data: &[u8],
    ) -> crate::Result<Texture> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_texture(name, usage, bindings, format, size)
            .map(|handle| Texture {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_sampler(
        &self,
        name: &str,
        sampler_create_info: &SamplerCreateInfo,
    ) -> crate::Result<Sampler> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_sampler(name, sampler_create_info)
            .map(|handle| Sampler {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }
}
