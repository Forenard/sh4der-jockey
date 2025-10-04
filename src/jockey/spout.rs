use std::ptr;
use gl::types::*;

#[cfg(target_os = "windows")]
#[path = "spout_ffi.rs"]
mod spout_ffi;

/// Spout sender for sharing OpenGL textures
pub struct SpoutSender {
    sender_name: String,
    width: u32,
    height: u32,
    share_handle: isize,
    initialized: bool,
    #[cfg(target_os = "windows")]
    ffi_sender: Option<spout_ffi::SpoutLibrarySender>,
}

impl SpoutSender {
    /// Create a new Spout sender
    pub fn new(name: &str) -> Self {
        log::info!("Creating Spout sender: {}", name);

        #[cfg(target_os = "windows")]
        let ffi_sender = match spout_ffi::SpoutLibrarySender::new(name) {
            Ok(sender) => {
                log::info!("Using SpoutLibrary.dll for Spout sending");
                Some(sender)
            }
            Err(e) => {
                log::warn!("Failed to initialize SpoutLibrary: {}", e);
                log::warn!("Falling back to basic OpenGL implementation");
                None
            }
        };

        Self {
            sender_name: name.to_string(),
            width: 0,
            height: 0,
            share_handle: 0,
            initialized: false,
            #[cfg(target_os = "windows")]
            ffi_sender,
        }
    }

    /// Initialize the sender with texture dimensions
    pub fn init(&mut self, width: u32, height: u32) -> std::result::Result<(), String> {
        if self.initialized && self.width == width && self.height == height {
            return Ok(());
        }

        log::info!("Initializing Spout sender '{}' with dimensions {}x{}",
            self.sender_name, width, height);

        self.width = width;
        self.height = height;

        // Create shared texture handle using OpenGL
        unsafe {
            let mut texture_id: GLuint = 0;
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as GLint,
                width as GLint,
                height as GLint,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                ptr::null(),
            );

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);

            // Get the shared handle for this texture
            // Note: This is a simplified approach. Full Spout implementation would use
            // wglDXOpenDeviceNV and DirectX interop for proper shared texture handles
            self.share_handle = texture_id as isize;

            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        self.initialized = true;
        log::info!("Spout sender '{}' initialized successfully", self.sender_name);
        Ok(())
    }

    /// Send a texture to Spout
    pub fn send_texture(&mut self, texture_id: GLuint, width: u32, height: u32) -> std::result::Result<(), String> {
        // Try using FFI sender first
        #[cfg(target_os = "windows")]
        if let Some(ffi) = &mut self.ffi_sender {
            return ffi.send_texture(texture_id, width, height);
        }

        // Fallback to basic OpenGL implementation
        if !self.initialized || self.width != width || self.height != height {
            self.init(width, height)?;
        }

        unsafe {
            // Copy the texture data
            // This is a fallback implementation that won't actually share with Spout receivers
            let mut fbo: GLuint = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(
                gl::READ_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                texture_id,
                0,
            );

            gl::BindTexture(gl::TEXTURE_2D, self.share_handle as GLuint);
            gl::CopyTexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                0,
                0,
                width as GLint,
                height as GLint,
            );

            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::DeleteFramebuffers(1, &fbo);

            // Check for OpenGL errors
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                return Err(format!("OpenGL error during texture copy: 0x{:X}", error));
            }
        }

        log::debug!("Sent texture {} ({}x{}) to Spout sender '{}' (fallback mode)",
            texture_id, width, height, self.sender_name);
        Ok(())
    }

    /// Get the sender name
    pub fn name(&self) -> &str {
        &self.sender_name
    }

    /// Check if the sender is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Release resources
    pub fn release(&mut self) {
        if self.initialized {
            unsafe {
                if self.share_handle != 0 {
                    let texture_id = self.share_handle as GLuint;
                    gl::DeleteTextures(1, &texture_id);
                }
            }
            self.share_handle = 0;
            self.initialized = false;
            log::info!("Released Spout sender '{}'", self.sender_name);
        }
    }
}

impl Drop for SpoutSender {
    fn drop(&mut self) {
        self.release();
    }
}

/// Spout configuration
#[derive(Debug, Clone)]
pub struct SpoutConfig {
    pub enabled: bool,
    pub sender_name: String,
}

impl Default for SpoutConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sender_name: "Sh4derJockey".to_string(),
        }
    }
}

impl SpoutConfig {
    /// Parse Spout configuration from YAML
    pub fn from_yaml(value: &serde_yaml::Value) -> std::result::Result<Self, String> {
        let mut config = Self::default();

        if let Some(enabled) = value.get("enabled") {
            config.enabled = enabled.as_bool()
                .ok_or("Spout 'enabled' must be a boolean")?;
        }

        if let Some(name) = value.get("name") {
            config.sender_name = name.as_str()
                .ok_or("Spout 'name' must be a string")?
                .to_string();
        }

        Ok(config)
    }
}
