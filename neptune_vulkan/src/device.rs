use crate::buffer::{Buffer, BufferDescription};
use crate::image::{Image, ImageDescription2D};
use crate::instance::AshInstance;
use crate::pipeline::RasterPipelineDescription;
use crate::render_graph::{BasicRenderGraphExecutor, BufferAccess, RenderGraph, RenderPass};
use crate::resource_managers::{PersistentResourceManager, TransientResourceManager};
use crate::swapchain::{SurfaceSettings, Swapchain, SwapchainManager};
use crate::{
    BufferHandle, ComputePipelineHandle, ImageHandle, RasterPipelineHandle, RasterPipleineKey,
    SurfaceHandle, VulkanError, VulkanFuture,
};
use ash::vk;
use log::error;
use slotmap::SlotMap;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct AshQueue {
    pub family_index: u32,
    pub handle: vk::Queue,
    pub flags: vk::QueueFlags,
}

pub struct AshRaytracing {
    pub acceleration_structure: ash::extensions::khr::AccelerationStructure,
    pub raytracing_pipeline: ash::extensions::khr::RayTracingPipeline,
}

pub struct AshDevice {
    pub instance: Arc<AshInstance>,
    pub physical: vk::PhysicalDevice,
    pub queues: Vec<AshQueue>,
    pub core: ash::Device,
    pub swapchain: ash::extensions::khr::Swapchain,
    pub mesh_shading: Option<ash::extensions::ext::MeshShader>,
    pub raytracing: Option<AshRaytracing>,
    pub allocator: ManuallyDrop<Mutex<gpu_allocator::vulkan::Allocator>>,
}

impl AshDevice {
    pub fn new(
        instance: Arc<AshInstance>,
        physical_device: vk::PhysicalDevice,
        queues_indices: &[u32],
    ) -> Result<Self, VulkanError> {
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = queues_indices
            .iter()
            .map(|family_index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*family_index)
                    .queue_priorities(&[1.0])
                    .build()
            })
            .collect();

        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut vulkan_1_1_features =
            vk::PhysicalDeviceVulkan12Features::builder().buffer_device_address(true);

        let mut vulkan_1_3_features = vk::PhysicalDeviceVulkan13Features::builder()
            .synchronization2(true)
            .dynamic_rendering(true);

        let core = unsafe {
            instance.core.create_device(
                physical_device,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_create_infos)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut vulkan_1_1_features)
                    .push_next(&mut vulkan_1_3_features)
                    .build(),
                None,
            )
        }?;

        let swapchain = ash::extensions::khr::Swapchain::new(&instance.core, &core);

        let queue_family_properties = unsafe {
            instance
                .core
                .get_physical_device_queue_family_properties(physical_device)
        };

        let queues = queues_indices
            .iter()
            .map(|&family_index| AshQueue {
                family_index,
                handle: unsafe { core.get_device_queue(family_index, 0) },
                flags: queue_family_properties[family_index as usize].queue_flags,
            })
            .collect();

        let allocator = ManuallyDrop::new(Mutex::new(gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.core.clone(),
                device: core.clone(),
                physical_device,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: true,
            },
        )?));

        Ok(Self {
            instance,
            physical: physical_device,
            queues,
            core,
            swapchain,
            mesh_shading: None,
            raytracing: None,
            allocator,
        })
    }
}

impl Drop for AshDevice {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.allocator);
            self.core.destroy_device(None);
        }
    }
}

pub struct DeviceSettings {
    pub frames_in_flight: usize,
}

pub struct Device {
    device: Arc<AshDevice>,

    pipeline_layout: vk::PipelineLayout,
    raster_pipelines: SlotMap<RasterPipleineKey, vk::Pipeline>,

    persistent_resource_manager: PersistentResourceManager,
    transient_resource_manager: TransientResourceManager,
    swapchain_manager: SwapchainManager,

    transfer_list: Vec<(BufferHandle, BufferHandle)>,
    graph_executor: BasicRenderGraphExecutor,
}

impl Device {
    pub fn new(
        instance: Arc<AshInstance>,
        physical_device: vk::PhysicalDevice,
        settings: &DeviceSettings,
    ) -> Result<Device, VulkanError> {
        let graphics_queue_index = 0;

        let device =
            AshDevice::new(instance, physical_device, &[graphics_queue_index]).map(Arc::new)?;
        let persistent_resource_manager = PersistentResourceManager::new(device.clone());
        let transient_resource_manager = TransientResourceManager::new(device.clone());
        let swapchain_manager = SwapchainManager::default();

        let graph_executor = BasicRenderGraphExecutor::new(device.clone(), graphics_queue_index)?;

        //TODO: bindless descriptor layout
        let pipeline_layout = unsafe {
            device.core.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder().push_constant_ranges(&[
                    vk::PushConstantRange {
                        stage_flags: vk::ShaderStageFlags::ALL,
                        offset: 0,
                        size: 128,
                    },
                ]),
                None,
            )?
        };

        Ok(Device {
            device,
            pipeline_layout,
            raster_pipelines: SlotMap::with_key(),
            persistent_resource_manager,
            transient_resource_manager,
            swapchain_manager,
            transfer_list: Vec::new(),
            graph_executor,
        })
    }

    pub fn create_buffer(
        &mut self,
        name: &str,
        description: &BufferDescription,
    ) -> Result<BufferHandle, VulkanError> {
        let buffer = Buffer::new(self.device.clone(), name, description)?;

        Ok(BufferHandle::Persistent(
            self.persistent_resource_manager.add_buffer(buffer),
        ))
    }
    pub fn destroy_buffer(&mut self, buffer_handle: BufferHandle) {
        match buffer_handle {
            BufferHandle::Persistent(key) => self.persistent_resource_manager.remove_buffer(key),
            BufferHandle::Transient(index) => {
                error!("Transient buffer {index} cannot be destroyed, this shouldn't happen")
            }
        }
    }
    pub fn update_data_to_buffer(
        &mut self,
        buffer_handle: BufferHandle,
        data: &[u8],
    ) -> Result<(), VulkanError> {
        let mut staging_buffer = Buffer::new(
            self.device.clone(),
            "Stating Buffer",
            &BufferDescription {
                size: data.len() as vk::DeviceSize,
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                location: gpu_allocator::MemoryLocation::CpuToGpu,
            },
        )?;

        let mut_slice = match staging_buffer.allocation.mapped_slice_mut() {
            None => return Err(VulkanError::Vk(vk::Result::ERROR_MEMORY_MAP_FAILED)),
            Some(mut_slice) => mut_slice,
        };
        mut_slice.copy_from_slice(data);

        let staging_handle =
            BufferHandle::Persistent(self.persistent_resource_manager.add_buffer(staging_buffer));

        self.transfer_list.push((staging_handle, buffer_handle));

        //Destroy stating buffer once frame is done
        self.destroy_buffer(staging_handle);

        Ok(())
    }
    pub fn create_buffer_init(
        &mut self,
        name: &str,
        usage: vk::BufferUsageFlags,
        location: gpu_allocator::MemoryLocation,
        data: &[u8],
    ) -> Result<BufferHandle, VulkanError> {
        let buffer = self.create_buffer(
            name,
            &BufferDescription {
                size: data.len() as vk::DeviceSize,
                usage,
                location,
            },
        )?;
        self.update_data_to_buffer(buffer, data)?;
        Ok(buffer)
    }

    pub fn create_image(
        &mut self,
        name: &str,
        description: &ImageDescription2D,
    ) -> Result<ImageHandle, VulkanError> {
        let image = Image::new_2d(self.device.clone(), name, description)?;

        Ok(ImageHandle::Persistent(
            self.persistent_resource_manager.add_image(image),
        ))
    }
    pub fn destroy_image(&mut self, image_handle: ImageHandle) {
        todo!()
    }

    pub fn create_compute_pipeline(
        &mut self,
    ) -> VulkanFuture<Result<ComputePipelineHandle, VulkanError>> {
        todo!()
    }
    pub fn destroy_compute_pipeline(&mut self, compute_pipeline_handle: ComputePipelineHandle) {
        todo!()
    }

    //TODO: allow multiple creation of multiple pipelines at once?
    //TODO: use vulkan future and some aync pipeline creation method to avoid pipeline creation in the main code paths
    pub fn create_raster_pipeline(
        &mut self,
        description: &RasterPipelineDescription,
    ) -> Result<RasterPipelineHandle, VulkanError> {
        let new_pipeline =
            crate::pipeline::create_pipeline(&self.device.core, self.pipeline_layout, description)?;
        Ok(RasterPipelineHandle(
            self.raster_pipelines.insert(new_pipeline),
        ))
    }
    pub fn destroy_raster_pipeline(&mut self, raster_pipeline_handle: RasterPipelineHandle) {
        if let Some(pipeline) = self.raster_pipelines.remove(raster_pipeline_handle.0) {
            unsafe {
                self.device.core.destroy_pipeline(pipeline, None);
            }
        }
    }

    pub fn configure_surface(
        &mut self,
        surface_handle: SurfaceHandle,
        settings: &SurfaceSettings,
    ) -> Result<(), VulkanError> {
        let surface = self
            .device
            .instance
            .surface_list
            .get(surface_handle.0)
            .unwrap();

        if let Some(swapchain) = self.swapchain_manager.get(surface) {
            swapchain.update_settings(settings)?;
        } else {
            self.swapchain_manager
                .add(Swapchain::new(self.device.clone(), surface, settings)?);
        }

        Ok(())
    }
    pub fn release_surface(&mut self, surface_handle: SurfaceHandle) {
        let surface = self
            .device
            .instance
            .surface_list
            .get(surface_handle.0)
            .unwrap();

        let _ = self.swapchain_manager.swapchains.remove(&surface);
    }

    pub fn submit_frame(&mut self, render_graph: &RenderGraph) -> Result<(), VulkanError> {
        let transfer_pass = (!self.transfer_list.is_empty()).then(|| {
            let mut buffer_usages = HashMap::new();

            for &(staging_handle, target_handle) in self.transfer_list.iter() {
                buffer_usages.insert(
                    staging_handle,
                    BufferAccess {
                        write: false,
                        stage: vk::PipelineStageFlags2::TRANSFER,
                        access: vk::AccessFlags2::TRANSFER_READ,
                    },
                );
                buffer_usages.insert(
                    target_handle,
                    BufferAccess {
                        write: true,
                        stage: vk::PipelineStageFlags2::TRANSFER,
                        access: vk::AccessFlags2::TRANSFER_WRITE,
                    },
                );
            }
            let transfer_list = std::mem::take(&mut self.transfer_list);
            RenderPass {
                name: "Transfer Pass".to_string(),
                queue: Default::default(),
                buffer_usages,
                image_usages: Default::default(),
                framebuffer: None,
                build_cmd_fn: Some(Box::new(move |device, command_buffer, resources| {
                    for &(staging_handle, target_handle) in transfer_list.iter() {
                        let staging_buffer = resources.get_buffer(staging_handle);
                        let target_buffer = resources.get_buffer(target_handle);
                        unsafe {
                            device.core.cmd_copy_buffer2(
                                command_buffer,
                                &vk::CopyBufferInfo2::builder()
                                    .src_buffer(staging_buffer.handle)
                                    .dst_buffer(target_buffer.handle)
                                    .regions(&[vk::BufferCopy2::builder()
                                        .src_offset(0)
                                        .dst_offset(0)
                                        .size(staging_buffer.size)
                                        .build()]),
                            );
                        }
                    }
                })),
            }
        });

        self.graph_executor.execute_graph(
            transfer_pass,
            render_graph,
            &mut self.persistent_resource_manager,
            &mut self.transient_resource_manager,
            &mut self.swapchain_manager,
            &self.raster_pipelines,
        )?;
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.core.device_wait_idle();

            for (_key, pipeline) in self.raster_pipelines.iter() {
                self.device.core.destroy_pipeline(*pipeline, None);
            }

            self.device
                .core
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
