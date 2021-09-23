use ash::*;

pub struct SwapchainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupportDetails {
    pub fn new(
        pdevice: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
    ) -> Self {
        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(pdevice, surface)
                .unwrap()
        };

        let formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(pdevice, surface)
                .unwrap()
        };

        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(pdevice, surface)
                .unwrap()
        };

        Self {
            capabilities,
            formats,
            present_modes,
        }
    }

    pub fn get_size(&self, desired_size: vk::Extent2D) -> vk::Extent2D {
        if self.capabilities.current_extent.width != u32::MAX {
            return self.capabilities.current_extent;
        }

        vk::Extent2D::builder()
            .width(u32::clamp(
                desired_size.width,
                self.capabilities.min_image_extent.width,
                self.capabilities.max_image_extent.width,
            ))
            .height(u32::clamp(
                desired_size.height,
                self.capabilities.min_image_extent.height,
                self.capabilities.max_image_extent.height,
            ))
            .build()
    }

    pub fn get_format(&self, desired_format: vk::Format) -> vk::SurfaceFormatKHR {
        *self
            .formats
            .iter()
            .find(|surface_format| surface_format.format == desired_format)
            .unwrap_or(&self.formats[0])
    }

    pub fn get_present_mode(&self, desired_mode: vk::PresentModeKHR) -> vk::PresentModeKHR {
        *self
            .present_modes
            .iter()
            .find(|&&present_mode| present_mode == desired_mode)
            .unwrap_or(&self.present_modes[0])
    }

    pub fn get_image_count(&self, desired_count: u32) -> u32 {
        u32::clamp(
            desired_count,
            self.capabilities.min_image_count,
            self.capabilities.max_image_count,
        )
    }
}

pub struct Swapchain {
    pdevice: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
    pub(crate) loader: ash::extensions::khr::Swapchain,
    pub(crate) handle: vk::SwapchainKHR,

    pub(crate) format: vk::Format,
    pub(crate) size: vk::Extent2D,
    pub(crate) mode: vk::PresentModeKHR,
    pub(crate) images: Vec<vk::Image>,
}

impl Swapchain {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        pdevice: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: ash::extensions::khr::Surface,
    ) -> Self {
        let loader = ash::extensions::khr::Swapchain::new(instance, device);

        //Temp values
        let handle = vk::SwapchainKHR::null();
        let format = vk::Format::UNDEFINED;
        let size = vk::Extent2D::builder().build();
        let mode = vk::PresentModeKHR::FIFO;
        let images = Vec::new();

        let mut new = Self {
            pdevice,
            surface,
            surface_loader,
            loader,
            handle,
            format,
            size,
            mode,
            images,
        };
        new.rebuild();
        new
    }

    fn rebuild(&mut self) {
        let swapchain_support =
            SwapchainSupportDetails::new(self.pdevice, self.surface, &self.surface_loader);

        let present_mode = swapchain_support.get_present_mode(vk::PresentModeKHR::MAILBOX);
        let surface_format = swapchain_support.get_format(vk::Format::B8G8R8A8_UNORM);
        let image_count = swapchain_support.get_image_count(3);

        //TODO: get size
        let surface_size = swapchain_support.get_size(vk::Extent2D::builder().build());

        let old_swapchain = self.handle;

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_size)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(swapchain_support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain)
            .build();

        self.handle = unsafe { self.loader.create_swapchain(&create_info, None) }
            .expect("Failed to create swapchain!");

        self.format = surface_format.format;
        self.size = surface_size;
        self.mode = present_mode;

        self.images = unsafe { self.loader.get_swapchain_images(self.handle) }
            .expect("Failed to get swapchain images");

        unsafe {
            self.loader.destroy_swapchain(old_swapchain, None);
        }
    }

    pub fn acquire_next_image(&mut self, image_ready_semaphore: vk::Semaphore) -> u32 {
        loop {
            let (index, suboptimal) = unsafe {
                self.loader
                    .acquire_next_image(
                        self.handle,
                        u64::MAX,
                        image_ready_semaphore,
                        vk::Fence::null(),
                    )
                    .unwrap_or((0, true))
            };

            if !suboptimal {
                return index;
            }

            self.rebuild();
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}
