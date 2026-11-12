//! Now-playing media through the system media transport controls: the same
//! session Windows shows in its own volume flyout. User-level; no admin and no
//! package identity needed.

use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as MediaProperties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Storage::Streams::DataReader;

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

/// Thumbnails larger than this are not sent to the island.
// shortcut: the 1 s status poll re-sends the art, so keep it small; send it only on track change if IPC cost shows up.
const MAX_ARTWORK_BYTES: u64 = 256 * 1024;

/// Thumbnail of the last track read, keyed by title and artist. The poll asks every
/// second, so the thumbnail is read once per track; a failed or missing read is cached
/// as "no artwork" too, so a player without thumbnails costs one read per track.
static ARTWORK: Mutex<Option<(String, Option<String>)>> = Mutex::new(None);

fn artwork_for(track: &str, properties: &MediaProperties) -> Option<String> {
    let mut cache = ARTWORK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((cached, artwork)) = cache.as_ref() {
        if cached == track {
            return artwork.clone();
        }
    }
    let artwork = read_thumbnail(properties).ok().flatten();
    *cache = Some((track.to_string(), artwork.clone()));
    artwork
}

/// The session thumbnail as a `data:` URL, or `None` when there is none or it is too large.
fn read_thumbnail(properties: &MediaProperties) -> Result<Option<String>> {
    let Ok(reference) = properties.Thumbnail() else {
        return Ok(None);
    };
    let stream = reference
        .OpenReadAsync()
        .context("opening the media thumbnail")?
        .get()
        .context("waiting for the media thumbnail")?;
    let size = stream.Size().context("reading the thumbnail size")?;
    if size == 0 || size > MAX_ARTWORK_BYTES {
        return Ok(None);
    }
    let input = stream.GetInputStreamAt(0).context("opening the thumbnail stream")?;
    let reader = DataReader::CreateDataReader(&input).context("creating a data reader")?;
    reader
        .LoadAsync(size as u32)
        .context("loading the thumbnail")?
        .get()
        .context("waiting for the thumbnail bytes")?;
    let length = reader.UnconsumedBufferLength().context("reading the thumbnail length")?;
    let mut bytes = vec![0u8; length as usize];
    reader.ReadBytes(&mut bytes).context("copying the thumbnail bytes")?;
    let mime = stream.ContentType().map(|value| value.to_string()).unwrap_or_default();
    let mime = if mime.starts_with("image/") { mime } else { "image/png".to_string() };
    Ok(Some(format!("data:{mime};base64,{}", base64(&bytes))))
}

/// Standard base64 with padding (RFC 4648); too small to justify a dependency.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &byte)| acc | (u32::from(byte) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
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
        let title = text(properties.Title());
        let artist = text(properties.Artist());
        let artwork_data_url = artwork_for(&format!("{title}\u{1f}{artist}"), &properties);

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
            title,
            artist,
            album: (!album.is_empty()).then_some(album),
            app_id: session.SourceAppUserModelId().ok().map(|id| id.to_string()),
            playing,
            position_ms,
            duration_ms,
            can_seek: playback
                .Controls()
                .and_then(|controls| controls.IsPlaybackPositionEnabled())
                .unwrap_or(false),
            artwork_data_url,
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
            MediaCommand::Seek(position_ms) => {
                // The requested position counts from the timeline's own start time.
                let start = session
                    .GetTimelineProperties()
                    .and_then(|timeline| timeline.StartTime())
                    .map(|time| time.Duration)
                    .unwrap_or(0);
                let ticks = i64::try_from(position_ms).unwrap_or(i64::MAX).saturating_mul(10_000);
                session.TryChangePlaybackPositionAsync(start.saturating_add(ticks))
            }
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

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
