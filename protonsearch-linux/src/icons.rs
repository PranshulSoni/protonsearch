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
    let pixbuf = Pixbuf::from_read(Cursor::new(bytes)).ok()?;
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
    SOURCE_ICONS
        .iter()
        .find(|(key, _)| *key == name)
        .and_then(|(_, bytes)| image_from_bytes(bytes, size))
}
