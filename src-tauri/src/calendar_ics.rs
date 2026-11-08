use chrono::{Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub title: String,
    pub location: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub all_day: bool,
}
#[derive(Default)]
struct EventFields {
    start: Option<(String, Vec<String>)>,
    end: Option<(String, Vec<String>)>,
    title: Option<String>,
    location: Option<String>,
    uid: Option<String>,
    cancelled: bool,
}
fn text(raw: &str, maximum: usize) -> Result<String, String> {
    let mut value = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            value.push(c);
            continue;
        }
        value.push(match chars.next() {
            Some('n' | 'N') => '\n',
            Some(c @ ('\\' | ',' | ';')) => c,
            _ => return Err("Invalid escaped calendar text".into()),
        });
    }
    Ok(value.chars().take(maximum).collect())
}
fn date(raw: &str) -> Result<NaiveDate, String> {
    if raw.len() != 8 || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Invalid calendar date".into());
    }
    NaiveDate::parse_from_str(raw, "%Y%m%d").map_err(|_| "Invalid calendar date".into())
}
fn local_ms(value: NaiveDateTime) -> Result<i64, String> {
    Local
        .from_local_datetime(&value)
        .single()
        .map(|date| date.timestamp_millis())
        .ok_or_else(|| "Calendar contains an ambiguous or nonexistent local time".into())
}
fn moment(raw: &str, params: &[String]) -> Result<(i64, bool), String> {
    let mut all_day = false;
    let mut utc = false;
    let mut seen = HashSet::new();
    for param in params {
        let (key, value) = param
            .split_once('=')
            .ok_or("Invalid calendar date parameter")?;
        if !seen.insert(key) {
            return Err("Duplicate calendar date parameter".into());
        }
        match key {
            "VALUE" if value.eq_ignore_ascii_case("DATE") => all_day = true,
            "VALUE" if value.eq_ignore_ascii_case("DATE-TIME") => {}
            "TZID"
                if ["UTC", "ETC/UTC", "GMT"]
                    .contains(&value.trim_matches('"').to_ascii_uppercase().as_str()) =>
            {
                utc = true
            }
            "TZID" => {
                return Err(
                    "Calendar timezone is unsupported; export UTC or floating local times".into(),
                )
            }
            _ => return Err("Unsupported calendar date parameter".into()),
        }
    }
    if all_day {
        if utc {
            return Err("All-day dates must not contain a timezone".into());
        }
        return Ok((
            local_ms(
                date(raw)?
                    .and_hms_opt(0, 0, 0)
                    .ok_or("Invalid all-day date")?,
            )?,
            true,
        ));
    }
    let has_z = raw.ends_with('Z');
    let value = raw.strip_suffix('Z').unwrap_or(raw);
    if value.len() != 15
        || value.as_bytes()[8] != b'T'
        || !value
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 8 || b.is_ascii_digit())
        || &value[13..] == "60"
    {
        return Err("Invalid or unsupported calendar date-time".into());
    }
    let value = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .map_err(|_| "Invalid calendar date-time")?;
    Ok((
        if utc || has_z {
            Utc.from_utc_datetime(&value).timestamp_millis()
        } else {
            local_ms(value)?
        },
        false,
    ))
}
impl EventFields {
    fn finish(self) -> Result<Option<Event>, String> {
        if self.cancelled {
            return Ok(None);
        }
        let (raw_start, params) = self.start.ok_or("Calendar event has no start time")?;
        let (start_ms, all_day) = moment(&raw_start, &params)?;
        let end_ms = if let Some((value, params)) = self.end {
            let (end, end_all_day) = moment(&value, &params)?;
            if all_day != end_all_day || end <= start_ms {
                return Err("Invalid calendar event end".into());
            }
            end
        } else if all_day {
            let next = date(&raw_start)?
                .checked_add_signed(Duration::days(1))
                .ok_or("Invalid all-day end")?;
            local_ms(next.and_hms_opt(0, 0, 0).ok_or("Invalid all-day end")?)?
        } else {
            start_ms
        };
        Ok(Some(Event {
            title: self.title.unwrap_or_else(|| "Untitled event".into()),
            location: self.location.unwrap_or_default(),
            start_ms,
            end_ms,
            all_day,
        }))
    }
}

/// A deliberate one-off-event subset: reject recurrence and unsupported timezones before publishing any events.
pub fn parse(input: &str, now_ms: i64) -> Result<Vec<Event>, String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in input.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim_end_matches('\r');
        if line.starts_with([' ', '\t']) {
            lines
                .last_mut()
                .ok_or("Invalid calendar line folding")?
                .push_str(&line[1..]);
        } else if !line.is_empty() {
            lines.push(line.to_string());
        }
    }
    let mut stack: Vec<String> = Vec::new();
    let mut fields: Option<EventFields> = None;
    let mut events = Vec::new();
    let mut seen_uids = HashSet::new();
    let mut opened = false;
    let mut closed = false;
    let mut version = false;
    let mut event_count = 0;
    let mut singleton_fields = HashSet::new();
    for line in lines {
        let (head, value) = line
            .split_once(':')
            .ok_or("Malformed calendar content line")?;
        let mut parts = head.split(';');
        let name = parts.next().unwrap_or("").to_ascii_uppercase();
        let params: Vec<String> = parts
            .map(|part| {
                part.split_once('=')
                    .map(|(key, value)| format!("{}={value}", key.to_ascii_uppercase()))
                    .unwrap_or_else(|| part.to_string())
            })
            .collect();
        if name == "BEGIN" {
            let component = value.to_ascii_uppercase();
            if stack.len() >= 8 {
                return Err("Calendar nesting limit exceeded".into());
            }
            match component.as_str() {
                "VCALENDAR" if !opened && stack.is_empty() => opened = true,
                "VEVENT" if stack.as_slice() == ["VCALENDAR"] => {
                    event_count += 1;
                    if event_count > 1000 {
                        return Err("Calendar contains more than 1000 events".into());
                    }
                    fields = Some(EventFields::default());
                    singleton_fields.clear();
                }
                "VTIMEZONE" => {
                    return Err(
                        "Embedded calendar timezones are unsupported; export UTC times".into(),
                    )
                }
                "VCALENDAR" | "VEVENT" => return Err("Invalid calendar component nesting".into()),
                _ if !opened || closed => return Err("Invalid calendar structure".into()),
                _ => {}
            }
            stack.push(component);
            continue;
        }
        if name == "END" {
            let component = value.to_ascii_uppercase();
            if stack.pop().as_deref() != Some(component.as_str()) {
                return Err("Mismatched calendar component".into());
            }
            if component == "VEVENT" {
                let event = fields.take().ok_or("Invalid calendar event")?;
                if let Some(uid) = &event.uid {
                    if !seen_uids.insert(uid.clone()) {
                        return Err("Duplicate calendar event identities are unsupported".into());
                    }
                }
                if let Some(event) = event.finish()? {
                    events.push(event);
                }
            }
            if component == "VCALENDAR" {
                closed = true;
            }
            continue;
        }
        if stack.as_slice() == ["VCALENDAR"] {
            if name == "VERSION" {
                if value != "2.0" {
                    return Err("Only iCalendar 2.0 is supported".into());
                }
                version = true;
            }
            if name == "METHOD" && !value.eq_ignore_ascii_case("PUBLISH") {
                return Err("Calendar invitation/cancellation methods are unsupported".into());
            }
        } else if stack.last().map(String::as_str) == Some("VEVENT") {
            let event = fields.as_mut().ok_or("Invalid calendar event")?;
            if ["SUMMARY", "LOCATION", "UID", "STATUS"].contains(&name.as_str())
                && !singleton_fields.insert(name.clone())
            {
                return Err("Duplicate calendar event field".into());
            }
            match name.as_str() {
                "DTSTART" => {
                    if event.start.replace((value.to_string(), params)).is_some() {
                        return Err("Duplicate event start".into());
                    }
                }
                "DTEND" => {
                    if event.end.replace((value.to_string(), params)).is_some() {
                        return Err("Duplicate event end".into());
                    }
                }
                "SUMMARY" => event.title = Some(text(value, 120)?),
                "LOCATION" => event.location = Some(text(value, 80)?),
                "UID" => event.uid = Some(value.to_string()),
                "STATUS" => event.cancelled = value.eq_ignore_ascii_case("CANCELLED"),
                "RRULE" | "RDATE" | "EXDATE" | "RECURRENCE-ID" => return Err(
                    "Recurring calendar events are unsupported; select a one-off-event calendar"
                        .into(),
                ),
                "DURATION" => {
                    return Err(
                        "Calendar DURATION is unsupported; export explicit end times".into(),
                    )
                }
                _ => {}
            }
        } else if stack.is_empty() {
            return Err("Content outside calendar".into());
        }
    }
    if !opened || !closed || !version || !stack.is_empty() {
        return Err("Incomplete iCalendar document".into());
    }
    events.retain(|event| event.start_ms >= now_ms || event.end_ms > now_ms);
    events.sort_by_key(|event| event.start_ms);
    events.truncate(3);
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn calendar(body: &str) -> String {
        format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{body}\r\nEND:VCALENDAR\r\n")
    }
    #[test]
    fn unfolds_escaped_titles_and_keeps_ongoing_events_in_order() {
        let input = calendar("BEGIN:VEVENT\r\nDTSTART:20261105T100000Z\r\nDTEND:20261105T120000Z\r\nSUMMARY:Review\\, notes\\nnext\r\n line 😀\r\nLOCATION:Room\\; 2\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nDTSTART:20261105T130000Z\r\nSUMMARY:Later\r\nEND:VEVENT");
        let now = Utc
            .with_ymd_and_hms(2026, 11, 5, 11, 0, 0)
            .unwrap()
            .timestamp_millis();
        let events = parse(&input, now).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Review, notes\nnextline 😀");
        assert_eq!(events[0].location, "Room; 2");
        assert!(events[0].start_ms < now && events[0].end_ms > now);
    }
    #[test]
    fn all_day_end_is_exclusive_and_missing_end_is_next_local_day() {
        let input =
            calendar("BEGIN:VEVENT\nDTSTART;VALUE=DATE:20261105\nSUMMARY:All day\nEND:VEVENT");
        let start = local_ms(date("20261105").unwrap().and_hms_opt(0, 0, 0).unwrap()).unwrap();
        let events = parse(&input, start).unwrap();
        assert!(events[0].all_day);
        let end = local_ms(date("20261106").unwrap().and_hms_opt(0, 0, 0).unwrap()).unwrap();
        assert_eq!(events[0].end_ms, end);
        assert!(parse(&input, end).unwrap().is_empty());
    }
    #[test]
    fn supports_explicit_utc_and_floating_local_time_without_guessing_zones() {
        let input = calendar("BEGIN:VEVENT\nDTSTART;TZID=UTC:20261105T120000\nEND:VEVENT\nBEGIN:VEVENT\nDTSTART:20261105T130000\nEND:VEVENT");
        let events = parse(&input, 0).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().any(|event| event.start_ms
            == Utc
                .with_ymd_and_hms(2026, 11, 5, 12, 0, 0)
                .unwrap()
                .timestamp_millis()));
    }
    #[test]
    fn rejects_unsupported_or_malformed_semantics_instead_of_guessing_next_event() {
        for line in [
            "DTSTART;TZID=Europe/Paris:20261105T120000",
            "DTSTART:20260230T120000Z",
            "DTSTART:20261105T120000Z\nRRULE:FREQ=DAILY",
            "DTSTART:20261105T120000Z\nDURATION:PT1H",
            "DTSTART;VALUE=DATE:20261105\nDTEND:20261105T140000Z",
            "DTSTART;VALUE=DATE;VALUE=DATE-TIME:20261105",
            "DTSTART:20261105T120000Z\nSTATUS:CANCELLED\nSTATUS:CONFIRMED",
            "DTSTART:20261105T120000Z\nUID:first\nUID:second",
        ] {
            assert!(parse(&calendar(&format!("BEGIN:VEVENT\n{line}\nEND:VEVENT")), 0).is_err());
        }
        assert!(parse(&calendar("BEGIN:VTIMEZONE\nEND:VTIMEZONE"), 0).is_err());
        assert!(parse("BEGIN:VCALENDAR\nVERSION:2.0", 0).is_err());
    }
}
