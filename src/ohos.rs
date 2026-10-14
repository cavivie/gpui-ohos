mod dispatcher;
mod display;
mod keyboard;
mod platform;
mod text_system;
mod wgpu_atlas;
mod wgpu_context;
mod wgpu_renderer;
mod window;

use std::sync::atomic::{AtomicU8, Ordering};

use openharmony_ability::ime::InputType;

use openharmony_ability::OpenHarmonyApp;

pub fn current_platform(app: OpenHarmonyApp, _headless: bool) -> std::rc::Rc<dyn gpui::Platform> {
    std::rc::Rc::new(
        platform::OhosPlatform::new(app)
            .inspect_err(|err| {
                log::error!("Failed to initialize OHOS platform: {}", err);
            })
            .unwrap_or_else(|_| panic!("Failed to initialize OHOS platform")),
    )
}

/// The kind of text the focused field takes, which decides the keyboard the
/// system input method offers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardType {
    #[default]
    Text,
    Phone,
    Number,
}

static WANTED_KEYBOARD: AtomicU8 = AtomicU8::new(KeyboardType::Text as u8);

/// Sets the kind of keyboard asked for the next time a field gains focus.
/// Applications call it as a touch lands, before the focus it causes.
pub fn set_keyboard_type(kind: KeyboardType) {
    WANTED_KEYBOARD.store(kind as u8, Ordering::Relaxed);
}

fn wanted_input_type() -> InputType {
    match WANTED_KEYBOARD.load(Ordering::Relaxed) {
        wanted if wanted == KeyboardType::Phone as u8 => InputType::Phone,
        wanted if wanted == KeyboardType::Number as u8 => InputType::Number,
        _ => InputType::Text,
    }
}
