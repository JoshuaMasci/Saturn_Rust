use ash::vk;
use std::ffi::CStr;

pub(crate) struct DebugMessenger {
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_call_back: vk::DebugUtilsMessengerEXT,
}

impl DebugMessenger {
    pub(crate) fn new(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let debug_call_back = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(
                    &vk::DebugUtilsMessengerCreateInfoEXT::builder()
                        .message_severity(
                            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                        )
                        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                        .pfn_user_callback(Some(vulkan_debug_callback)),
                    None,
                )
                .unwrap()
        };

        Self {
            debug_utils_loader,
            debug_call_back,
        }
    }
}

impl Drop for DebugMessenger {
    fn drop(&mut self) {
        //TODO: re-enable when drop works
        // unsafe {
        //     self.debug_utils_loader
        //         .destroy_debug_utils_messenger(self.debug_call_back, None);
        // }
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use std::borrow::Cow;
    let callback_data = *p_callback_data;
    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    if message_severity != vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        println!("Vulkan {:?}: {}", message_severity, message,);
    }

    vk::FALSE
}
