import { useState } from 'react';
import { useWeather, weatherLocation } from '../services/weather';
import { Icon } from './sdk/primitives';
import type { WidgetViewProps } from './sdk/types';
import './weather.css';

function condition(code: number) {
  if (code === 0) return 'Clear';
  if (code === 1) return 'Mainly clear';
  if (code === 2) return 'Partly cloudy';
  if (code === 3) return 'Overcast';
  if (code === 45 || code === 48) return 'Fog';
  if (code >= 51 && code <= 57) return 'Drizzle';
  if (code >= 61 && code <= 67) return 'Rain';
  if (code >= 71 && code <= 77) return 'Snow';
  if (code >= 80 && code <= 82) return 'Rain showers';
  if (code === 85 || code === 86) return 'Snow showers';
  if (code >= 95 && code <= 99) return 'Thunderstorm';
  return 'Conditions unavailable';
}

export function WeatherWidget({ settings, size }: WidgetViewProps) {
  const snapshot = useWeather(settings);
  const configured = weatherLocation(settings);
  const data = snapshot.data;
  const state = !configured ? 'Weather not configured' : snapshot.status === 'loading' ? 'Loading' : snapshot.status === 'stale' ? 'Stale data' : !data ? 'Weather unavailable' : null;
  const label = typeof settings.weatherLabel === 'string' && settings.weatherLabel.trim() ? settings.weatherLabel.slice(0, 64) : 'Weather';
  return <div className="fresh-weather-main weather-live" title="Weather by Open-Meteo · °C. Configure coordinates in Widgets.">
    {size === 'compact' && <small className="compact-provider-state" role="status">{!configured ? 'Offline' : state ?? 'Open-Meteo'}</small>}
    <div className="fresh-weather-icon"><Icon name="weather" className="fresh-icon" /></div>
    <div className="fresh-weather-copy">
      <span className="fresh-kicker">{label}</span>
      <strong>{data ? `${Math.round(data.temperature)}°` : '—°'}</strong>
      {size !== 'compact' && <small role={state ? 'status' : undefined}>{state ?? condition(data!.weatherCode)}</small>}
    </div>
    {size === 'wide' && data && <><div className="fresh-weather-divider" /><div className="fresh-weather-forecast">
      {data.hours.map((hour) => <div className="fw-hour" key={hour.time} title={condition(hour.weatherCode)}>
        <span>{String(new Date((hour.time + data.utcOffsetSeconds) * 1000).getUTCHours()).padStart(2, '0')}:00</span>
        <Icon name="weather" /><b>{Math.round(hour.temperature)}°</b>
      </div>)}
    </div></>}
  </div>;
}

export function WeatherSettings({ settings, onChange }: { settings: Record<string, unknown>; onChange: (settings: Record<string, unknown>) => void }) {
  const [enabled, setEnabled] = useState(settings.weatherEnabled === true);
  const [latitude, setLatitude] = useState(typeof settings.weatherLatitude === 'number' ? String(settings.weatherLatitude) : '');
  const [longitude, setLongitude] = useState(typeof settings.weatherLongitude === 'number' ? String(settings.weatherLongitude) : '');
  const [label, setLabel] = useState(typeof settings.weatherLabel === 'string' ? settings.weatherLabel : '');
  const [error, setError] = useState('');
  return <form className="weather-settings" aria-label="Weather location settings" noValidate onSubmit={(event) => {
    event.preventDefault();
    const next = { ...settings, weatherEnabled: enabled, weatherLatitude: Number(latitude), weatherLongitude: Number(longitude), weatherLabel: label.trim().slice(0, 64) };
    if (enabled && (!latitude.trim() || !longitude.trim() || !weatherLocation(next))) { setError('Enter latitude −90 to 90 and longitude −180 to 180.'); return; }
    setError('');
    onChange(enabled ? next : { ...settings, weatherEnabled: false });
  }}>
    <label><input type="checkbox" checked={enabled} onChange={(event) => setEnabled(event.target.checked)} />Enable weather (send coordinates to Open-Meteo)</label>
    <label>Location label<input value={label} maxLength={64} onChange={(event) => setLabel(event.target.value)} /></label>
    <div className="weather-coordinate-fields">
      <label>Latitude<input type="number" step="any" min={-90} max={90} value={latitude} disabled={!enabled} onChange={(event) => setLatitude(event.target.value)} /></label>
      <label>Longitude<input type="number" step="any" min={-180} max={180} value={longitude} disabled={!enabled} onChange={(event) => setLongitude(event.target.value)} /></label>
    </div>
    <small>No automatic location lookup. Weather by Open-Meteo. Celsius; refreshes every 10 minutes.</small>
    <button type="submit">Apply weather settings</button>
    {error && <small role="alert">{error}</small>}
  </form>;
}
