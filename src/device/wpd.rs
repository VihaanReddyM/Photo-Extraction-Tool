//! Device module for Windows Portable Devices (WPD) enumeration and management
//!
//! This module handles discovering connected iOS devices (iPhone/iPad) and managing
//! connections to them through the Windows Portable Devices API.
//! No iTunes installation or additional drivers are required on Windows 10/11.
//!
//! It implements the device traits from `traits.rs` to enable testability
//! by allowing mock devices to be substituted for real iOS devices.

use crate::core::error::{ExtractionError, Result};
use crate::device::traits::{
    DeviceContentTrait, DeviceInfo, DeviceManagerTrait, DeviceObject, DeviceSimulationConfig,
};
use log::{debug, info, trace, warn};
use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::{Arc, RwLock};
use windows::{
    core::{GUID, PCWSTR, PWSTR},
    Win32::{
        Devices::PortableDevices::{
            IEnumPortableDeviceObjectIDs, IPortableDevice, IPortableDeviceContent,
            IPortableDeviceKeyCollection, IPortableDeviceManager, IPortableDeviceProperties,
            IPortableDeviceValues, PortableDeviceFTM, PortableDeviceKeyCollection,
            PortableDeviceManager, PortableDeviceValues, WPD_CLIENT_MAJOR_VERSION,
            WPD_CLIENT_MINOR_VERSION, WPD_CLIENT_NAME, WPD_CLIENT_REVISION,
            WPD_CLIENT_SECURITY_QUALITY_OF_SERVICE, WPD_OBJECT_CONTENT_TYPE, WPD_OBJECT_NAME,
            WPD_OBJECT_ORIGINAL_FILE_NAME, WPD_OBJECT_SIZE, WPD_RESOURCE_DEFAULT,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IStream,
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
        },
    },
};

/// GUID for folder content type
const WPD_CONTENT_TYPE_FOLDER: GUID = GUID::from_u128(0x27e2e392_a111_48e0_ab0c_e17705a05f85);

/// GUID for functional object content type (used for storage objects like "Internal Storage")
const WPD_CONTENT_TYPE_FUNCTIONAL_OBJECT: GUID =
    GUID::from_u128(0x99ed0160_17ff_4c44_9d98_1d7a6f941921);

/// Manager for device enumeration and connection
///
/// This is the real WPD implementation that talks to actual connected devices.
pub struct DeviceManager {
    manager: IPortableDeviceManager,
    /// Cache of device info for quick lookups
    device_cache: Arc<RwLock<HashMap<String, DeviceInfo>>>,
}

// DeviceManager is not Send/Sync due to COM, but we can mark it as such
// because we only use it from a single thread at a time in practice.
// The cache is protected by RwLock for interior mutability.
unsafe impl Send for DeviceManager {}
unsafe impl Sync for DeviceManager {}

impl DeviceManager {
    /// Create a new DeviceManager (COM must already be initialized)
    pub fn new() -> Result<Self> {
        unsafe {
            // Create the portable device manager
            let manager: IPortableDeviceManager =
                CoCreateInstance(&PortableDeviceManager, None, CLSCTX_INPROC_SERVER).map_err(
                    |e| {
                        ExtractionError::ComError(format!("Failed to create device manager: {}", e))
                    },
                )?;

            Ok(Self {
                manager,
                device_cache: Arc::new(RwLock::new(HashMap::new())),
            })
        }
    }

    /// Get device friendly name
    fn get_device_friendly_name(&self, device_id: &str) -> Result<String> {
        unsafe {
            let device_id_wide: Vec<u16> =
                device_id.encode_utf16().chain(std::iter::once(0)).collect();

            let mut length: u32 = 0;
            let _ = self.manager.GetDeviceFriendlyName(
                PCWSTR(device_id_wide.as_ptr()),
                PWSTR::null(),
                &mut length,
            );

            if length == 0 {
                return Err(ExtractionError::DeviceError(
                    "Property not found".to_string(),
                ));
            }

            let mut buffer: Vec<u16> = vec![0; length as usize];
            self.manager
                .GetDeviceFriendlyName(
                    PCWSTR(device_id_wide.as_ptr()),
                    PWSTR(buffer.as_mut_ptr()),
                    &mut length,
                )
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to get friendly name: {}", e))
                })?;

            Ok(String::from_utf16_lossy(&buffer[..length as usize - 1]))
        }
    }

    /// Get device manufacturer
    fn get_device_manufacturer(&self, device_id: &str) -> Result<String> {
        unsafe {
            let device_id_wide: Vec<u16> =
                device_id.encode_utf16().chain(std::iter::once(0)).collect();

            let mut length: u32 = 0;
            let _ = self.manager.GetDeviceManufacturer(
                PCWSTR(device_id_wide.as_ptr()),
                PWSTR::null(),
                &mut length,
            );

            if length == 0 {
                return Err(ExtractionError::DeviceError(
                    "Property not found".to_string(),
                ));
            }

            let mut buffer: Vec<u16> = vec![0; length as usize];
            self.manager
                .GetDeviceManufacturer(
                    PCWSTR(device_id_wide.as_ptr()),
                    PWSTR(buffer.as_mut_ptr()),
                    &mut length,
                )
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to get manufacturer: {}", e))
                })?;

            Ok(String::from_utf16_lossy(&buffer[..length as usize - 1]))
        }
    }

    /// Get device description/model
    fn get_device_description(&self, device_id: &str) -> Result<String> {
        unsafe {
            let device_id_wide: Vec<u16> =
                device_id.encode_utf16().chain(std::iter::once(0)).collect();

            let mut length: u32 = 0;
            let _ = self.manager.GetDeviceDescription(
                PCWSTR(device_id_wide.as_ptr()),
                PWSTR::null(),
                &mut length,
            );

            if length == 0 {
                return Err(ExtractionError::DeviceError(
                    "Property not found".to_string(),
                ));
            }

            let mut buffer: Vec<u16> = vec![0; length as usize];
            self.manager
                .GetDeviceDescription(
                    PCWSTR(device_id_wide.as_ptr()),
                    PWSTR(buffer.as_mut_ptr()),
                    &mut length,
                )
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to get description: {}", e))
                })?;

            Ok(String::from_utf16_lossy(&buffer[..length as usize - 1]))
        }
    }

    /// Open a connection to a device and return a PortableDevice instance
    fn open_device_internal(&self, device_id: &str) -> Result<PortableDevice> {
        unsafe {
            // Create the portable device object
            let device: IPortableDevice =
                CoCreateInstance(&PortableDeviceFTM, None, CLSCTX_INPROC_SERVER).map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to create device object: {}", e))
                })?;

            // Create client information
            let client_info: IPortableDeviceValues =
                CoCreateInstance(&PortableDeviceValues, None, CLSCTX_INPROC_SERVER).map_err(
                    |e| {
                        ExtractionError::DeviceError(format!("Failed to create client info: {}", e))
                    },
                )?;

            // Set client info properties
            let client_name: Vec<u16> = "Photo Extraction Tool"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            client_info
                .SetStringValue(&WPD_CLIENT_NAME, PCWSTR(client_name.as_ptr()))
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to set client name: {}", e))
                })?;

            client_info
                .SetUnsignedIntegerValue(&WPD_CLIENT_MAJOR_VERSION, 1)
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to set major version: {}", e))
                })?;

            client_info
                .SetUnsignedIntegerValue(&WPD_CLIENT_MINOR_VERSION, 0)
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to set minor version: {}", e))
                })?;

            client_info
                .SetUnsignedIntegerValue(&WPD_CLIENT_REVISION, 0)
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to set revision: {}", e))
                })?;

            client_info
                .SetUnsignedIntegerValue(&WPD_CLIENT_SECURITY_QUALITY_OF_SERVICE, 0x00020000)
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to set security QOS: {}", e))
                })?;

            // Open the device
            let device_id_wide: Vec<u16> =
                device_id.encode_utf16().chain(std::iter::once(0)).collect();
            device
                .Open(PCWSTR(device_id_wide.as_ptr()), &client_info)
                .map_err(|e| {
                    if e.code().0 as u32 == 0x80070005 {
                        // E_ACCESSDENIED
                        ExtractionError::AccessDenied
                    } else {
                        ExtractionError::DeviceError(format!("Failed to open device: {}", e))
                    }
                })?;

            info!("Successfully opened device: {}", device_id);

            Ok(PortableDevice {
                device,
                device_id: device_id.to_string(),
            })
        }
    }
}

impl DeviceManagerTrait for DeviceManager {
    type Content = DeviceContent;

    fn enumerate_apple_devices(&self) -> Result<Vec<DeviceInfo>> {
        let all_devices = self.enumerate_all_devices()?;

        let apple_devices: Vec<DeviceInfo> = all_devices
            .into_iter()
            .filter(|d| {
                d.manufacturer.to_lowercase().contains("apple")
                    || d.model.to_lowercase().contains("iphone")
                    || d.model.to_lowercase().contains("ipad")
                    || d.friendly_name.to_lowercase().contains("iphone")
                    || d.friendly_name.to_lowercase().contains("ipad")
                    || d.friendly_name.to_lowercase().contains("apple")
            })
            .collect();

        Ok(apple_devices)
    }

    fn enumerate_all_devices(&self) -> Result<Vec<DeviceInfo>> {
        unsafe {
            // Refresh device list first
            let _ = self.manager.RefreshDeviceList();

            // Get the count of devices
            let mut device_count: u32 = 0;
            self.manager
                .GetDevices(null_mut(), &mut device_count)
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to get device count: {}", e))
                })?;

            if device_count == 0 {
                return Ok(Vec::new());
            }

            // Allocate buffer for device IDs
            let mut device_ids: Vec<PWSTR> = vec![PWSTR::null(); device_count as usize];
            self.manager
                .GetDevices(device_ids.as_mut_ptr(), &mut device_count)
                .map_err(|e| {
                    ExtractionError::DeviceError(format!("Failed to enumerate devices: {}", e))
                })?;

            let mut devices = Vec::new();

            for device_id_ptr in device_ids.iter().take(device_count as usize) {
                if device_id_ptr.is_null() {
                    continue;
                }

                let device_id = device_id_ptr.to_string().unwrap_or_default();

                let friendly_name = self
                    .get_device_friendly_name(&device_id)
                    .unwrap_or_else(|_| "Unknown Device".to_string());
                let manufacturer = self
                    .get_device_manufacturer(&device_id)
                    .unwrap_or_else(|_| "Unknown".to_string());
                let model = self
                    .get_device_description(&device_id)
                    .unwrap_or_else(|_| "Unknown".to_string());

                let info = DeviceInfo {
                    device_id: device_id.clone(),
                    friendly_name,
                    manufacturer,
                    model,
                };

                // Cache the device info
                if let Ok(mut cache) = self.device_cache.write() {
                    cache.insert(device_id, info.clone());
                }

                devices.push(info);

                // Free the device ID string
                CoTaskMemFree(Some(device_id_ptr.0 as *const _));
            }

            Ok(devices)
        }
    }

    fn open_device(&self, device_id: &str) -> Result<Self::Content> {
        let portable_device = self.open_device_internal(device_id)?;
        portable_device.get_content()
    }

    fn get_device_info(&self, device_id: &str) -> Option<DeviceInfo> {
        // Check cache first
        if let Ok(cache) = self.device_cache.read() {
            if let Some(info) = cache.get(device_id) {
                return Some(info.clone());
            }
        }

        // Try to get fresh info
        let friendly_name = self
            .get_device_friendly_name(device_id)
            .unwrap_or_else(|_| "Unknown Device".to_string());
        let manufacturer = self
            .get_device_manufacturer(device_id)
            .unwrap_or_else(|_| "Unknown".to_string());
        let model = self
            .get_device_description(device_id)
            .unwrap_or_else(|_| "Unknown".to_string());

        Some(DeviceInfo {
            device_id: device_id.to_string(),
            friendly_name,
            manufacturer,
            model,
        })
    }

    fn device_count(&self) -> usize {
        self.enumerate_all_devices().map(|d| d.len()).unwrap_or(0)
    }
}

/// Represents an open connection to a portable device
#[allow(dead_code)]
pub struct PortableDevice {
    device: IPortableDevice,
    /// The device ID for this connection
    pub device_id: String,
}

impl PortableDevice {
    /// Get a DeviceContent interface for browsing device contents
    pub fn get_content(&self) -> Result<DeviceContent> {
        unsafe {
            let content = self.device.Content().map_err(|e| {
                ExtractionError::ContentError(format!("Failed to get device content: {}", e))
            })?;

            Ok(DeviceContent::new(content))
        }
    }
}

impl Drop for PortableDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.Close();
        }
    }
}

/// Provides access to device content (files and folders)
///
/// This is the real WPD implementation that talks to actual connected devices.
pub struct DeviceContent {
    content: IPortableDeviceContent,
    /// Cache of objects for path building
    object_cache: Arc<RwLock<HashMap<String, DeviceObject>>>,
    /// Parent relationships for path building
    parent_cache: Arc<RwLock<HashMap<String, String>>>,
}

// DeviceContent is not Send/Sync due to COM, but we mark it as such
// because we only use it from a single thread at a time in practice.
unsafe impl Send for DeviceContent {}
unsafe impl Sync for DeviceContent {}

impl DeviceContent {
    /// Create a new DeviceContent wrapper
    fn new(content: IPortableDeviceContent) -> Self {
        Self {
            content,
            object_cache: Arc::new(RwLock::new(HashMap::new())),
            parent_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the inner IPortableDeviceContent interface
    pub fn inner(&self) -> &IPortableDeviceContent {
        &self.content
    }

    /// Parse object properties from IPortableDeviceValues
    fn parse_object_properties(
        &self,
        object_id: &str,
        parent_id: &str,
        values: &IPortableDeviceValues,
    ) -> DeviceObject {
        unsafe {
            // Get name (try original filename first, then object name)
            let name = self
                .get_string_value(values, &WPD_OBJECT_ORIGINAL_FILE_NAME)
                .or_else(|_| self.get_string_value(values, &WPD_OBJECT_NAME))
                .unwrap_or_else(|_| "Unknown".to_string());

            // Check if it's a folder or a functional object (like "Internal Storage")
            let is_folder = if let Ok(content_type) = values.GetGuidValue(&WPD_OBJECT_CONTENT_TYPE)
            {
                trace!("Object '{}' content type GUID: {:?}", name, content_type);
                content_type == WPD_CONTENT_TYPE_FOLDER
                    || content_type == WPD_CONTENT_TYPE_FUNCTIONAL_OBJECT
            } else {
                // If we can't get the content type, check if the name suggests it's a container
                let name_upper = name.to_uppercase();
                name_upper.contains("STORAGE")
                    || name_upper.contains("INTERNAL")
                    || name_upper == "DCIM"
            };

            // Get size
            let size = values
                .GetUnsignedLargeIntegerValue(&WPD_OBJECT_SIZE)
                .unwrap_or(0);

            DeviceObject {
                object_id: object_id.to_string(),
                parent_id: parent_id.to_string(),
                name,
                is_folder,
                size,
                date_modified: None,
                content_type: None,
            }
        }
    }

    /// Get a string value from IPortableDeviceValues
    fn get_string_value(
        &self,
        values: &IPortableDeviceValues,
        key: &windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY,
    ) -> Result<String> {
        unsafe {
            let pwstr = values.GetStringValue(key).map_err(|e| {
                ExtractionError::ContentError(format!("Failed to get string value: {}", e))
            })?;

            let result = pwstr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(pwstr.0 as *const _));
            Ok(result)
        }
    }

    /// Cache an object and its parent relationship
    fn cache_object(&self, object: &DeviceObject) {
        if let Ok(mut cache) = self.object_cache.write() {
            cache.insert(object.object_id.clone(), object.clone());
        }
        if let Ok(mut parent_cache) = self.parent_cache.write() {
            parent_cache.insert(object.object_id.clone(), object.parent_id.clone());
        }
    }
}

impl DeviceContentTrait for DeviceContent {
    fn enumerate_objects(&self) -> Result<Vec<DeviceObject>> {
        debug!("Enumerating root objects (parent: DEVICE)");
        let objects = self.enumerate_children("DEVICE")?;

        for obj in &objects {
            info!(
                "Root object: '{}' (id: {}, folder: {}, size: {})",
                obj.name, obj.object_id, obj.is_folder, obj.size
            );
        }

        Ok(objects)
    }

    fn enumerate_children(&self, parent_id: &str) -> Result<Vec<DeviceObject>> {
        trace!("Enumerating children of: {}", parent_id);

        unsafe {
            let parent_id_wide: Vec<u16> =
                parent_id.encode_utf16().chain(std::iter::once(0)).collect();

            // Get the enumeration interface
            let enum_objects: IEnumPortableDeviceObjectIDs = self
                .content
                .EnumObjects(0, PCWSTR(parent_id_wide.as_ptr()), None)
                .map_err(|e| {
                    warn!("Failed to enumerate objects in '{}': {}", parent_id, e);
                    ExtractionError::ContentError(format!("Failed to enumerate objects: {}", e))
                })?;

            // Get properties interface
            let properties: IPortableDeviceProperties = self.content.Properties().map_err(|e| {
                ExtractionError::ContentError(format!("Failed to get properties: {}", e))
            })?;

            // Create key collection for properties we want
            let keys_to_read: IPortableDeviceKeyCollection =
                CoCreateInstance(&PortableDeviceKeyCollection, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| {
                        ExtractionError::ContentError(format!(
                            "Failed to create key collection: {}",
                            e
                        ))
                    })?;

            keys_to_read.Add(&WPD_OBJECT_NAME)?;
            keys_to_read.Add(&WPD_OBJECT_ORIGINAL_FILE_NAME)?;
            keys_to_read.Add(&WPD_OBJECT_CONTENT_TYPE)?;
            keys_to_read.Add(&WPD_OBJECT_SIZE)?;

            let mut objects = Vec::new();
            let batch_size = 100u32;

            loop {
                let mut object_ids: [PWSTR; 100] = [PWSTR::null(); 100];
                let mut fetched: u32 = 0;

                let result = enum_objects.Next(
                    &mut object_ids[..batch_size as usize],
                    &mut fetched as *mut u32,
                );

                if fetched == 0 {
                    break;
                }

                for object_id_ptr in object_ids.iter().take(fetched as usize) {
                    if object_id_ptr.is_null() {
                        continue;
                    }

                    let object_id = object_id_ptr.to_string().unwrap_or_default();
                    trace!("Found object ID: {}", object_id);

                    // Get object properties
                    let object_id_wide: Vec<u16> =
                        object_id.encode_utf16().chain(std::iter::once(0)).collect();

                    match properties.GetValues(PCWSTR(object_id_wide.as_ptr()), &keys_to_read) {
                        Ok(values) => {
                            let device_object =
                                self.parse_object_properties(&object_id, parent_id, &values);
                            trace!(
                                "  -> '{}' (folder: {}, size: {})",
                                device_object.name,
                                device_object.is_folder,
                                device_object.size
                            );
                            self.cache_object(&device_object);
                            objects.push(device_object);
                        }
                        Err(e) => {
                            warn!("Failed to get properties for object '{}': {}", object_id, e);
                        }
                    }

                    // Free the object ID string
                    CoTaskMemFree(Some(object_id_ptr.0 as *const _));
                }

                if result.is_err() {
                    break;
                }
            }

            debug!("Found {} objects in '{}'", objects.len(), parent_id);

            if objects.is_empty() && parent_id == "DEVICE" {
                warn!("No objects found at root level. This might indicate:");
                warn!("  - The iOS device is locked — please unlock it");
                warn!("  - You haven't tapped 'Trust' on the device");
                warn!("  - Try a different USB cable or port");
            }

            Ok(objects)
        }
    }

    fn read_file(&self, object_id: &str) -> Result<Vec<u8>> {
        unsafe {
            // Get the resources interface
            let resources = self.content.Transfer().map_err(|e| {
                ExtractionError::ContentError(format!("Failed to get transfer interface: {}", e))
            })?;

            // Convert object ID to wide string
            let object_id_wide: Vec<u16> =
                object_id.encode_utf16().chain(std::iter::once(0)).collect();

            // Get the stream for reading - STGM_READ = 0
            let mut optimal_buffer_size: u32 = 0;
            let mut stream_opt: Option<IStream> = None;

            resources
                .GetStream(
                    PCWSTR(object_id_wide.as_ptr()),
                    &WPD_RESOURCE_DEFAULT,
                    0, // STGM_READ
                    &mut optimal_buffer_size,
                    &mut stream_opt,
                )
                .map_err(|e| {
                    ExtractionError::ContentError(format!("Failed to get file stream: {}", e))
                })?;

            let stream = stream_opt.ok_or_else(|| {
                ExtractionError::ContentError("Failed to get stream: stream is None".to_string())
            })?;

            // Use a reasonable buffer size
            let buffer_size = if optimal_buffer_size > 0 && optimal_buffer_size <= 1048576 {
                optimal_buffer_size as usize
            } else {
                262144 // 256KB default
            };

            let mut data = Vec::new();
            let mut buffer = vec![0u8; buffer_size];

            loop {
                let mut bytes_read: u32 = 0;
                let result = stream.Read(
                    buffer.as_mut_ptr() as *mut _,
                    buffer_size as u32,
                    Some(&mut bytes_read),
                );

                if bytes_read == 0 {
                    break;
                }

                data.extend_from_slice(&buffer[..bytes_read as usize]);

                // Check if we've read all data or encountered an error
                if result.is_err() || bytes_read < buffer_size as u32 {
                    break;
                }
            }

            Ok(data)
        }
    }

    fn get_object(&self, object_id: &str) -> Result<Option<DeviceObject>> {
        // Check cache first
        if let Ok(cache) = self.object_cache.read() {
            if let Some(obj) = cache.get(object_id) {
                return Ok(Some(obj.clone()));
            }
        }

        // Object not in cache - we can't easily fetch a single object with WPD
        // without knowing its parent, so return None
        Ok(None)
    }

    fn get_object_path(&self, object_id: &str) -> Option<String> {
        let mut parts = Vec::new();
        let mut current_id = object_id.to_string();

        // Try to build path from cache
        loop {
            if current_id == "DEVICE" {
                break;
            }

            // Get object name
            if let Ok(cache) = self.object_cache.read() {
                if let Some(obj) = cache.get(&current_id) {
                    parts.push(obj.name.clone());
                    current_id = obj.parent_id.clone();
                    continue;
                }
            }

            // Not found in cache
            break;
        }

        if parts.is_empty() {
            None
        } else {
            parts.reverse();
            Some(parts.join("/"))
        }
    }
}

/// RAII guard for COM initialization
pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    /// Initialize COM library
    pub fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| {
                    ExtractionError::ComError(format!("Failed to initialize COM: {}", e))
                })?;

            Ok(Self { initialized: true })
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// Initialize COM and return a guard that will uninitialize on drop
pub fn initialize_com() -> Result<ComGuard> {
    ComGuard::new()
}

/// Enumerate all Apple devices (convenience function)
///
/// This is a convenience wrapper that creates a DeviceManager
/// and enumerates Apple devices in one call.
#[allow(dead_code)]
pub fn enumerate_devices() -> Result<Vec<DeviceInfo>> {
    let manager = DeviceManager::new()?;
    manager.enumerate_apple_devices()
}

// =============================================================================
// Legacy type aliases for backward compatibility
// =============================================================================

/// Legacy DeviceInfo type alias - prefer using traits::DeviceInfo
#[deprecated(note = "Use crate::device::DeviceInfo from traits module instead")]
pub type LegacyDeviceInfo = DeviceInfo;

/// Legacy DeviceObject type alias - prefer using traits::DeviceObject
#[deprecated(note = "Use crate::device::DeviceObject from traits module instead")]
pub type LegacyDeviceObject = DeviceObject;
