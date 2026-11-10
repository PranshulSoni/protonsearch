//! Opens allow-listed Windows Settings pages (typed enum lives in crate root).

use anyhow::Result;

use crate::SettingsPage;

pub(crate) fn open(page: SettingsPage) -> Result<()> {
    super::shell::shell_execute("open", page.uri(), None)
}
