#[cfg(windows)]
use std::{
    mem::size_of,
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
    slice,
};

#[cfg(windows)]
use winapi::{
    shared::{
        minwindef::{DWORD, HKEY},
        winerror::{ERROR_SUCCESS, S_OK},
        ntdef::HANDLE,
    },
    um::{
        combaseapi::{CoInitializeEx, CoUninitialize},
        d3d11::{
            D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
            D3D11_SDK_VERSION, ID3D11Resource,
            D3D11_USAGE_STAGING, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ,
            D3D11_TEXTURE2D_DESC, D3D11_MAPPED_SUBRESOURCE,
        },
        d3dcommon::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0},
        objbase::COINIT_APARTMENTTHREADED,
        winreg::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER},
        memoryapi::{OpenFileMappingW, MapViewOfFile, UnmapViewOfFile},
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        memoryapi::FILE_MAP_READ,
        errhandlingapi::GetLastError,
    },
    Interface,
};

#[cfg(windows)]
pub struct SpoutReceiver {
    sender_name: String,
    width: u32,
    height: u32,
    d3d_device: Option<*mut ID3D11Device>,
    d3d_context: Option<*mut ID3D11DeviceContext>,
    shared_texture: Option<*mut ID3D11Texture2D>,
    shared_handle: Option<usize>,
}

#[cfg(windows)]
impl std::fmt::Debug for SpoutReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpoutReceiver")
            .field("sender_name", &self.sender_name)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

#[cfg(windows)]
unsafe impl Send for SpoutReceiver {}

#[cfg(windows)]
unsafe impl Sync for SpoutReceiver {}

#[cfg(windows)]
impl Drop for SpoutReceiver {
    fn drop(&mut self) {
        unsafe {
            if let Some(texture) = self.shared_texture {
                (*texture).Release();
            }
            if let Some(context) = self.d3d_context {
                (*context).Release();
            }
            if let Some(device) = self.d3d_device {
                (*device).Release();
            }
            CoUninitialize();
        }
    }
}

#[cfg(windows)]
impl SpoutReceiver {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let hr = CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED);
            // S_OK = 0, S_FALSE = 1, RPC_E_CHANGED_MODE = 0x80010106
            if hr != S_OK && hr != 1 && hr != 0x80010106u32 as i32 {
                return Err(format!("Failed to initialize COM: 0x{:08x}", hr));
            }
            log::error!("=== SPOUT DEBUG: COM initialized successfully (hr: 0x{:08x})", hr);

            // Create D3D11 device
            let mut device: *mut ID3D11Device = null_mut();
            let mut context: *mut ID3D11DeviceContext = null_mut();
            let mut feature_level: D3D_FEATURE_LEVEL = D3D_FEATURE_LEVEL_11_0;

            let hr = D3D11CreateDevice(
                null_mut(),                    // adapter
                D3D_DRIVER_TYPE_HARDWARE,      // driver type
                null_mut(),                    // software
                0,                             // flags
                [D3D_FEATURE_LEVEL_11_0].as_ptr(), // feature levels
                1,                             // num feature levels
                D3D11_SDK_VERSION,             // SDK version
                &mut device,                   // device
                &mut feature_level,            // feature level
                &mut context,                  // context
            );

            if hr != S_OK {
                CoUninitialize();
                return Err(format!("Failed to create D3D11 device: 0x{:08x}", hr));
            }

            Ok(SpoutReceiver {
                sender_name: String::new(),
                width: 0,
                height: 0,
                d3d_device: Some(device),
                d3d_context: Some(context),
                shared_texture: None,
                shared_handle: None,
            })
        }
    }

    pub fn set_receiver_name(&mut self, name: &str) -> bool {
        self.sender_name = name.to_string();
        true
    }

    pub fn check_receiver(&mut self, width: &mut u32, height: &mut u32) -> bool {
        if self.sender_name.is_empty() {
            log::error!("=== SPOUT DEBUG: Sender name is empty");
            return false;
        }

        // Try to get sender info from registry
        match self.get_sender_info(&self.sender_name) {
            Some((w, h, handle)) => {
                log::error!("=== SPOUT DEBUG: Found Spout sender '{}': {}x{}, handle: 0x{:x}",
                          self.sender_name, w, h, handle);
                if w != self.width || h != self.height || self.shared_handle.is_none() {
                    self.width = w;
                    self.height = h;
                    self.shared_handle = Some(handle);

                    // Create shared texture
                    if let Err(e) = self.create_shared_texture() {
                        log::error!("=== SPOUT DEBUG: Failed to create shared texture: {}", e);
                        return false;
                    }
                }
                *width = self.width;
                *height = self.height;
                true
            }
            None => {
                log::error!("=== SPOUT DEBUG: No Spout sender '{}' found", self.sender_name);
                false
            }
        }
    }

    pub fn receive_texture(&mut self, pixels: *mut u8, width: u32, height: u32) -> bool {
        if width != self.width || height != self.height {
            log::error!("=== SPOUT DEBUG: Size mismatch: expected {}x{}, got {}x{}",
                       self.width, self.height, width, height);
            return false;
        }

        unsafe {
            if let (Some(_device), Some(_context)) = (self.d3d_device, self.d3d_context) {
                let pixel_count = (width * height * 4) as usize;
                let pixel_buffer = slice::from_raw_parts_mut(pixels, pixel_count);

                // Only try to read actual Spout texture data
                if self.shared_handle.is_some() && self.shared_handle.unwrap() != 0 {
                    log::error!("=== SPOUT DEBUG: Reading from shared texture handle: 0x{:x}",
                               self.shared_handle.unwrap());

                    return self.read_spout_texture(pixel_buffer, width, height);
                } else {
                    log::error!("=== SPOUT DEBUG: No shared texture handle available");
                    return false;
                }
            } else {
                log::error!("=== SPOUT DEBUG: D3D11 resources not available");
                false
            }
        }
    }

    fn read_spout_texture(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        unsafe {
            if let (Some(device), Some(context)) = (self.d3d_device, self.d3d_context) {
                if let Some(shared_handle) = self.shared_handle {
                    if shared_handle == 0 {
                        return false;
                    }

                    log::error!("=== SPOUT DEBUG: Opening shared texture with handle: 0x{:x}", shared_handle);

                    // Open the shared texture using the handle
                    // Spout uses DXGI shared handles which are HANDLE values
                    let d3d_handle = shared_handle as HANDLE;
                    log::error!("=== SPOUT DEBUG: Opening shared texture with HANDLE: 0x{:x}", d3d_handle as usize);

                    let mut shared_texture: *mut ID3D11Texture2D = null_mut();
                    let hr = (*device).OpenSharedResource(
                        d3d_handle,
                        &ID3D11Texture2D::uuidof(),
                        &mut shared_texture as *mut *mut ID3D11Texture2D as *mut *mut winapi::ctypes::c_void,
                    );

                    if hr != S_OK {
                        log::error!("=== SPOUT DEBUG: Failed to open shared texture: 0x{:08x}", hr);

                        // Try alternative handle interpretations
                        log::error!("=== SPOUT DEBUG: Trying alternative handle formats...");

                        // Try as pointer value directly
                        let alt_handle = shared_handle as *mut winapi::ctypes::c_void;
                        let hr2 = (*device).OpenSharedResource(
                            alt_handle as HANDLE,
                            &ID3D11Texture2D::uuidof(),
                            &mut shared_texture as *mut *mut ID3D11Texture2D as *mut *mut winapi::ctypes::c_void,
                        );

                        if hr2 != S_OK {
                            log::error!("=== SPOUT DEBUG: Alternative handle format also failed: 0x{:08x}", hr2);
                            return false;
                        } else {
                            log::error!("=== SPOUT DEBUG: Alternative handle format succeeded!");
                        }
                    }

                    log::error!("=== SPOUT DEBUG: Successfully opened shared texture");

                    // Create a staging texture to read the data
                    let mut texture_desc = std::mem::zeroed::<D3D11_TEXTURE2D_DESC>();
                    (*shared_texture).GetDesc(&mut texture_desc);

                    texture_desc.Usage = D3D11_USAGE_STAGING;
                    texture_desc.BindFlags = 0;
                    texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
                    texture_desc.MiscFlags = 0;

                    let mut staging_texture: *mut ID3D11Texture2D = null_mut();
                    let hr = (*device).CreateTexture2D(
                        &texture_desc,
                        null_mut(),
                        &mut staging_texture,
                    );

                    if hr != S_OK {
                        log::error!("=== SPOUT DEBUG: Failed to create staging texture: 0x{:08x}", hr);
                        (*shared_texture).Release();
                        return false;
                    }

                    // Copy the shared texture to staging texture
                    (*context).CopyResource(
                        staging_texture as *mut ID3D11Resource,
                        shared_texture as *mut ID3D11Resource,
                    );

                    // Map the staging texture to read pixel data
                    let mut mapped_resource = std::mem::zeroed::<D3D11_MAPPED_SUBRESOURCE>();
                    let hr = (*context).Map(
                        staging_texture as *mut ID3D11Resource,
                        0,
                        D3D11_MAP_READ,
                        0,
                        &mut mapped_resource,
                    );

                    if hr != S_OK {
                        log::error!("=== SPOUT DEBUG: Failed to map staging texture: 0x{:08x}", hr);
                        (*staging_texture).Release();
                        (*shared_texture).Release();
                        return false;
                    }

                    log::error!("=== SPOUT DEBUG: Successfully mapped texture, reading pixel data");

                    // Copy pixel data from mapped resource to our buffer
                    let src_data = mapped_resource.pData as *const u8;
                    let src_pitch = mapped_resource.RowPitch as usize;
                    let bytes_per_pixel = 4; // RGBA

                    for y in 0..height {
                        let src_row = src_data.add(y as usize * src_pitch);
                        let dst_row = pixels.as_mut_ptr().add(y as usize * width as usize * bytes_per_pixel);
                        std::ptr::copy_nonoverlapping(src_row, dst_row, width as usize * bytes_per_pixel);
                    }

                    // Unmap and cleanup
                    (*context).Unmap(staging_texture as *mut ID3D11Resource, 0);
                    (*staging_texture).Release();
                    (*shared_texture).Release();

                    log::error!("=== SPOUT DEBUG: Successfully read {} bytes from TestSpoutSender", pixels.len());
                    return true;
                }
            }
        }

        false
    }

    fn get_sender_info(&self, sender_name: &str) -> Option<(u32, u32, usize)> {
        // Spout uses memory mapping instead of registry for sender info
        // Let's first check if we can find the sender in active memory mappings
        log::error!("=== SPOUT DEBUG: Looking for sender '{}'", sender_name);

        // Try standard Spout registry paths
        let paths = [
            format!("SOFTWARE\\Leading Edge\\Spout\\{}", sender_name),
            format!("SOFTWARE\\WOW6432Node\\Leading Edge\\Spout\\{}", sender_name),
        ];

        for path in &paths {
            log::error!("=== SPOUT DEBUG: Checking registry path: {}", path);
            if let Some(result) = self.try_registry_path(path) {
                log::error!("=== SPOUT DEBUG: Found sender info in registry");
                return Some(result);
            }
        }

        // If not found in registry, try to use memory mapping approach
        // This is more accurate for active Spout senders
        self.get_sender_from_memory_map(sender_name)
    }

    fn get_sender_from_memory_map(&self, sender_name: &str) -> Option<(u32, u32, usize)> {
        log::error!("=== SPOUT DEBUG: Looking up sender info for '{}'", sender_name);

        unsafe {
            // First try to read from SpoutSenderNames to get the complete sender list
            if let Some(result) = self.read_from_sender_names(sender_name) {
                return Some(result);
            }

            // Fallback: try individual sender memory mapping
            let individual_names = vec![
                format!("{}", sender_name),
                format!("Local\\{}", sender_name),
                format!("Global\\{}", sender_name),
            ];

            for memory_name in &individual_names {
                log::error!("=== SPOUT DEBUG: Trying individual sender mapping: '{}'", memory_name);
                if let Some(result) = self.read_individual_sender(memory_name, sender_name) {
                    return Some(result);
                }
            }

            None
        }
    }

    fn read_from_sender_names(&self, sender_name: &str) -> Option<(u32, u32, usize)> {
        let memory_names = ["SpoutSenderNames", "Local\\SpoutSenderNames"];

        for memory_name in &memory_names {
            log::error!("=== SPOUT DEBUG: Attempting to access sender list: '{}'", memory_name);
            if let Some(result) = self.scan_sender_list(memory_name, sender_name) {
                return Some(result);
            }
        }

        // Try to enumerate all available Spout senders
        self.enumerate_all_senders();
        None
    }

    fn enumerate_all_senders(&self) {
        log::error!("=== SPOUT DEBUG: Enumerating all available Spout senders...");

        let memory_names = [
            "SpoutSenderNames",
            "Local\\SpoutSenderNames",
            "Global\\SpoutSenderNames"
        ];

        for memory_name in &memory_names {
            self.dump_sender_list(memory_name);
        }
    }

    fn dump_sender_list(&self, memory_name: &str) {
        unsafe {
            let memory_name_wide: Vec<u16> = memory_name
                .encode_utf16()
                .chain(Some(0))
                .collect();

            let h_map = OpenFileMappingW(FILE_MAP_READ, 0, memory_name_wide.as_ptr());
            if h_map.is_null() || h_map == INVALID_HANDLE_VALUE {
                let error = GetLastError();
                log::error!("=== SPOUT DEBUG: Cannot access '{}', error: {}", memory_name, error);
                return;
            }

            let mapped_memory = MapViewOfFile(h_map, FILE_MAP_READ, 0, 0, 0);
            if mapped_memory.is_null() {
                log::error!("=== SPOUT DEBUG: Cannot map view of '{}'", memory_name);
                CloseHandle(h_map);
                return;
            }

            log::error!("=== SPOUT DEBUG: Successfully opened '{}', scanning for senders...", memory_name);

            // Try to read any valid sender names found in memory
            let data_ptr = mapped_memory as *const u8;
            for offset in (0..4096).step_by(256) {
                let name_ptr = data_ptr.add(offset);
                let mut name_bytes = Vec::new();

                for i in 0..256 {
                    let byte = *name_ptr.add(i);
                    if byte == 0 {
                        break;
                    }
                    if byte.is_ascii_graphic() || byte == b' ' {
                        name_bytes.push(byte);
                    } else {
                        break;
                    }
                }

                if name_bytes.len() > 3 {
                    if let Ok(name) = std::str::from_utf8(&name_bytes) {
                        if !name.trim().is_empty() {
                            log::error!("=== SPOUT DEBUG: Found potential sender name: '{}'", name.trim());
                        }
                    }
                }
            }

            UnmapViewOfFile(mapped_memory);
            CloseHandle(h_map);
        }
    }

    fn scan_sender_list(&self, memory_name: &str, target_sender: &str) -> Option<(u32, u32, usize)> {
        unsafe {
            let memory_name_wide: Vec<u16> = memory_name
                .encode_utf16()
                .chain(Some(0))
                .collect();

            let h_map = OpenFileMappingW(FILE_MAP_READ, 0, memory_name_wide.as_ptr());
            if h_map.is_null() || h_map == INVALID_HANDLE_VALUE {
                let error = GetLastError();
                log::error!("=== SPOUT DEBUG: Failed to open memory map '{}', error: {}", memory_name, error);
                return None;
            }

            let mapped_memory = MapViewOfFile(h_map, FILE_MAP_READ, 0, 0, 0);
            if mapped_memory.is_null() {
                CloseHandle(h_map);
                return None;
            }

            log::error!("=== SPOUT DEBUG: Scanning sender list in '{}'", memory_name);

            // Spout sender info structure (based on Spout SDK)
            #[repr(C)]
            #[derive(Copy, Clone)]
            struct SpoutSenderInfo {
                name: [u8; 256],           // Sender name
                width: u32,                // Texture width
                height: u32,               // Texture height
                handle: u32,               // Shared texture handle (D3D11)
                format: u32,               // Texture format
                usage: u32,                // Usage flags
                description: [u8; 512],    // Optional description
            }

            let max_senders = 64; // Typical Spout limit
            let base_ptr = mapped_memory as *const SpoutSenderInfo;

            for i in 0..max_senders {
                let sender_info = base_ptr.add(i);
                let info = *sender_info;

                // Check if this entry has valid data
                if info.width > 0 && info.width <= 8192 && info.height > 0 && info.height <= 8192 {
                    // Convert name to string
                    let name_bytes: Vec<u8> = info.name.iter()
                        .take_while(|&&b| b != 0)
                        .copied()
                        .collect();

                    if let Ok(name) = std::str::from_utf8(&name_bytes) {
                        log::error!("=== SPOUT DEBUG: Found sender '{}': {}x{}, handle: 0x{:x}",
                                   name, info.width, info.height, info.handle);

                        if name == target_sender && info.handle != 0 {
                            log::error!("=== SPOUT DEBUG: Target sender '{}' found!", target_sender);

                            UnmapViewOfFile(mapped_memory);
                            CloseHandle(h_map);

                            return Some((info.width, info.height, info.handle as usize));
                        }
                    }
                }
            }

            UnmapViewOfFile(mapped_memory);
            CloseHandle(h_map);
            None
        }
    }

    fn read_individual_sender(&self, memory_name: &str, sender_name: &str) -> Option<(u32, u32, usize)> {
        unsafe {
            let memory_name_wide: Vec<u16> = memory_name
                .encode_utf16()
                .chain(Some(0))
                .collect();

            let h_map = OpenFileMappingW(FILE_MAP_READ, 0, memory_name_wide.as_ptr());
            if h_map.is_null() || h_map == INVALID_HANDLE_VALUE {
                let error = GetLastError();
                log::error!("=== SPOUT DEBUG: Failed to open memory map '{}', error: {}", memory_name, error);
                return None;
            }

            let mapped_memory = MapViewOfFile(h_map, FILE_MAP_READ, 0, 0, 0);
            if mapped_memory.is_null() {
                CloseHandle(h_map);
                return None;
            }

            log::error!("=== SPOUT DEBUG: Reading individual sender mapping for '{}'", memory_name);

            // Individual sender memory structure
            #[repr(C)]
            #[derive(Copy, Clone)]
            struct SpoutTexture {
                width: u32,
                height: u32,
                format: u32,
                usage: u32,
                share_handle: u32,
                adapter_luid: u64,
                padding: [u8; 256],
            }

            let texture_info = mapped_memory as *const SpoutTexture;
            let info = *texture_info;

            if info.width > 0 && info.width <= 8192 && info.height > 0 && info.height <= 8192 && info.share_handle != 0 {
                log::error!("=== SPOUT DEBUG: Individual sender data: {}x{}, handle: 0x{:x}",
                           info.width, info.height, info.share_handle);

                UnmapViewOfFile(mapped_memory);
                CloseHandle(h_map);

                return Some((info.width, info.height, info.share_handle as usize));
            }

            UnmapViewOfFile(mapped_memory);
            CloseHandle(h_map);
            None
        }
    }

    fn try_registry_path(&self, subkey: &str) -> Option<(u32, u32, usize)> {
        unsafe {
            let mut hkey: HKEY = null_mut();
            let subkey_wide: Vec<u16> = subkey
                .encode_utf16()
                .chain(Some(0))
                .collect();

            let result = RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey_wide.as_ptr(),
                0,
                0x20019, // KEY_READ
                &mut hkey,
            );

            if result != ERROR_SUCCESS as i32 {
                log::debug!("Failed to open registry key {}: error {}", subkey, result);
                return None;
            }

            let width = self.read_registry_dword(hkey, "Width")?;
            let height = self.read_registry_dword(hkey, "Height")?;
            let handle = self.read_registry_dword(hkey, "Handle")? as usize;

            RegCloseKey(hkey);

            log::info!("Successfully read from registry: {}x{}, handle: 0x{:x}", width, height, handle);
            Some((width, height, handle))
        }
    }


    fn read_registry_dword(&self, hkey: HKEY, value_name: &str) -> Option<u32> {
        unsafe {
            let value_wide: Vec<u16> = value_name
                .encode_utf16()
                .chain(Some(0))
                .collect();

            let mut data: DWORD = 0;
            let mut data_size = size_of::<DWORD>() as DWORD;
            let mut data_type: DWORD = 0;

            let result = RegQueryValueExW(
                hkey,
                value_wide.as_ptr(),
                null_mut(),
                &mut data_type,
                &mut data as *mut DWORD as *mut u8,
                &mut data_size,
            );

            if result == ERROR_SUCCESS as i32 {
                Some(data)
            } else {
                None
            }
        }
    }

    fn create_shared_texture(&mut self) -> Result<(), String> {
        // Shared texture will be opened on-demand in read_spout_texture
        // based on the handle from memory mapping
        Ok(())
    }
}

#[cfg(not(windows))]
#[derive(Debug)]
pub struct SpoutReceiver;

#[cfg(not(windows))]
impl SpoutReceiver {
    pub fn new() -> Result<Self, String> {
        Err("Spout is only supported on Windows".to_string())
    }

    pub fn set_receiver_name(&mut self, _name: &str) -> bool {
        false
    }

    pub fn check_receiver(&mut self, _width: &mut u32, _height: &mut u32) -> bool {
        false
    }

    pub fn receive_texture(&mut self, _pixels: *mut u8, _width: u32, _height: u32) -> bool {
        false
    }
}