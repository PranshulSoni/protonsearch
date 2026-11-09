//! Installed-app discovery (Start Menu shortcuts + packaged/UWP apps).
//! OWNER: sources-content.

use super::AppEntry;

/// Enumerate installed applications. Bounded, no elevation.
pub fn scan() -> (Vec<AppEntry>, Vec<String>) {
    (Vec::new(), Vec::new())
}
