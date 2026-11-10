//! Now-playing media through the system media transport controls: the same
//! session Windows shows in its own volume flyout. User-level; no admin and no
//! package identity needed.

use anyhow::{bail, Context, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

use crate::{MediaCommand, MediaSession};

/// 100 ns ticks between the Windows epoch (1601) and the Unix epoch (1970).
pub(crate) const UNIX_EPOCH_TICKS: i64 = 116_444_736_000_000_000;

/// The session Windows considers current; `None` when nothing has registered one.
fn current_session() -> Result<Option<Session>> {
    let manager = Manager::RequestAsync()
        .context("requesting the media session manager")?
        .get()
        .context("waiting for the media session manager")?;
    // A null session comes back as an error from the bindings.
    Ok(manager.GetCurrentSession().ok())
}

fn ticks_to_ms(ticks: i64) -> Option<u64> {
    u64::try_from(ticks / 10_000).ok()
}

pub fn now_playing() -> Result<Option<MediaSession>> {
    super::winrt::run(|| {
        let Some(session) = current_session()? else {
            return Ok(None);
        };
        let properties = session
            .TryGetMediaPropertiesAsync()
            .context("requesting media properties")?
            .get()
            .context("reading media properties")?;
        let playback = session.GetPlaybackInfo().context("reading playback state")?;
        let playing = playback.PlaybackStatus().ok() == Some(PlaybackStatus::Playing);
        let text = |value: windows::core::Result<windows::core::HSTRING>| {
            value.map(|value| value.to_string()).unwrap_or_default()
        };
        let album = text(properties.AlbumTitle());

        // Players publish their position only now and then; while playing, the
        // real position is that snapshot plus the time since it was taken.
        let (mut position_ms, mut duration_ms) = (None, None);
        if let Ok(timeline) = session.GetTimelineProperties() {
            let start = timeline.StartTime().map(|t| t.Duration).unwrap_or(0);
            let end = timeline.EndTime().map(|t| t.Duration).unwrap_or(0);
            if end > start {
                let length = end - start;
                let mut position = timeline.Position().map(|t| t.Duration).unwrap_or(start) - start;
                let updated = timeline.LastUpdatedTime().map(|t| t.UniversalTime).unwrap_or(0);
                if playing && updated > UNIX_EPOCH_TICKS {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|elapsed| (elapsed.as_nanos() / 100) as i64 + UNIX_EPOCH_TICKS)
                        .unwrap_or(updated);
                    position += (now - updated).max(0);
                }
                position_ms = ticks_to_ms(position.clamp(0, length));
                duration_ms = ticks_to_ms(length);
            }
        }

        Ok(Some(MediaSession {
            title: text(properties.Title()),
            artist: text(properties.Artist()),
            album: (!album.is_empty()).then_some(album),
            app_id: session.SourceAppUserModelId().ok().map(|id| id.to_string()),
            playing,
            position_ms,
            duration_ms,
        }))
    })
}

pub fn command(command: MediaCommand) -> Result<()> {
    super::winrt::run(move || {
        let session = current_session()?.context("nothing is playing right now")?;
        let accepted = match command {
            MediaCommand::Play => session.TryPlayAsync(),
            MediaCommand::Pause => session.TryPauseAsync(),
            MediaCommand::PlayPause => session.TryTogglePlayPauseAsync(),
            MediaCommand::Next => session.TrySkipNextAsync(),
            MediaCommand::Previous => session.TrySkipPreviousAsync(),
        }
        .context("sending the media command")?
        .get()
        .context("waiting for the media app")?;
        if !accepted {
            bail!("the media app did not accept that command");
        }
        Ok(())
    })
}
