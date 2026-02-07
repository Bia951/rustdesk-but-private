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

    pub fn all() -> io::Result<Vec<Display>> {
        // Try PipeWire first
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
        match drm::DrmCapturable::new("/dev/dri/card0") {
            Ok(drm_capturable) => {
                Ok(vec![Display(Box::new(drm_capturable))])
            }
            Err(e) => {
                // Try framebuffer as last resort
                eprintln!("DRM failed, trying framebuffer: {}", e);
                match drm::DrmCapturable::new("/dev/fb0") {
                    Ok(fb_capturable) => {
                        Ok(vec![Display(Box::new(fb_capturable))])
                    }
                    Err(e) => {
                        Err(map_err(format!("All capture methods failed: {}", e)))
                    }
                }
            }
        }
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