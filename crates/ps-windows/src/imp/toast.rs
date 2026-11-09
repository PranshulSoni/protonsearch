//! Local Windows toast notifications for the app's own notifications.
//! Other applications' Windows notifications are NOT accessible to normal
//! desktop apps; the notification store is the source of truth and this module
//! only surfaces ProtonSearch-originated toasts where Windows allows it.

use anyhow::{Context, Result};
use windows::core::HSTRING;

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// True when Windows accepts a toast notifier for this app identity.
pub fn supported() -> bool {
    let owned = super::winrt::ensure_apartment();
    let app_id = HSTRING::from("ProtonSearch");
    let result = windows::UI::Notifications::ToastNotificationManager::CreateToastNotifierWithId(&app_id).is_ok();
    super::winrt::release_apartment(owned);
    result
}

/// Show a local Toast notification (title + body). Returns a structured error
/// when this Windows identity/build cannot show app toasts (e.g. unpackaged app).
pub fn show(title: &str, body: &str) -> Result<()> {
    let owned = super::winrt::ensure_apartment();
    let result = (|| -> Result<()> {
        let template = format!(
            "<toast><visual><binding template=\"ToastText02\"><text id=\"1\">{}</text><text id=\"2\">{}</text></binding></visual></toast>",
            xml_escape(title),
            xml_escape(body)
        );
        let document = windows::Data::Xml::Dom::XmlDocument::new().context("XmlDocument::new")?;
        document
            .LoadXml(&HSTRING::from(template))
            .context("loading toast XML")?;
        let notification = windows::UI::Notifications::ToastNotification::CreateToastNotification(&document)
            .context("CreateToastNotification")?;
        let app_id = HSTRING::from("ProtonSearch");
        let notifier = windows::UI::Notifications::ToastNotificationManager::CreateToastNotifierWithId(&app_id)
            .context("CreateToastNotifierWithId(ProtonSearch)")?;
        notifier.Show(&notification).context("ToastNotifier::Show")
    })();
    super::winrt::release_apartment(owned);
    result
}
