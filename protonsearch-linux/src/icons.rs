//! Embedded ProtonSearch artwork and launcher source icons.
//!
//! Keeping these assets in the binary makes the launcher independent of the
//! Windows checkout path and avoids falling back to whichever theme symbols a
//! Linux desktop happens to provide.

use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::WidgetExt;
use gtk4::{gdk, Image};
use std::io::Cursor;

const PROTONSEARCH: &[u8] = include_bytes!("../assets/branding/protonsearch.png");

const SOURCE_ICONS: &[(&str, &[u8])] = &[
    (
        "agent-history",
        include_bytes!("../assets/sources/agent-history.png"),
    ),
    ("agents", include_bytes!("../assets/sources/agents.png")),
    ("all", include_bytes!("../assets/sources/all.png")),
    (
        "browser-bookmarks",
        include_bytes!("../assets/sources/browser-bookmarks.png"),
    ),
    (
        "browser-history",
        include_bytes!("../assets/sources/browser-history.png"),
    ),
    (
        "clipboard-history",
        include_bytes!("../assets/sources/clipboard-history.png"),
    ),
    ("commands", include_bytes!("../assets/sources/commands.png")),
    ("content", include_bytes!("../assets/sources/content.png")),
    ("files", include_bytes!("../assets/sources/files.png")),
    (
        "git-commits",
        include_bytes!("../assets/sources/git-commits.png"),
    ),
    ("images", include_bytes!("../assets/sources/images.png")),
    (
        "local-files",
        include_bytes!("../assets/sources/local-files.png"),
    ),
    ("settings", include_bytes!("../assets/sources/settings.png")),
    (
        "source-code",
        include_bytes!("../assets/sources/source-code.png"),
    ),
];

fn image_from_bytes(bytes: &'static [u8], size: i32) -> Option<Image> {
    image_from_bytes_with_tint(bytes, size, None)
}

fn image_from_bytes_with_tint(
    bytes: &'static [u8],
    size: i32,
    tint: Option<(u8, u8, u8)>,
) -> Option<Image> {
    let pixbuf = Pixbuf::from_read(Cursor::new(bytes)).ok()?;
    if let Some((red, green, blue)) = tint {
        let channels = pixbuf.n_channels() as usize;
        let rowstride = pixbuf.rowstride() as usize;
        let width = pixbuf.width() as usize;
        let height = pixbuf.height() as usize;
        // Source artwork is intentionally white for the dark launcher. Tint
        // it at runtime for light mode while preserving anti-aliased alpha.
        unsafe {
            let pixels = pixbuf.pixels();
            for y in 0..height {
                for x in 0..width {
                    let offset = y * rowstride + x * channels;
                    pixels[offset] = red;
                    pixels[offset + 1] = green;
                    pixels[offset + 2] = blue;
                }
            }
        }
    }
    let texture = gdk::Texture::for_pixbuf(&pixbuf);
    let image = Image::from_paintable(Some(&texture));
    image.set_pixel_size(size);
    image.add_css_class("asset-icon");
    Some(image)
}

pub fn protonsearch(size: i32) -> Image {
    image_from_bytes(PROTONSEARCH, size).expect("embedded ProtonSearch logo is valid")
}

pub fn source(name: &str, size: i32) -> Option<Image> {
    source_for_theme(name, size, false)
}

pub fn source_for_theme(name: &str, size: i32, light_theme: bool) -> Option<Image> {
    SOURCE_ICONS
        .iter()
        .find(|(key, _)| *key == name)
        .and_then(|(_, bytes)| {
            let tint = light_theme.then_some((73, 82, 92));
            image_from_bytes_with_tint(bytes, size, tint)
        })
}
