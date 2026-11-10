//! The user's Windows notification feed (what the notification centre lists),
//! read live through the notification listener. Nothing is stored: an entry
//! exists here only for as long as Windows itself keeps it.
//!
//! Windows grants desktop apps this access as a group ("Let desktop apps access
//! your notifications" under Settings > Privacy & security > Notifications);
//! when that is off every call reports it instead of returning an empty list.

use anyhow::{bail, Context, Result};
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus as Access,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};

use crate::SystemNotification;

fn listener() -> Result<UserNotificationListener> {
    let listener = UserNotificationListener::Current().context("opening the notification listener")?;
    let mut access = listener.GetAccessStatus().context("checking notification access")?;
    if access == Access::Unspecified {
        access = listener
            .RequestAccessAsync()
            .context("requesting notification access")?
            .get()
            .context("waiting for notification access")?;
    }
    if access != Access::Allowed {
        bail!("Windows is not letting desktop apps read notifications; allow it under Settings > Privacy & security > Notifications");
    }
    Ok(listener)
}

fn describe(item: &UserNotification) -> Option<SystemNotification> {
    let id = item.Id().ok()?;
    let created = item.CreationTime().map(|time| time.UniversalTime).unwrap_or(0);
    let app = item
        .AppInfo()
        .and_then(|info| info.DisplayInfo())
        .and_then(|display| display.DisplayName())
        .map(|name| name.to_string())
        .unwrap_or_default();
    // Toast text lines in order: the first is the title, the rest the body.
    let mut lines = Vec::new();
    if let Ok(elements) = item
        .Notification()
        .and_then(|notification| notification.Visual())
        .and_then(|visual| visual.GetBinding(&KnownNotificationBindings::ToastGeneric()?))
        .and_then(|binding| binding.GetTextElements())
    {
        for element in &elements {
            let line = element.Text().map(|text| text.to_string()).unwrap_or_default();
            if !line.trim().is_empty() {
                lines.push(line.trim().to_string());
            }
        }
    }
    if lines.is_empty() {
        return None;
    }
    if lines.len() > 1 && lines[0].eq_ignore_ascii_case(&app) {
        lines.remove(0);
    }
    Some(SystemNotification {
        id,
        app,
        title: lines.remove(0),
        body: lines.join(" "),
        created_ms: (created - super::media::UNIX_EPOCH_TICKS) / 10_000,
    })
}

/// Newest first, at most `limit` entries.
pub fn list(limit: usize) -> Result<Vec<SystemNotification>> {
    super::winrt::run(move || {
        let items = listener()?
            .GetNotificationsAsync(NotificationKinds::Toast)
            .context("requesting notifications")?
            .get()
            .context("reading notifications")?;
        let mut entries: Vec<SystemNotification> = (&items).into_iter().filter_map(|item| describe(&item)).collect();
        entries.sort_by(|a, b| b.created_ms.cmp(&a.created_ms));
        entries.truncate(limit);
        Ok(entries)
    })
}

pub fn dismiss(id: u32) -> Result<()> {
    super::winrt::run(move || listener()?.RemoveNotification(id).context("removing the notification"))
}

pub fn clear() -> Result<()> {
    super::winrt::run(|| listener()?.ClearNotifications().context("clearing notifications"))
}
