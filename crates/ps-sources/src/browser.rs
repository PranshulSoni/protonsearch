//! Browser history + bookmarks (Chromium: Chrome/Edge/Brave, Gecko: Firefox).
//! OWNER: sources-content. DBs are copied before reading; secrets are never read.

use super::BrowserEntry;

pub fn scan() -> (Vec<BrowserEntry>, Vec<String>) {
    (Vec::new(), Vec::new())
}
