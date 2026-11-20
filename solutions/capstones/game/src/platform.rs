//! The platform layer: the only file that names the window crate.
//!
//! It does three things and nothing else: open a window, show a
//! framebuffer in it, and say which keys are down. Swapping `minifb` for
//! another crate, or for a GPU surface, means rewriting this file only.

use game::{Framebuffer, Input};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};

/// Keys that act once per press, outside the simulation.
#[derive(Clone, Copy)]
pub(crate) enum Command {
    Restart,
    Screenshot,
}

pub(crate) struct Platform {
    window: Window,
}

impl Platform {
    pub(crate) fn open(
        title: &str,
        width: usize,
        height: usize,
        scale: u32,
    ) -> Result<Platform, String> {
        let scale = match scale {
            1 => Scale::X1,
            2 => Scale::X2,
            4 => Scale::X4,
            _ => return Err("--scale must be 1, 2 or 4".to_owned()),
        };
        let options = WindowOptions {
            scale,
            ..WindowOptions::default()
        };
        let mut window = Window::new(title, width, height, options)
            .map_err(|e| format!("cannot open a window: {e}"))?;
        // Let the window crate sleep between frames; without this the loop
        // would redraw as fast as the processor allows.
        window.set_target_fps(60);
        Ok(Platform { window })
    }

    /// False once the user has closed the window or pressed escape.
    pub(crate) fn is_open(&self) -> bool {
        self.window.is_open() && !self.window.is_key_down(Key::Escape)
    }

    /// The buttons held right now.
    pub(crate) fn input(&self) -> Input {
        let down = |keys: &[Key]| keys.iter().any(|&key| self.window.is_key_down(key));
        let mut mask = 0;
        if down(&[Key::Left, Key::A]) {
            mask |= Input::LEFT;
        }
        if down(&[Key::Right, Key::D]) {
            mask |= Input::RIGHT;
        }
        if down(&[Key::Space, Key::Up, Key::W, Key::Z]) {
            mask |= Input::JUMP;
        }
        Input(mask)
    }

    /// True in the frame in which the key for `command` went down.
    pub(crate) fn pressed(&self, command: Command) -> bool {
        let key = match command {
            Command::Restart => Key::R,
            Command::Screenshot => Key::F2,
        };
        self.window.is_key_pressed(key, KeyRepeat::No)
    }

    /// Shows the framebuffer and lets the window process its events.
    ///
    /// The game's pixels are `0xAARRGGBB`, which is the layout `minifb`
    /// expects (it ignores the top byte), so the buffer is passed as it is.
    pub(crate) fn present(&mut self, fb: &Framebuffer) -> Result<(), String> {
        self.window
            .update_with_buffer(fb.pixels(), fb.width(), fb.height())
            .map_err(|e| format!("cannot update the window: {e}"))
    }
}
