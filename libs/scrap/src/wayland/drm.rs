use std::fs::File;
use std::io::Read;
use std::os::unix::io::{AsRawFd, RawFd};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hbb_common::{bail, ResultType};

use crate::wayland::capturable::{Capturable, PixelProvider, Recorder};

// Simple DRM framebuffer implementation for screen capture
pub struct DrmCapturable {
    pub path: String,
    pub width: usize,
    pub height: usize,
    pub format: u32,
}

impl DrmCapturable {
    pub fn new(path: &str) -> ResultType<Self> {
        // Try to detect actual display properties from fbset output
        match Command::new("fbset").arg("-fb").arg(path).output() {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                
                // Parse width, height, and format
                let width = Self::parse_fbset_value(&output_str, "geometry", 1)
                    .unwrap_or(1920);
                let height = Self::parse_fbset_value(&output_str, "geometry", 2)
                    .unwrap_or(1080);
                let format = Self::parse_fbset_format(&output_str)
                    .unwrap_or(0x34325241); // Default to AR24
                
                Ok(Self {
                    path: path.to_string(),
                    width,
                    height,
                    format,
                })
            },
            Err(_) => {
                // Fallback to default values if fbset is not available
                Ok(Self {
                    path: path.to_string(),
                    width: 1920,
                    height: 1080,
                    format: 0x34325241, // AR24 format
                })
            },
        }
    }
    
    /// Parse a value from fbset output
    fn parse_fbset_value(output: &str, section: &str, index: usize) -> Option<usize> {
        output.lines().find_map(|line| {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.first()?.eq_ignore_ascii_case(section) && parts.len() > index + 1 {
                parts[index + 1].parse::<usize>().ok()
            } else {
                None
            }
        })
    }
    
    /// Parse pixel format from fbset output
    fn parse_fbset_format(output: &str) -> Option<u32> {
        // Look for something like "R:   0 G:   8 B:  16 A:  24 bits"
        output.lines().find_map(|line| {
            if line.starts_with("R:") && line.contains("G:") && line.contains("B:") {
                // Extract bits per component
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 12 {
                    // Parse red offset
                    let red_off = parts[1].parse::<u32>().ok()?;
                    let green_off = parts[3].parse::<u32>().ok()?;
                    let blue_off = parts[5].parse::<u32>().ok()?;
                    let alpha_off = parts[7].parse::<u32>().ok()?;
                    
                    // Create DRM format (example: AR24 = 0x34325241)
                    Some(format_drm_fourcc(red_off, green_off, blue_off, alpha_off))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

impl Capturable for DrmCapturable {
    fn name(&self) -> String {
        format!("DRM: {}", self.path)
    }

    fn geometry_relative(&self) -> Result<(f64, f64, f64, f64), Box<dyn std::error::Error>> {
        // Return full screen geometry
        Ok((0.0, 0.0, 1.0, 1.0))
    }

    fn before_input(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // No action needed for DRM
        Ok(())
    }

    fn recorder(&self, capture_cursor: bool) -> Result<Box<dyn Recorder>, Box<dyn std::error::Error>> {
        Ok(Box::new(DrmRecorder::new(self.clone())?))
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct DrmRecorder {
    capturable: DrmCapturable,
    fb_file: File,
}

impl DrmRecorder {
    pub fn new(capturable: DrmCapturable) -> ResultType<Self> {
        // Try to open the correct framebuffer device based on the capturable path
        // If it's a DRM card, try to find the corresponding framebuffer
        let fb_path = if capturable.path.starts_with("/dev/dri/card") {
            // For DRM cards, try /dev/fb0 as default
            "/dev/fb0".to_string()
        } else {
            // Otherwise use the path directly (should be a framebuffer device)
            capturable.path.clone()
        };
        
        let fb_file = File::open(fb_path)?;
        Ok(Self {
            capturable,
            fb_file,
        })
    }
}

impl Recorder for DrmRecorder {
    fn capture(&mut self, timeout_ms: u64) -> Result<PixelProvider, Box<dyn std::error::Error>> {
        // Calculate buffer size based on format
        let bytes_per_pixel = match self.capturable.format {
            0x34325241 => 4, // AR24 (ARGB8888)
            0x41524742 => 4, // ABGR
            0x38384252 => 3, // R8G8B8
            0x38384242 => 3, // B8G8R8
            _ => 4, // Default to 4 bytes per pixel
        };
        
        let size = self.capturable.width * self.capturable.height * bytes_per_pixel;
        let mut buffer = vec![0u8; size];
        
        // Try to read from framebuffer
        match self.fb_file.read(&mut buffer) {
            Ok(actual_size) if actual_size > 0 => {
                // Ensure buffer is properly sized
                if actual_size < size {
                    buffer.resize(actual_size, 0);
                }
                
                // Convert to BGR0 format which is expected by the PixelProvider
                match self.capturable.format {
                    0x34325241 => {
                        // ARGB8888 to BGR0 conversion
                        let mut bgr0_buffer = vec![0u8; self.capturable.width * self.capturable.height * 4];
                        for i in 0..self.capturable.height {
                            for j in 0..self.capturable.width {
                                let src_idx = (i * self.capturable.width + j) * 4;
                                let dst_idx = (i * self.capturable.width + j) * 4;
                                if src_idx + 3 < buffer.len() {
                                    bgr0_buffer[dst_idx] = buffer[src_idx + 2]; // Blue
                                    bgr0_buffer[dst_idx + 1] = buffer[src_idx + 1]; // Green
                                    bgr0_buffer[dst_idx + 2] = buffer[src_idx]; // Red
                                    bgr0_buffer[dst_idx + 3] = buffer[src_idx + 3]; // Alpha
                                }
                            }
                        }
                        Ok(PixelProvider::BGR0(
                            self.capturable.width, 
                            self.capturable.height, 
                            &bgr0_buffer
                        ))
                    },
                    0x41524742 => {
                        // ABGR8888 to BGR0 conversion
                        let mut bgr0_buffer = vec![0u8; self.capturable.width * self.capturable.height * 4];
                        for i in 0..self.capturable.height {
                            for j in 0..self.capturable.width {
                                let src_idx = (i * self.capturable.width + j) * 4;
                                let dst_idx = (i * self.capturable.width + j) * 4;
                                if src_idx + 3 < buffer.len() {
                                    bgr0_buffer[dst_idx] = buffer[src_idx + 1]; // Blue
                                    bgr0_buffer[dst_idx + 1] = buffer[src_idx + 2]; // Green
                                    bgr0_buffer[dst_idx + 2] = buffer[src_idx + 3]; // Red
                                    bgr0_buffer[dst_idx + 3] = buffer[src_idx]; // Alpha
                                }
                            }
                        }
                        Ok(PixelProvider::BGR0(
                            self.capturable.width, 
                            self.capturable.height, 
                            &bgr0_buffer
                        ))
                    },
                    0x38384252 => {
                        // R8G8B8 to BGR0 conversion
                        let mut bgr0_buffer = vec![0u8; self.capturable.width * self.capturable.height * 4];
                        for i in 0..self.capturable.height {
                            for j in 0..self.capturable.width {
                                let src_idx = (i * self.capturable.width + j) * 3;
                                let dst_idx = (i * self.capturable.width + j) * 4;
                                if src_idx + 2 < buffer.len() {
                                    bgr0_buffer[dst_idx] = buffer[src_idx + 2]; // Blue
                                    bgr0_buffer[dst_idx + 1] = buffer[src_idx + 1]; // Green
                                    bgr0_buffer[dst_idx + 2] = buffer[src_idx]; // Red
                                    bgr0_buffer[dst_idx + 3] = 255; // Alpha (opaque)
                                }
                            }
                        }
                        Ok(PixelProvider::BGR0(
                            self.capturable.width, 
                            self.capturable.height, 
                            &bgr0_buffer
                        ))
                    },
                    0x38384242 => {
                        // B8G8R8 to BGR0 conversion
                        let mut bgr0_buffer = vec![0u8; self.capturable.width * self.capturable.height * 4];
                        for i in 0..self.capturable.height {
                            for j in 0..self.capturable.width {
                                let src_idx = (i * self.capturable.width + j) * 3;
                                let dst_idx = (i * self.capturable.width + j) * 4;
                                if src_idx + 2 < buffer.len() {
                                    bgr0_buffer[dst_idx] = buffer[src_idx]; // Blue
                                    bgr0_buffer[dst_idx + 1] = buffer[src_idx + 1]; // Green
                                    bgr0_buffer[dst_idx + 2] = buffer[src_idx + 2]; // Red
                                    bgr0_buffer[dst_idx + 3] = 255; // Alpha (opaque)
                                }
                            }
                        }
                        Ok(PixelProvider::BGR0(
                            self.capturable.width, 
                            self.capturable.height, 
                            &bgr0_buffer
                        ))
                    },
                    _ => {
                        // Unknown format, try to convert as ARGB8888
                        let mut bgr0_buffer = vec![0u8; self.capturable.width * self.capturable.height * 4];
                        for i in 0..self.capturable.height {
                            for j in 0..self.capturable.width {
                                let src_idx = (i * self.capturable.width + j) * bytes_per_pixel;
                                let dst_idx = (i * self.capturable.width + j) * 4;
                                if src_idx < buffer.len() {
                                    bgr0_buffer[dst_idx] = buffer[src_idx]; // Blue
                                    bgr0_buffer[dst_idx + 1] = buffer[src_idx]; // Green
                                    bgr0_buffer[dst_idx + 2] = buffer[src_idx]; // Red
                                    bgr0_buffer[dst_idx + 3] = 255; // Alpha (opaque)
                                }
                            }
                        }
                        Ok(PixelProvider::BGR0(
                            self.capturable.width, 
                            self.capturable.height, 
                            &bgr0_buffer
                        ))
                    },
                }
            },
            Ok(_) => {
                // Read 0 bytes, which means nothing was captured
                Ok(PixelProvider::NONE)
            },
            Err(e) => {
                Err(Box::new(e))
            },
        }
    }
}

/// Helper function to create DRM fourcc format from component offsets
fn format_drm_fourcc(red_off: u32, green_off: u32, blue_off: u32, alpha_off: u32) -> u32 {
    // This is a simplified implementation
    // In reality, we would need to handle different pixel formats properly
    match (red_off, green_off, blue_off, alpha_off) {
        // ARGB8888
        (16, 8, 0, 24) => 0x34325241, // AR24
        // BGRA8888
        (0, 8, 16, 24) => 0x41524742, // ABGR
        // RGB888
        (16, 8, 0, 0) => 0x38384252, // R8G8B8
        // BGR888
        (0, 8, 16, 0) => 0x38384242, // B8G8R8
        // Default to ARGB8888
        _ => 0x34325241,
    }
}

/// Helper function to create DRM fourcc format from component offsets
fn format_drm_fourcc(red_off: u32, green_off: u32, blue_off: u32, alpha_off: u32) -> u32 {
    // This is a simplified implementation
    // In reality, we would need to handle different pixel formats properly
    match (red_off, green_off, blue_off, alpha_off) {
        // ARGB8888
        (16, 8, 0, 24) => 0x34325241, // AR24
        // BGRA8888
        (0, 8, 16, 24) => 0x41524742, // ABGR
        // RGB888
        (16, 8, 0, 0) => 0x38384252, // R8G8B8
        // BGR888
        (0, 8, 16, 0) => 0x38384242, // B8G8R8
        // Default to ARGB8888
        _ => 0x34325241,
    }
}

// Simple evdev implementation for input control
pub struct DrmInputController {
    mouse_fd: Option<File>,
    keyboard_fd: Option<File>,
}

impl DrmInputController {
    pub fn new() -> ResultType<Self> {
        // In a real implementation, you would:
        // 1. Find the mouse and keyboard evdev devices
        // 2. Open them with O_RDWR
        // 3. Grab the devices for exclusive access
        Ok(Self {
            mouse_fd: None,
            keyboard_fd: None,
        })
    }

    pub fn send_mouse_move(&mut self, dx: i32, dy: i32) -> ResultType<()> {
        // In a real implementation, you would send EV_REL events
        Ok(())
    }

    pub fn send_mouse_click(&mut self, button: u32, pressed: bool) -> ResultType<()> {
        // In a real implementation, you would send EV_KEY events
        Ok(())
    }

    pub fn send_key(&mut self, keycode: u32, pressed: bool) -> ResultType<()> {
        // In a real implementation, you would send EV_KEY events
        Ok(())
    }
}