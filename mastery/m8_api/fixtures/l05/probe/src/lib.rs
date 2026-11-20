//! Uses three things that `swatch` offers only behind a feature. With no
//! feature turned on, every one of the three functions must be an error.
//! You do not need to edit this file.

pub fn needs_names() -> Option<swatch::Color> {
    swatch::Color::named("red")
}

pub fn needs_json() -> String {
    swatch::Color::new(1, 2, 3).to_json()
}

pub fn needs_wide() -> String {
    swatch::Color16::new(1, 2, 3).hex()
}
