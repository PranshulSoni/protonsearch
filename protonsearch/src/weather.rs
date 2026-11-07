//! Opt-in weather for saved dock coordinates. No geolocation or arbitrary URL fetching.
use serde_json::{json, Value};
use std::{
    io::Read,
    path::Path,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Cache {
    coordinates: (f64, f64),
    snapshot: Value,
    next: Instant,
}

fn allowed_profile(version: u64, profile: &str) -> bool {
    match version {
        1 => matches!(profile, "daily" | "founder"),
        2 => matches!(profile, "daily" | "work" | "founder" | "focus"),
        _ => false,
    }
}

fn location(layout: &Value) -> Result<Option<(f64, f64)>, String> {
    let version = layout["version"]
        .as_u64()
        .filter(|version| matches!(version, 1 | 2))
        .ok_or("Invalid weather layout version")?;
    let profile = layout["profile"]
        .as_str()
        .ok_or("Invalid weather profile")?;
    if !allowed_profile(version, profile) {
        return Err("Invalid weather profile".into());
    }
    let items = layout["profiles"][profile]
        .as_array()
        .ok_or("Invalid weather layout")?;
    let Some(widget) = items.iter().find(|item| item["id"] == "weather") else {
        return Ok(None);
    };
    let settings = &widget["settings"];
    if settings["weatherEnabled"] != true {
        return Ok(None);
    }
    let latitude = settings["weatherLatitude"]
        .as_f64()
        .filter(|v| v.is_finite() && (-90.0..=90.0).contains(v));
    let longitude = settings["weatherLongitude"]
        .as_f64()
        .filter(|v| v.is_finite() && (-180.0..=180.0).contains(v));
    Ok(Some((
        latitude.ok_or("Invalid weather latitude")?,
        longitude.ok_or("Invalid weather longitude")?,
    )))
}

fn bounded_json(reader: impl Read, limit: usize) -> Result<Value, String> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read weather response")?;
    if bytes.len() > limit {
        return Err("Weather response too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid weather response".into())
}

fn temperature(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|v| v.is_finite() && (-120.0..=70.0).contains(v))
        .ok_or("Invalid weather temperature".into())
}
fn code(value: &Value) -> Result<u64, String> {
    value
        .as_u64()
        .filter(|v| *v <= 99)
        .ok_or("Invalid weather conditions".into())
}
fn timestamp(value: &Value) -> Result<i64, String> {
    value
        .as_i64()
        .filter(|v| (0..=4_102_444_800).contains(v))
        .ok_or("Invalid weather time".into())
}
fn parse_weather(value: Value) -> Result<Value, String> {
    let current = &value["current"];
    let time = timestamp(&current["time"])?;
    let offset = value["utc_offset_seconds"]
        .as_i64()
        .filter(|v| (-86400..=86400).contains(v))
        .ok_or("Invalid weather timezone")?;
    let hourly = &value["hourly"];
    let times = hourly["time"]
        .as_array()
        .ok_or("Missing weather forecast")?;
    let temperatures = hourly["temperature_2m"]
        .as_array()
        .ok_or("Missing weather forecast")?;
    let codes = hourly["weather_code"]
        .as_array()
        .ok_or("Missing weather forecast")?;
    if times.is_empty()
        || times.len() > 12
        || times.len() != temperatures.len()
        || times.len() != codes.len()
    {
        return Err("Invalid weather forecast length".into());
    }
    let visibilities = hourly["visibility"].as_array();
    if visibilities.is_some_and(|values| values.len() != times.len()) {
        return Err("Invalid weather visibility".into());
    }
    let mut hours = Vec::new();
    let mut previous = None;
    let mut closest: Option<(i64, usize)> = None;
    for (index, value) in times.iter().enumerate() {
        let hour = timestamp(value)?;
        if previous.is_some_and(|previous| hour <= previous) {
            return Err("Invalid weather forecast order".into());
        }
        previous = Some(hour);
        let distance = (hour - time).abs();
        if closest.is_none_or(|(current, _)| distance < current) {
            closest = Some((distance, index));
        }
        let item = json!({"time":hour,"temperature":temperature(&temperatures[index])?,"weatherCode":code(&codes[index])?});
        if hour > time && hours.len() < 6 {
            hours.push(item);
        }
    }
    let visibility_km = closest
        .and_then(|(_, index)| {
            visibilities
                .and_then(|values| values.get(index))
                .and_then(Value::as_f64)
        })
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|meters| (meters / 100.0).round() / 10.0);

    let current = &value["current"];
    let daily = &value["daily"];
    let high_c = daily["temperature_2m_max"]
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && (-100.0..=70.0).contains(value));
    let low_c = daily["temperature_2m_min"]
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && (-100.0..=70.0).contains(value));
    let uv_index = daily["uv_index_max"]
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && (0.0..=30.0).contains(value));
    let feels_like_c = current["apparent_temperature"]
        .as_f64()
        .filter(|value| value.is_finite() && (-100.0..=70.0).contains(value));
    let wind_kph = current["wind_speed_10m"]
        .as_f64()
        .filter(|value| value.is_finite() && (0.0..=1_000.0).contains(value));
    let humidity_percent = current["relative_humidity_2m"]
        .as_f64()
        .filter(|value| (0.0..=100.0).contains(value));

    let mut data = json!({"temperature":temperature(&current["temperature_2m"])?,"weatherCode":code(&current["weather_code"])?,"time":time,"utcOffsetSeconds":offset,"hours":hours});
    for (key, value) in [
        ("highC", high_c),
        ("lowC", low_c),
        ("uvIndex", uv_index),
        ("feelsLikeC", feels_like_c),
        ("windKph", wind_kph),
        ("humidityPercent", humidity_percent),
        ("visibilityKm", visibility_km),
    ] {
        if let Some(value) = value {
            data[key] = json!(value);
        }
    }
    Ok(data)
}

fn fetch(coordinates: (f64, f64)) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(8))
        .timeout_connect(Duration::from_secs(4))
        .timeout_read(Duration::from_secs(4))
        .build();
    let response = agent
        .get("https://api.open-meteo.com/v1/forecast")
        .query("latitude", &coordinates.0.to_string())
        .query("longitude", &coordinates.1.to_string())
        .query(
            "current",
            "temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m",
        )
        .query(
            "daily",
            "temperature_2m_max,temperature_2m_min,uv_index_max",
        )
        .query("hourly", "temperature_2m,weather_code,visibility")
        .query("forecast_hours", "10")
        .query("wind_speed_unit", "kmh")
        .query("timezone", "auto")
        .query("timeformat", "unixtime")
        .call()
        .map_err(|_| "Weather service unavailable")?;
    bounded_json(response.into_reader(), 65_536)
}

fn refresh(
    coordinates: (f64, f64),
    cache: &mut Option<Cache>,
    now: Instant,
    fetch: impl FnOnce() -> Result<Value, String>,
) -> Result<Value, String> {
    if let Some(entry) = cache
        .as_ref()
        .filter(|entry| entry.coordinates == coordinates && now < entry.next)
    {
        return Ok(entry.snapshot.clone());
    }
    let result = fetch().and_then(parse_weather);
    let success = result.is_ok();
    let snapshot = match result {
        Ok(data) => {
            json!({"status":"live","data":data,"updatedAt":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "Invalid system clock")?.as_millis()})
        }
        Err(message) => {
            let mut previous = cache
                .as_ref()
                .filter(|entry| {
                    entry.coordinates == coordinates && entry.snapshot.get("data").is_some()
                })
                .map(|entry| entry.snapshot.clone())
                .unwrap_or_else(|| json!({"updatedAt":0}));
            previous["status"] = json!(if previous.get("data").is_some() {
                "stale"
            } else {
                "error"
            });
            previous["message"] = json!(message);
            previous
        }
    };
    *cache = Some(Cache {
        coordinates,
        snapshot: snapshot.clone(),
        next: now + Duration::from_secs(if success { 600 } else { 60 }),
    });
    Ok(snapshot)
}

fn coordinates(latitude: f64, longitude: f64) -> Result<(f64, f64), String> {
    if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
        return Err("Invalid weather latitude".into());
    }
    if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
        return Err("Invalid weather longitude".into());
    }
    Ok((latitude, longitude))
}

fn snapshot_for(coordinates: (f64, f64)) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<Option<Cache>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Weather cache unavailable")?;
    let mut value = refresh(coordinates, &mut cache, Instant::now(), || {
        fetch(coordinates)
    })?;
    value["location"] = json!({"latitude":coordinates.0,"longitude":coordinates.1});
    Ok(value)
}

/// Read the persisted, active profile's opt-in before any external request.
pub fn snapshot(layout_path: &Path) -> Result<Value, String> {
    let file = match std::fs::File::open(layout_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"status":"disconnected","updatedAt":0}))
        }
        Err(_) => return Err("Cannot read weather settings".into()),
    };
    let Some(coordinates) = location(&bounded_json(file, 1_048_576)?)? else {
        return Ok(json!({"status":"disconnected","updatedAt":0}));
    };
    snapshot_for(coordinates)
}

/// Fetch weather for coordinates the frontend already resolved from active settings.
pub fn snapshot_coordinates(latitude: f64, longitude: f64) -> Result<Value, String> {
    snapshot_for(coordinates(latitude, longitude)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn layout(settings: Value) -> Value {
        json!({"version":1,"profile":"daily","profiles":{"daily":[{"id":"weather","settings":settings}]}})
    }
    fn layout_for(version: u64, profile: &str, settings: Value) -> Value {
        json!({"version":version,"profile":profile,"profiles":{profile:[{"id":"weather","settings":settings}]}})
    }
    fn fixture() -> Value {
        json!({
            "utc_offset_seconds":19800,
            "current":{
                "time":1760000400_i64,
                "temperature_2m":21.5,
                "relative_humidity_2m":64,
                "apparent_temperature":23.0,
                "weather_code":2,
                "wind_speed_10m":14.0
            },
            "daily":{
                "temperature_2m_max":[28.0],
                "temperature_2m_min":[18.0],
                "uv_index_max":[5.4]
            },
            "hourly":{
                "time":[1760000400_i64,1760004000,1760007600],
                "temperature_2m":[21.5,22.0,23.0],
                "weather_code":[2,3,0],
                "visibility":[12000,10000,8000]
            }
        })
    }

    #[test]
    fn location_requires_explicit_opt_in_and_finite_bounded_coordinates() {
        assert!(location(&layout(json!({}))).unwrap().is_none());
        assert!(location(&layout(
            json!({"weatherEnabled":false,"weatherLatitude":500})
        ))
        .unwrap()
        .is_none());
        for settings in [
            json!({"weatherEnabled":true}),
            json!({"weatherEnabled":true,"weatherLatitude":91,"weatherLongitude":0}),
            json!({"weatherEnabled":true,"weatherLatitude":0,"weatherLongitude":181}),
        ] {
            assert!(location(&layout(settings)).is_err());
        }
        assert_eq!(
            location(&layout(
                json!({"weatherEnabled":true,"weatherLatitude":0,"weatherLongitude":0})
            ))
            .unwrap(),
            Some((0.0, 0.0))
        );
    }

    #[test]
    fn version_two_accepts_all_profiles_and_reads_the_active_profile_item() {
        let settings =
            json!({"weatherEnabled":true,"weatherLatitude":12.5,"weatherLongitude":-45.0});
        for profile in ["daily", "work", "founder", "focus"] {
            assert_eq!(
                location(&layout_for(2, profile, settings.clone())).unwrap(),
                Some((12.5, -45.0)),
                "profile {profile}"
            );
        }
    }

    #[test]
    fn version_two_requires_the_active_profile_item_instead_of_falling_back() {
        let value = json!({"version":2,"profile":"work","profiles":{
            "daily":[{"id":"weather","settings":{"weatherEnabled":true,"weatherLatitude":1.0,"weatherLongitude":2.0}}],
            "work":[{"id":"clock"}]}});
        assert_eq!(location(&value).unwrap(), None);
        let mut value = value;
        value["profiles"]["work"] = json!([{"id":"weather"}]);
        assert_eq!(location(&value).unwrap(), None);
    }

    #[test]
    fn version_two_respects_disabled_weather_and_rejects_invalid_profiles_or_versions() {
        let disabled = json!({"weatherEnabled":false,"weatherLatitude":0,"weatherLongitude":0});
        assert_eq!(location(&layout_for(2, "focus", disabled)).unwrap(), None);
        assert!(location(&layout_for(2, "later", json!({}))).is_err());
        assert!(location(&layout_for(1, "work", json!({}))).is_err());
        assert!(location(&layout_for(3, "daily", json!({}))).is_err());
    }

    #[test]
    fn direct_coordinates_are_validated_before_any_fetch() {
        for (latitude, longitude) in [
            (91.0, 0.0),
            (-91.0, 0.0),
            (0.0, 181.0),
            (0.0, -181.0),
            (f64::NAN, 0.0),
            (0.0, f64::INFINITY),
        ] {
            assert!(snapshot_coordinates(latitude, longitude).is_err());
        }
    }

    #[test]
    fn parser_returns_real_values_and_rejects_missing_or_out_of_range_measurements() {
        let data = parse_weather(fixture()).unwrap();
        assert_eq!(data["temperature"], 21.5);
        assert_eq!(data["hours"].as_array().unwrap().len(), 2);
        assert_eq!(data["highC"], 28.0);
        assert_eq!(data["lowC"], 18.0);
        assert_eq!(data["feelsLikeC"], 23.0);
        assert_eq!(data["windKph"], 14.0);
        assert_eq!(data["humidityPercent"], 64.0);
        assert_eq!(data["uvIndex"], 5.4);
        assert_eq!(data["visibilityKm"], 12.0);
        let mut value = fixture();
        value["current"]["temperature_2m"] = json!(null);
        assert!(parse_weather(value).is_err());
        let mut value = fixture();
        value["current"]["temperature_2m"] = json!(300);
        assert!(parse_weather(value).is_err());
        let mut value = fixture();
        value["hourly"]["time"] = json!([]);
        assert!(parse_weather(value).is_err());
    }

    #[test]
    fn bounded_reader_and_unknown_layout_reject_before_fetching() {
        assert!(bounded_json(vec![b' '; 65_537].as_slice(), 65_536).is_err());
        assert!(bounded_json(b"not-json".as_slice(), 65_536).is_err());
        let mut value =
            layout(json!({"weatherEnabled":true,"weatherLatitude":0,"weatherLongitude":0}));
        value["version"] = json!(3);
        assert!(location(&value).is_err());
    }

    #[test]
    fn cache_coalesces_requests_and_keeps_explicit_stale_state_on_failure() {
        let now = Instant::now();
        let mut cache = None;
        let first = refresh((10.0, 20.0), &mut cache, now, || Ok(fixture())).unwrap();
        assert_eq!(first["status"], "live");
        assert_eq!(
            refresh((10.0, 20.0), &mut cache, now, || panic!(
                "cached request fetched"
            ))
            .unwrap()["status"],
            "live"
        );
        let failed = refresh(
            (10.0, 20.0),
            &mut cache,
            now + Duration::from_secs(601),
            || Err("Weather service unavailable".into()),
        )
        .unwrap();
        assert_eq!(failed["status"], "stale");
        assert_eq!(failed["data"]["temperature"], 21.5);
        let other = refresh(
            (11.0, 20.0),
            &mut cache,
            now + Duration::from_secs(602),
            || Err("Weather service unavailable".into()),
        )
        .unwrap();
        assert_eq!(other["status"], "error");
        assert!(other.get("data").is_none());
    }
}
