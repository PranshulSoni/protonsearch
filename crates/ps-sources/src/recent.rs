//! Recently used files (Windows Recent folder + our own launch history).
//! OWNER: sources-content.

use super::RecentEntry;

pub fn scan(_our_db: Option<&std::path::Path>) -> (Vec<RecentEntry>, Vec<String>) {
    (Vec::new(), Vec::new())
}
