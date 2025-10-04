// FFI bindings for SpoutLibrary.dll
use std::ffi::CString;
use std::os::raw::{c_char, c_uint, c_void};
use libloading::{Library, Symbol};
use std::sync::OnceLock;

static SPOUT_LIB: OnceLock<Option<Library>> = OnceLock::new();

fn get_spout_lib() -> Option<&'static Library> {
    SPOUT_LIB.get_or_init(|| {
        // Try to load SpoutLibrary.dll
        unsafe {
            Library::new("SpoutLibrary.dll")
                .or_else(|_| Library::new("./SpoutLibrary.dll"))
                .ok()
        }
    }).as_ref()
}

// SPOUTLIBRARY is an opaque handle to the Spout library instance
type SpoutHandle = *mut c_void;

// Factory function to get SPOUTLIBRARY instance
type GetSpoutFn = unsafe extern "C" fn() -> SpoutHandle;

// Virtual table for SPOUTLIBRARY methods
// Based on exact order from SpoutLibrary.h
#[repr(C)]
struct SpoutVTable {
    // Sender methods (in exact order from header)
    set_sender_name: unsafe extern "C" fn(SpoutHandle, *const c_char),           // 0
    set_sender_format: unsafe extern "C" fn(SpoutHandle, u32),                    // 1
    release_sender: unsafe extern "C" fn(SpoutHandle, u32),                       // 2
    send_fbo: unsafe extern "C" fn(SpoutHandle, c_uint, c_uint, c_uint, bool) -> bool, // 3
    send_texture: unsafe extern "C" fn(SpoutHandle, c_uint, c_uint, c_uint, c_uint, bool, c_uint) -> bool, // 4
    // ... other virtual methods omitted
}

pub struct SpoutLibrarySender {
    name: CString,
    width: u32,
    height: u32,
    initialized: bool,
    spout_handle: Option<SpoutHandle>,
}

impl SpoutLibrarySender {
    pub fn new(name: &str) -> Result<Self, String> {
        let name_c = CString::new(name)
            .map_err(|e| format!("Invalid sender name: {}", e))?;

        // Get Spout instance handle
        let lib = get_spout_lib().ok_or("SpoutLibrary.dll not found")?;
        let spout_handle = unsafe {
            let get_spout: Symbol<GetSpoutFn> = lib
                .get(b"GetSpout\0")
                .map_err(|e| format!("Failed to get GetSpout function: {}", e))?;

            let handle = get_spout();
            if handle.is_null() {
                return Err("Failed to get Spout instance".to_string());
            }
            handle
        };

        log::info!("Got Spout instance handle");

        Ok(Self {
            name: name_c,
            width: 0,
            height: 0,
            initialized: false,
            spout_handle: Some(spout_handle),
        })
    }

    pub fn init(&mut self, width: u32, height: u32) -> Result<(), String> {
        if self.initialized && self.width == width && self.height == height {
            return Ok(());
        }

        let handle = self.spout_handle.ok_or("No Spout handle")?;

        unsafe {
            let vtable = *(handle as *const *const SpoutVTable);

            // If already initialized with different size, release first
            if self.initialized && (self.width != width || self.height != height) {
                log::info!("Spout sender '{}' resolution changed from {}x{} to {}x{}, releasing...",
                    self.name.to_str().unwrap(), self.width, self.height, width, height);
                let release_sender = (*vtable).release_sender;
                release_sender(handle, 0);
                self.initialized = false;
            }

            // Set sender name (creates sender on first SendTexture call)
            let set_sender_name = (*vtable).set_sender_name;
            set_sender_name(handle, self.name.as_ptr());
        }

        self.width = width;
        self.height = height;
        self.initialized = true;

        log::info!("Spout sender '{}' configured ({}x{})",
            self.name.to_str().unwrap(), width, height);
        Ok(())
    }

    pub fn send_texture(&mut self, texture_id: u32, width: u32, height: u32) -> Result<(), String> {
        if !self.initialized || self.width != width || self.height != height {
            self.init(width, height)?;
        }

        let handle = self.spout_handle.ok_or("No Spout handle")?;

        unsafe {
            let vtable = *(handle as *const *const SpoutVTable);
            let send_texture = (*vtable).send_texture;

            const GL_TEXTURE_2D: u32 = 0x0DE1;
            if !send_texture(
                handle,
                texture_id,
                GL_TEXTURE_2D,
                width,
                height,
                false, // Don't invert
                0,     // No host FBO
            ) {
                return Err("Failed to send texture to Spout".to_string());
            }
        }

        log::debug!("Sent texture {} ({}x{}) to Spout sender '{}'",
            texture_id, width, height, self.name.to_str().unwrap());
        Ok(())
    }

    pub fn name(&self) -> &str {
        self.name.to_str().unwrap()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn release(&mut self) {
        if self.initialized {
            if let Some(handle) = self.spout_handle {
                unsafe {
                    let vtable = *(handle as *const *const SpoutVTable);
                    let release_sender = (*vtable).release_sender;
                    release_sender(handle, 0);
                }
            }
            self.initialized = false;
            log::info!("Released Spout sender '{}'", self.name.to_str().unwrap());
        }
    }
}

impl Drop for SpoutLibrarySender {
    fn drop(&mut self) {
        self.release();
    }
}
