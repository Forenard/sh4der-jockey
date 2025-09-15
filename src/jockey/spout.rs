use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::*;
use super::spout_native::SpoutReceiver;
use image::GenericImageView;

#[derive(Debug)]
pub struct Spout {
    receivers: HashMap<String, Arc<Mutex<Option<SpoutReceiverData>>>>,
    videos: HashMap<String, Arc<Mutex<image::DynamicImage>>>,
    disabled: bool,
}

#[derive(Debug)]
struct SpoutReceiverData {
    receiver: SpoutReceiver,
    width: u32,
    height: u32,
}

impl Spout {
    pub fn new() -> Self {
        let disabled = !cfg!(windows);

        if disabled {
            log::info!("Spout functionality is only available on Windows");
        }

        Self {
            receivers: HashMap::new(),
            videos: HashMap::new(),
            disabled,
        }
    }

    pub fn connect<I, T>(&mut self, requested: &I) -> Result<(), String>
    where
        I: ExactSizeIterator<Item = T> + Clone,
        T: AsRef<str>,
    {
        if self.disabled || requested.len() == 0 {
            return Ok(());
        }

        for source_name in requested.clone() {
            let source_name = source_name.as_ref().to_string();

            if self.receivers.contains_key(&source_name) {
                continue;
            }

            match self.create_receiver(&source_name) {
                Ok(()) => {
                    log::error!("=== SPOUT DEBUG: Successfully connected to Spout source: {}", source_name);
                }
                Err(e) => {
                    log::error!("=== SPOUT DEBUG: Failed to connect to Spout source '{}': {}", source_name, e);
                    return Err(format!("Failed to connect to any Spout sources: {}", e));
                }
            }
        }

        Ok(())
    }


    fn create_receiver(&mut self, source_name: &str) -> Result<(), String> {
        log::error!("=== SPOUT DEBUG: Creating receiver for '{}'", source_name);

        let mut receiver = SpoutReceiver::new()
            .map_err(|e| {
                let msg = format!("Failed to create Spout receiver: {}", e);
                log::error!("=== SPOUT DEBUG: {}", msg);
                msg
            })?;

        receiver.set_receiver_name(source_name);
        log::error!("=== SPOUT DEBUG: Set receiver name to '{}'", source_name);

        let receiver_data = Arc::new(Mutex::new(Some(SpoutReceiverData {
            receiver,
            width: 1,
            height: 1,
        })));

        let video = Arc::new(Mutex::new(image::DynamicImage::ImageRgba8(
            image::ImageBuffer::new(1, 1),
        )));

        self.receivers.insert(source_name.to_string(), receiver_data.clone());
        self.videos.insert(source_name.to_string(), video.clone());

        log::error!("=== SPOUT DEBUG: Receiver created successfully");

        let weak_receiver = Arc::downgrade(&receiver_data);
        let weak_video = Arc::downgrade(&video);
        let source_name_clone = source_name.to_string();

        thread::spawn(move || {
            let mut buffer: Vec<u8> = Vec::new();

            loop {
                if weak_receiver.strong_count() == 0 || weak_video.strong_count() == 0 {
                    break;
                }

                if let (Some(receiver_arc), Some(video_arc)) = (weak_receiver.upgrade(), weak_video.upgrade()) {
                    if let Some(mut receiver_data) = receiver_data.lock().unwrap().take() {
                        match Self::receive_texture(&mut receiver_data, &mut buffer) {
                            Ok(Some(img)) => {
                                log::error!("=== SPOUT DEBUG: Received texture {}x{} from '{}'",
                                           img.width(), img.height(), source_name_clone);
                                *video_arc.lock().unwrap() = img;
                                *receiver_arc.lock().unwrap() = Some(receiver_data);
                            }
                            Ok(None) => {
                                *receiver_arc.lock().unwrap() = Some(receiver_data);
                            }
                            Err(e) => {
                                log::error!("=== SPOUT DEBUG: Error receiving Spout texture from '{}': {}", source_name_clone, e);
                                break;
                            }
                        }
                    }
                } else {
                    break;
                }

                thread::sleep(Duration::from_millis(16)); // ~60 FPS
            }

            log::info!("Terminating Spout receiver thread for '{}'", source_name_clone);
        });

        Ok(())
    }

    fn receive_texture(
        receiver_data: &mut SpoutReceiverData,
        buffer: &mut Vec<u8>,
    ) -> Result<Option<image::DynamicImage>, String> {
        let mut width = 0u32;
        let mut height = 0u32;

        // Check if there's a new frame available
        if !receiver_data.receiver.check_receiver(&mut width, &mut height) {
            return Ok(None);
        }

        if width == 0 || height == 0 {
            return Ok(None);
        }

        // Resize buffer if needed
        let required_size = (width * height * 4) as usize; // RGBA
        if buffer.len() != required_size {
            buffer.resize(required_size, 0);
        }

        // Receive the texture data
        if receiver_data.receiver.receive_texture(buffer.as_mut_ptr(), width, height) {
            receiver_data.width = width;
            receiver_data.height = height;

            let img_buffer = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
                width,
                height,
                buffer.clone(),
            ).ok_or("Failed to create image buffer")?;

            Ok(Some(image::DynamicImage::ImageRgba8(img_buffer)))
        } else {
            Ok(None)
        }
    }

    pub fn update_texture(&self, tex_name: &String, tex: &mut Texture2D) {
        if let Some(video) = self.videos.get(tex_name) {
            let video = video.lock().unwrap().to_rgba8();
            log::error!("=== SPOUT DEBUG: Updating texture '{}' with {}x{} image",
                       tex_name, video.width(), video.height());

            if tex.resolution() != [video.width(), video.height(), 0] {
                log::error!("=== SPOUT DEBUG: Creating new texture {}x{}", video.width(), video.height());
                *tex = Texture2D::with_params(
                    [video.width(), video.height()],
                    tex.min_filter,
                    tex.mag_filter,
                    tex.wrap_mode,
                    tex.format,
                    tex.mipmap,
                    video.as_ptr() as _,
                );
            } else {
                log::error!("=== SPOUT DEBUG: Writing to existing texture");
                tex.write(video.as_ptr() as _);
            }
        } else {
            log::error!("=== SPOUT DEBUG: No video data found for texture '{}'", tex_name);
        }
    }

    pub fn cleanup(&mut self) {
        self.receivers.clear();
        self.videos.clear();
    }
}

impl Drop for Spout {
    fn drop(&mut self) {
        self.cleanup();
    }
}