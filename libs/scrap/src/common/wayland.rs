use crate::{
    wayland::{capturable::*, *},
    Frame, TraitCapturer,
};
use std::{io, sync::RwLock, time::Duration};

use super::x11::PixelBuffer;

pub struct Capturer(Display, Box<dyn Recorder>, Vec<u8>);

lazy_static::lazy_static! {
    static ref MAP_ERR: RwLock<Option<fn(err: String)-> io::Error>> = Default::default();
}

pub fn set_map_err(f: fn(err: String) -> io::Error) {
    *MAP_ERR.write().unwrap() = Some(f);
}

fn map_err<E: ToString>(err: E) -> io::Error {
    if let Some(f) = *MAP_ERR.read().unwrap() {
        f(err.to_string())
    } else {
        io::Error::new(io::ErrorKind::Other, err.to_string())
    }
}

impl Capturer {
    pub fn new(display: Display) -> io::Result<Capturer> {
        let r = display.0.recorder(false).map_err(map_err)?;
        Ok(Capturer(display, r, Default::default()))
    }

    pub fn width(&self) -> usize {
        self.0.width()
    }

    pub fn height(&self) -> usize {
        self.0.height()
    }
}

impl TraitCapturer for Capturer {
    fn frame<'a>(&'a mut self, timeout: Duration) -> io::Result<Frame<'a>> {
        match self.1.capture(timeout.as_millis() as _).map_err(map_err)? {
            PixelProvider::BGR0(w, h, x) => Ok(Frame::PixelBuffer(PixelBuffer::new(
                x,
                crate::Pixfmt::BGRA,
                w,
                h,
            ))),
            PixelProvider::RGB0(w, h, x) => Ok(Frame::PixelBuffer(PixelBuffer::new(
                x,
                crate::Pixfmt::RGBA,
                w,
                h,
            ))),
            PixelProvider::NONE => Err(std::io::ErrorKind::WouldBlock.into()),
            _ => Err(map_err("Invalid data")),
        }
    }
}

pub struct Display(pub(crate) Box<dyn Capturable>);

impl Display {
    pub fn primary() -> io::Result<Display> {
        let mut all = Display::all()?;
        if all.is_empty() {
            return Err(io::ErrorKind::NotFound.into());
        }
        Ok(all.remove(0))
    }

    /// Check if the system is in lock screen or pre-login state
    fn is_lock_screen_or_pre_login() -> bool {
        // Check for common lock screen processes
        let lock_screen_processes = [
            "gdm-password",    // GNOME lock screen
            "sddm-greeter",   // KDE lock screen
            "lightdm-greeter", // LightDM greeter
            "plymouth",       // Boot splash screen
            "gdm-wayland-session", // GNOME Wayland session (lock screen)
        ];
        
        // Check if any lock screen process is running
        if let Ok(output) = std::process::Command::new("ps")
            .arg("aux")
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for process in lock_screen_processes {
                if output_str.contains(process) {
                    eprintln!("Detected lock screen process: {}", process);
                    return true;
                }
            }
        }
        
        // Check for special login-related environment variables
        if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland" {
            // In some cases, pre-login environment might not have user-specific variables
            if std::env::var("USER").unwrap_or_default() == "root" {
                eprintln!("Detected root user in Wayland session (likely pre-login)");
                return true;
            }
            
            // Check if we're in a login manager context
            if let Ok(display) = std::env::var("DISPLAY") {
                if display.contains(":0") && std::env::var("HOME").unwrap_or_default() == "/root" {
                    eprintln!("Detected root user with display :0 (likely login screen)");
                    return true;
                }
            }
        }
        
        false
    }
    
    pub fn all() -> io::Result<Vec<Display>> {
        // If we're in lock screen or pre-login state, use DRM directly
        if Self::is_lock_screen_or_pre_login() {
            eprintln!("Detected lock screen or pre-login state, using DRM directly");
            
            // First try all available DRM cards
            for i in 0..8 { // Try up to 8 DRM cards
                let drm_path = format!("/dev/dri/card{}", i);
                match crate::wayland::drm::DrmCapturable::new(&drm_path) {
                    Ok(drm_capturable) => {
                        eprintln!("Using DRM device for lock screen: {}", drm_path);
                        return Ok(vec![Display(Box::new(drm_capturable))]);
                    }
                    Err(e) => {
                        eprintln!("DRM device {} failed: {}", drm_path, e);
                        // Continue to next device
                    }
                }
            }
            
            // Try framebuffer devices as last resort
            for i in 0..4 { // Try up to 4 framebuffer devices
                let fb_path = format!("/dev/fb{}", i);
                match crate::wayland::drm::DrmCapturable::new(&fb_path) {
                    Ok(fb_capturable) => {
                        eprintln!("Using framebuffer device for lock screen: {}", fb_path);
                        return Ok(vec![Display(Box::new(fb_capturable))]);
                    }
                    Err(e) => {
                        eprintln!("Framebuffer device {} failed: {}", fb_path, e);
                        // Continue to next device
                    }
                }
            }
            
            return Err(map_err("All DRM and framebuffer devices failed in lock screen state".to_string()));
        }
        
        // Normal case: Try PipeWire first
        match pipewire::get_capturables() {
            Ok(capturables) => {
                if !capturables.is_empty() {
                    return Ok(capturables
                        .drain(..)
                        .map(|x| Display(Box::new(x)))
                        .collect());
                }
            }
            Err(e) => {
                // If PipeWire fails, try DRM
                eprintln!("PipeWire failed, trying DRM: {}", e);
            }
        }
        
        // Try DRM as fallback
        // First try all available DRM cards
        for i in 0..8 { // Try up to 8 DRM cards
            let drm_path = format!("/dev/dri/card{}", i);
            match crate::wayland::drm::DrmCapturable::new(&drm_path) {
                Ok(drm_capturable) => {
                    eprintln!("Using DRM device: {}", drm_path);
                    return Ok(vec![Display(Box::new(drm_capturable))]);
                }
                Err(e) => {
                    eprintln!("DRM device {} failed: {}", drm_path, e);
                    // Continue to next device
                }
            }
        }
        
        // Try framebuffer devices as last resort
        for i in 0..4 { // Try up to 4 framebuffer devices
            let fb_path = format!("/dev/fb{}", i);
            match crate::wayland::drm::DrmCapturable::new(&fb_path) {
                Ok(fb_capturable) => {
                    eprintln!("Using framebuffer device: {}", fb_path);
                    return Ok(vec![Display(Box::new(fb_capturable))]);
                }
                Err(e) => {
                    eprintln!("Framebuffer device {} failed: {}", fb_path, e);
                    // Continue to next device
                }
            }
        }
        
        // All capture methods failed
        Err(map_err("All capture methods failed: No working DRM or framebuffer device found".to_string()))
    }

    pub fn width(&self) -> usize {
        // This is a placeholder, in a real implementation we would need to get the width
        // from the underlying capturable
        1920
    }

    pub fn height(&self) -> usize {
        // This is a placeholder, in a real implementation we would need to get the height
        // from the underlying capturable
        1080
    }

    pub fn physical_width(&self) -> usize {
        self.width()
    }

    pub fn physical_height(&self) -> usize {
        self.height()
    }

    pub fn logical_width(&self) -> usize {
        self.width()
    }

    pub fn logical_height(&self) -> usize {
        self.height()
    }

    pub fn scale(&self) -> f64 {
        1.0
    }

    pub fn origin(&self) -> (i32, i32) {
        (0, 0)
    }

    pub fn is_online(&self) -> bool {
        true
    }

    pub fn is_primary(&self) -> bool {
        true
    }

    pub fn name(&self) -> String {
        self.0.name()
    }
}