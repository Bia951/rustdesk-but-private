use enigo::{Key, KeyboardControllable, MouseButton, MouseControllable};
use hbb_common::{bail, log, ResultType};
use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AttributeSet, EventType, InputEvent, Key as EvdevKey,
};
use std::fs::File;
use std::os::unix::io::{AsRawFd, RawFd};
use std::thread;
use std::time::Duration;

pub struct EvdevInputKeyboard {
    device: VirtualDevice,
}

impl EvdevInputKeyboard {
    pub fn new() -> ResultType<Self> {
        // Create a virtual keyboard device
        let device = VirtualDeviceBuilder::new()?
            .name("RustDesk Virtual Keyboard")
            .with_keys(&AttributeSet::from_iter([
                EvdevKey::KEY_A, EvdevKey::KEY_B, EvdevKey::KEY_C, EvdevKey::KEY_D, EvdevKey::KEY_E,
                EvdevKey::KEY_F, EvdevKey::KEY_G, EvdevKey::KEY_H, EvdevKey::KEY_I, EvdevKey::KEY_J,
                EvdevKey::KEY_K, EvdevKey::KEY_L, EvdevKey::KEY_M, EvdevKey::KEY_N, EvdevKey::KEY_O,
                EvdevKey::KEY_P, EvdevKey::KEY_Q, EvdevKey::KEY_R, EvdevKey::KEY_S, EvdevKey::KEY_T,
                EvdevKey::KEY_U, EvdevKey::KEY_V, EvdevKey::KEY_W, EvdevKey::KEY_X, EvdevKey::KEY_Y,
                EvdevKey::KEY_Z,
                EvdevKey::KEY_1, EvdevKey::KEY_2, EvdevKey::KEY_3, EvdevKey::KEY_4, EvdevKey::KEY_5,
                EvdevKey::KEY_6, EvdevKey::KEY_7, EvdevKey::KEY_8, EvdevKey::KEY_9, EvdevKey::KEY_0,
                EvdevKey::KEY_SPACE, EvdevKey::KEY_ENTER, EvdevKey::KEY_BACKSPACE,
                EvdevKey::KEY_LEFTSHIFT, EvdevKey::KEY_RIGHTSHIFT,
                EvdevKey::KEY_LEFTCTRL, EvdevKey::KEY_RIGHTCTRL,
                EvdevKey::KEY_LEFTALT, EvdevKey::KEY_RIGHTALT,
                EvdevKey::KEY_LEFTMETA, EvdevKey::KEY_RIGHTMETA,
                EvdevKey::KEY_TAB, EvdevKey::KEY_CAPSLOCK,
                EvdevKey::KEY_F1, EvdevKey::KEY_F2, EvdevKey::KEY_F3, EvdevKey::KEY_F4, EvdevKey::KEY_F5,
                EvdevKey::KEY_F6, EvdevKey::KEY_F7, EvdevKey::KEY_F8, EvdevKey::KEY_F9, EvdevKey::KEY_F10,
                EvdevKey::KEY_F11, EvdevKey::KEY_F12,
                EvdevKey::KEY_ESC, EvdevKey::KEY_SYSRQ,
                EvdevKey::KEY_SCROLLLOCK, EvdevKey::KEY_PAUSE,
                EvdevKey::KEY_INSERT, EvdevKey::KEY_HOME, EvdevKey::KEY_PAGEUP,
                EvdevKey::KEY_DELETE, EvdevKey::KEY_END, EvdevKey::KEY_PAGEDOWN,
                EvdevKey::KEY_RIGHT, EvdevKey::KEY_LEFT, EvdevKey::KEY_DOWN, EvdevKey::KEY_UP,
            ]))?
            .build()?;

        Ok(Self {
            device,
        })
    }

    fn map_enigo_key_to_evdev(&self, key: Key) -> Option<EvdevKey> {
        // Simple mapping from enigo key to evdev key
        match key {
            Key::Layout('a') => Some(EvdevKey::KEY_A),
            Key::Layout('b') => Some(EvdevKey::KEY_B),
            Key::Layout('c') => Some(EvdevKey::KEY_C),
            Key::Layout('d') => Some(EvdevKey::KEY_D),
            Key::Layout('e') => Some(EvdevKey::KEY_E),
            Key::Layout('f') => Some(EvdevKey::KEY_F),
            Key::Layout('g') => Some(EvdevKey::KEY_G),
            Key::Layout('h') => Some(EvdevKey::KEY_H),
            Key::Layout('i') => Some(EvdevKey::KEY_I),
            Key::Layout('j') => Some(EvdevKey::KEY_J),
            Key::Layout('k') => Some(EvdevKey::KEY_K),
            Key::Layout('l') => Some(EvdevKey::KEY_L),
            Key::Layout('m') => Some(EvdevKey::KEY_M),
            Key::Layout('n') => Some(EvdevKey::KEY_N),
            Key::Layout('o') => Some(EvdevKey::KEY_O),
            Key::Layout('p') => Some(EvdevKey::KEY_P),
            Key::Layout('q') => Some(EvdevKey::KEY_Q),
            Key::Layout('r') => Some(EvdevKey::KEY_R),
            Key::Layout('s') => Some(EvdevKey::KEY_S),
            Key::Layout('t') => Some(EvdevKey::KEY_T),
            Key::Layout('u') => Some(EvdevKey::KEY_U),
            Key::Layout('v') => Some(EvdevKey::KEY_V),
            Key::Layout('w') => Some(EvdevKey::KEY_W),
            Key::Layout('x') => Some(EvdevKey::KEY_X),
            Key::Layout('y') => Some(EvdevKey::KEY_Y),
            Key::Layout('z') => Some(EvdevKey::KEY_Z),
            Key::Layout('0') => Some(EvdevKey::KEY_0),
            Key::Layout('1') => Some(EvdevKey::KEY_1),
            Key::Layout('2') => Some(EvdevKey::KEY_2),
            Key::Layout('3') => Some(EvdevKey::KEY_3),
            Key::Layout('4') => Some(EvdevKey::KEY_4),
            Key::Layout('5') => Some(EvdevKey::KEY_5),
            Key::Layout('6') => Some(EvdevKey::KEY_6),
            Key::Layout('7') => Some(EvdevKey::KEY_7),
            Key::Layout('8') => Some(EvdevKey::KEY_8),
            Key::Layout('9') => Some(EvdevKey::KEY_9),
            Key::Space => Some(EvdevKey::KEY_SPACE),
            Key::Return => Some(EvdevKey::KEY_ENTER),
            Key::Backspace => Some(EvdevKey::KEY_BACKSPACE),
            Key::Tab => Some(EvdevKey::KEY_TAB),
            Key::Escape => Some(EvdevKey::KEY_ESC),
            Key::LeftShift => Some(EvdevKey::KEY_LEFTSHIFT),
            Key::RightShift => Some(EvdevKey::KEY_RIGHTSHIFT),
            Key::LeftControl => Some(EvdevKey::KEY_LEFTCTRL),
            Key::RightControl => Some(EvdevKey::KEY_RIGHTCTRL),
            Key::LeftAlt => Some(EvdevKey::KEY_LEFTALT),
            Key::RightAlt => Some(EvdevKey::KEY_RIGHTALT),
            Key::LeftMeta => Some(EvdevKey::KEY_LEFTMETA),
            Key::RightMeta => Some(EvdevKey::KEY_RIGHTMETA),
            Key::CapsLock => Some(EvdevKey::KEY_CAPSLOCK),
            Key::F1 => Some(EvdevKey::KEY_F1),
            Key::F2 => Some(EvdevKey::KEY_F2),
            Key::F3 => Some(EvdevKey::KEY_F3),
            Key::F4 => Some(EvdevKey::KEY_F4),
            Key::F5 => Some(EvdevKey::KEY_F5),
            Key::F6 => Some(EvdevKey::KEY_F6),
            Key::F7 => Some(EvdevKey::KEY_F7),
            Key::F8 => Some(EvdevKey::KEY_F8),
            Key::F9 => Some(EvdevKey::KEY_F9),
            Key::F10 => Some(EvdevKey::KEY_F10),
            Key::F11 => Some(EvdevKey::KEY_F11),
            Key::F12 => Some(EvdevKey::KEY_F12),
            Key::Delete => Some(EvdevKey::KEY_DELETE),
            Key::Home => Some(EvdevKey::KEY_HOME),
            Key::End => Some(EvdevKey::KEY_END),
            Key::PageUp => Some(EvdevKey::KEY_PAGEUP),
            Key::PageDown => Some(EvdevKey::KEY_PAGEDOWN),
            Key::UpArrow => Some(EvdevKey::KEY_UP),
            Key::DownArrow => Some(EvdevKey::KEY_DOWN),
            Key::LeftArrow => Some(EvdevKey::KEY_LEFT),
            Key::RightArrow => Some(EvdevKey::KEY_RIGHT),
            _ => {
                log::debug!("Unmapped key: {:?}", key);
                None
            },
        }
    }
}

impl KeyboardControllable for EvdevInputKeyboard {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_key_state(&mut self, _: Key) -> bool {
        // Virtual device doesn't track key state
        false
    }

    fn key_sequence(&mut self, s: &str) {
        for c in s.chars() {
            let key = Key::Layout(c);
            let _ = self.key_down(key);
            thread::sleep(Duration::from_millis(10));
            let _ = self.key_up(key);
        }
    }

    fn key_down(&mut self, key: Key) -> enigo::ResultType {
        if let Some(evdev_key) = self.map_enigo_key_to_evdev(key) {
            self.device.emit(&[InputEvent::new(EventType::KEY, evdev_key.code(), 1)])?;
            self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)])?;
        }
        Ok(())
    }

    fn key_up(&mut self, key: Key) {
        if let Some(evdev_key) = self.map_enigo_key_to_evdev(key) {
            let _ = self.device.emit(&[InputEvent::new(EventType::KEY, evdev_key.code(), 0)]);
            let _ = self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)]);
        }
    }

    fn key_click(&mut self, key: Key) {
        let _ = self.key_down(key);
        thread::sleep(Duration::from_millis(10));
        let _ = self.key_up(key);
    }
}

pub struct EvdevInputMouse {
    device: VirtualDevice,
    width: usize,
    height: usize,
    current_x: i32,
    current_y: i32,
}

impl EvdevInputMouse {
    pub fn new(width: usize, height: usize) -> ResultType<Self> {
        // Create a virtual mouse device
        let device = VirtualDeviceBuilder::new()?
            .name("RustDesk Virtual Mouse")
            .with_rel_events(&AttributeSet::from_iter([
                evdev::RelativeAxisType::REL_X,
                evdev::RelativeAxisType::REL_Y,
                evdev::RelativeAxisType::REL_WHEEL,
                evdev::RelativeAxisType::REL_HWHEEL,
            ]))?
            .with_keys(&AttributeSet::from_iter([
                EvdevKey::BTN_LEFT,
                EvdevKey::BTN_RIGHT,
                EvdevKey::BTN_MIDDLE,
                EvdevKey::BTN_SIDE,
                EvdevKey::BTN_EXTRA,
            ]))?
            .build()?;

        Ok(Self {
            device,
            width,
            height,
            current_x: width as i32 / 2,
            current_y: height as i32 / 2,
        })
    }

    fn map_enigo_button_to_evdev(&self, button: MouseButton) -> Option<EvdevKey> {
        match button {
            MouseButton::Left => Some(EvdevKey::BTN_LEFT),
            MouseButton::Right => Some(EvdevKey::BTN_RIGHT),
            MouseButton::Middle => Some(EvdevKey::BTN_MIDDLE),
            MouseButton::Back => Some(EvdevKey::BTN_SIDE),
            MouseButton::Forward => Some(EvdevKey::BTN_EXTRA),
            _ => None,
        }
    }
}

impl MouseControllable for EvdevInputMouse {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn mouse_move_to(&mut self, x: i32, y: i32) {
        // For virtual device, we use relative movement
        let dx = x - self.current_x;
        let dy = y - self.current_y;
        
        if dx != 0 || dy != 0 {
            let _ = self.device.emit(&[
                InputEvent::new(EventType::RELATIVE, evdev::RelativeAxisType::REL_X.code(), dx as i32),
                InputEvent::new(EventType::RELATIVE, evdev::RelativeAxisType::REL_Y.code(), dy as i32),
            ]);
            let _ = self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)]);
            
            self.current_x = x;
            self.current_y = y;
        }
    }

    fn mouse_move_relative(&mut self, dx: i32, dy: i32) {
        if dx != 0 || dy != 0 {
            let _ = self.device.emit(&[
                InputEvent::new(EventType::RELATIVE, evdev::RelativeAxisType::REL_X.code(), dx),
                InputEvent::new(EventType::RELATIVE, evdev::RelativeAxisType::REL_Y.code(), dy),
            ]);
            let _ = self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)]);
            
            self.current_x = (self.current_x + dx).clamp(0, self.width as i32);
            self.current_y = (self.current_y + dy).clamp(0, self.height as i32);
        }
    }

    fn mouse_down(&mut self, button: MouseButton) {
        if let Some(evdev_key) = self.map_enigo_button_to_evdev(button) {
            let _ = self.device.emit(&[InputEvent::new(EventType::KEY, evdev_key.code(), 1)]);
            let _ = self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)]);
        }
    }

    fn mouse_up(&mut self, button: MouseButton) {
        if let Some(evdev_key) = self.map_enigo_button_to_evdev(button) {
            let _ = self.device.emit(&[InputEvent::new(EventType::KEY, evdev_key.code(), 0)]);
            let _ = self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)]);
        }
    }

    fn mouse_click(&mut self, button: MouseButton) {
        self.mouse_down(button);
        thread::sleep(Duration::from_millis(10));
        self.mouse_up(button);
    }

    fn mouse_scroll_x(&mut self, delta: i32) {
        let _ = self.device.emit(&[
            InputEvent::new(EventType::RELATIVE, evdev::RelativeAxisType::REL_HWHEEL.code(), delta),
        ]);
        let _ = self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)]);
    }

    fn mouse_scroll_y(&mut self, delta: i32) {
        let _ = self.device.emit(&[
            InputEvent::new(EventType::RELATIVE, evdev::RelativeAxisType::REL_WHEEL.code(), delta),
        ]);
        let _ = self.device.emit(&[InputEvent::new(EventType::SYN, 0, 0)]);
    }
}