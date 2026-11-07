import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { isNative } from './native';
import type { Snapshot } from '../widgets/sdk/types';

export type WeatherData = {
  temperature: number; weatherCode: number; time: number; utcOffsetSeconds: number;
  hours: Array<{ time: number; temperature: number; weatherCode: number }>;
};
type WeatherSnapshot = Snapshot<WeatherData> & { location?: { latitude: number; longitude: number } };
export function weatherLocation(settings: Record<string, unknown>) {
  const latitude = settings.weatherLatitude, longitude = settings.weatherLongitude;
  return settings.weatherEnabled === true && typeof latitude === 'number' && Number.isFinite(latitude) && Math.abs(latitude) <= 90
    && typeof longitude === 'number' && Number.isFinite(longitude) && Math.abs(longitude) <= 180 ? { latitude, longitude } : null;
}

export function useWeather(settings: Record<string, unknown>): Snapshot<WeatherData> {
  const coordinates = weatherLocation(settings);
  const [snapshot, setSnapshot] = useState<Snapshot<WeatherData>>({ status: 'disconnected', updatedAt: 0 });
  useEffect(() => {
    if (!coordinates || !isNative()) {
      setSnapshot({ status: 'disconnected', updatedAt: 0 });
      return;
    }
    let current = true, pending = false, queued = false;
    setSnapshot({ status: 'loading', updatedAt: 0 });
    const refresh = () => {
      if (!current || document.hidden) return;
      if (pending) { queued = true; return; }
      pending = true;
      void invoke<WeatherSnapshot>('weather_snapshot')
        .then((value) => {
          if (current && value.location?.latitude === coordinates.latitude && value.location?.longitude === coordinates.longitude) setSnapshot(value);
        })
        .catch(() => { if (current) setSnapshot((previous) => ({ ...previous, status: previous.data ? 'stale' : 'error', message: 'Weather service unavailable' })); })
        .finally(() => {
          pending = false;
          if (queued && current) { queued = false; refresh(); }
        });
    };
    refresh();
    const timer = setInterval(refresh, 600000);
    const listener = listen('settings-updated', refresh).catch(() => {
      if (current) setSnapshot((previous) => ({ ...previous, status: previous.data ? 'stale' : 'error', message: 'Weather settings updates unavailable' }));
      return () => {};
    });
    document.addEventListener('visibilitychange', refresh);
    return () => {
      current = false;
      clearInterval(timer);
      document.removeEventListener('visibilitychange', refresh);
      void listener.then((unlisten) => unlisten());
    };
  }, [coordinates?.latitude, coordinates?.longitude]);
  return snapshot;
}
