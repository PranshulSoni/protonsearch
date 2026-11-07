import type { ReactNode } from 'react';
import type { FinalIconName } from './types';

export type WeatherKind = 'sun' | 'cloud' | 'rain' | 'partly';
export type AppGlyphName = 'code' | 'terminal' | 'browser' | 'slack' | 'figma' | 'folders' | 'music' | 'notes';

const glyphs: Record<FinalIconName, ReactNode> = {
  windows: (
    <>
      <rect x="3" y="3" width="7.6" height="7.6" rx="1.1" fill="currentColor" stroke="none" />
      <rect x="13.4" y="3" width="7.6" height="7.6" rx="1.1" fill="currentColor" stroke="none" />
      <rect x="3" y="13.4" width="7.6" height="7.6" rx="1.1" fill="currentColor" stroke="none" />
      <rect x="13.4" y="13.4" width="7.6" height="7.6" rx="1.1" fill="currentColor" stroke="none" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="6.4" />
      <path d="m19.8 19.8-3.9-3.9" />
    </>
  ),
  apps: (
    <>
      <rect x="3.2" y="3.2" width="7.2" height="7.2" rx="1.7" />
      <rect x="13.6" y="3.2" width="7.2" height="7.2" rx="1.7" />
      <rect x="3.2" y="13.6" width="7.2" height="7.2" rx="1.7" />
      <rect x="13.6" y="13.6" width="7.2" height="7.2" rx="1.7" />
    </>
  ),
  spotify: (
    <>
      <circle cx="12" cy="12" r="9.2" fill="currentColor" stroke="none" />
      <path d="M7.1 9.2c3.1-1 6.9-.5 9.8 1.2M7.6 12.3c2.6-.8 5.7-.4 8.1 1M8.1 15.2c2.1-.6 4.5-.3 6.3.8" stroke="#0b1a11" strokeWidth="1.8" fill="none" />
    </>
  ),
  weather: (
    <>
      <circle cx="8.4" cy="8.4" r="3.1" />
      <path d="M8.4 2.7v1.2M3 8.4H1.8M5.5 5.5 4.7 4.7M11.3 5.5l.8-.8" />
      <path d="M9.4 13.7h6.4a3.1 3.1 0 0 1 0 6.2H9.4a3.1 3.1 0 0 1 0-6.2Z" />
    </>
  ),
  system: (
    <>
      <path d="M4 18a8 8 0 1 1 16 0" />
      <path d="m12 18 4.2-5.3" />
      <circle cx="12" cy="18" r="1.4" fill="currentColor" stroke="none" />
    </>
  ),
  claude: <path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" />,
  chatgpt: (
    <>
      <path d="M12 2.9 20 7.4v9.2L12 21.1 4 16.6V7.4z" />
      <path d="M8.3 8.9h7.4M8.3 12h7.4M8.3 15.1h7.4" opacity=".65" />
    </>
  ),
  agent: (
    <>
      <rect x="4" y="7" width="16" height="12" rx="3.2" />
      <path d="M12 3.4V7M8.7 12h.01M15.3 12h.01M9.2 15.5h5.6" />
    </>
  ),
  files: (
    <>
      <path d="M6 2.9h7.2l4.8 4.8V21H6z" />
      <path d="M13.2 2.9V7.7H18" />
    </>
  ),
  folders: <path d="M3 7.2a2 2 0 0 1 2-2h3.9l1.8 2H19a2 2 0 0 1 2 2v8.6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
  trash: <path d="M4 6h16M9 6V4h6v2M6.4 6l.9 14h9.4l.9-14M10 10v6M14 10v6" />,
  volume: (
    <>
      <path d="M11 5 6.4 9H3.2v6h3.2L11 19z" />
      <path d="M15.4 8.6a5 5 0 0 1 0 6.8M18.3 5.7a9 9 0 0 1 0 12.6" />
    </>
  ),
  volumeMute: (
    <>
      <path d="M11 5 6.4 9H3.2v6h3.2L11 19z" />
      <path d="m16 9 5 5M21 9l-5 5" />
    </>
  ),
  clock: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.4V12l3.1 2" />
    </>
  ),
  cpu: (
    <>
      <rect x="7" y="7" width="10" height="10" rx="2" />
      <path d="M9.5 2.8V7M14.5 2.8V7M9.5 17v4.2M14.5 17v4.2M2.8 9.5H7M2.8 14.5H7M17 9.5h4.2M17 14.5h4.2" />
    </>
  ),
  ram: (
    <>
      <rect x="3" y="7" width="18" height="10" rx="2" />
      <path d="M7 17v3M12 17v3M17 17v3M7 11h2M11 11h2M15 11h2" />
    </>
  ),
  temp: (
    <>
      <path d="M10 14.2V5a2 2 0 1 1 4 0v9.2a4 4 0 1 1-4 0Z" />
      <path d="M12 9v6" />
    </>
  ),
  battery: (
    <>
      <rect x="3" y="7.5" width="16" height="9" rx="2" />
      <path d="M21 10.5v3" />
      <rect x="5.4" y="10" width="9" height="4" rx="1" fill="currentColor" stroke="none" />
    </>
  ),
  wifi: (
    <>
      <path d="M3 9.5a13 13 0 0 1 18 0M6.2 13a9 9 0 0 1 11.6 0M9.3 16.4a5 5 0 0 1 5.4 0" />
      <circle cx="12" cy="19.2" r="1" fill="currentColor" stroke="none" />
    </>
  ),
  bluetooth: <path d="m7 7 10 10-5 4V3l5 4L7 17" />,
  ethernet: (
    <>
      <rect x="4" y="9" width="16" height="8" rx="2" />
      <path d="M8 17v3M16 17v3M8 9V5h8v4M8 13h.01M11 13h.01M14 13h.01" />
    </>
  ),
  vpn: (
    <>
      <path d="M12 3 5 6v5c0 4.5 3 7.8 7 10 4-2.2 7-5.5 7-10V6z" />
      <path d="m9 12 2 2 4-4" />
    </>
  ),
  uptime: (
    <>
      <circle cx="12" cy="13" r="7.4" />
      <path d="M12 9.2v4l2.4 1.4M9 4h6" />
    </>
  ),
  play: <path d="M8 5.5v13l11-6.5z" fill="currentColor" stroke="none" />,
  pause: (
    <>
      <rect x="7" y="5" width="3.4" height="14" rx="1" fill="currentColor" stroke="none" />
      <rect x="13.6" y="5" width="3.4" height="14" rx="1" fill="currentColor" stroke="none" />
    </>
  ),
  prev: (
    <>
      <path d="M19 6v12l-9-6z" fill="currentColor" stroke="none" />
      <path d="M6 5v14" strokeWidth="2.2" />
    </>
  ),
  next: (
    <>
      <path d="M5 6v12l9-6z" fill="currentColor" stroke="none" />
      <path d="M18 5v14" strokeWidth="2.2" />
    </>
  ),
  device: (
    <>
      <rect x="3" y="4" width="18" height="12" rx="2" />
      <path d="M8 20h8M12 16v4" />
    </>
  ),
  close: <path d="m6 6 12 12M18 6 6 18" />,
  back: <path d="m14.5 5-7 7 7 7" />,
  check: <path d="m5 12.5 4.5 4.5L19 7.5" />,
  chevron: <path d="m9 5 7 7-7 7" />,
  drag: (
    <>
      <circle cx="9" cy="7" r="1" fill="currentColor" stroke="none" />
      <circle cx="15" cy="7" r="1" fill="currentColor" stroke="none" />
      <circle cx="9" cy="12" r="1" fill="currentColor" stroke="none" />
      <circle cx="15" cy="12" r="1" fill="currentColor" stroke="none" />
      <circle cx="9" cy="17" r="1" fill="currentColor" stroke="none" />
      <circle cx="15" cy="17" r="1" fill="currentColor" stroke="none" />
    </>
  ),
  sliders: (
    <>
      <path d="M4 7h10M18 7h2M4 17h2M10 17h10" />
      <circle cx="16" cy="7" r="2" />
      <circle cx="8" cy="17" r="2" />
    </>
  ),
  music: (
    <>
      <path d="M9 18V5l12-2v13" />
      <circle cx="6" cy="18" r="3" />
      <circle cx="18" cy="16" r="3" />
    </>
  ),
  calendar: (
    <>
      <rect x="3.5" y="5" width="17" height="15" rx="2" />
      <path d="M8 3v4M16 3v4M3.5 10h17" />
    </>
  ),
  bolt: <path d="M13 2 4.5 13.2H11L9.8 22 19 10.4h-6.4z" />,
  branch: (
    <>
      <circle cx="6.5" cy="6" r="2.3" />
      <circle cx="6.5" cy="18" r="2.3" />
      <circle cx="17.5" cy="8" r="2.3" />
      <path d="M6.5 8.3v7.4M6.5 14c0-3 2.5-4 5.5-4h3.2" />
    </>
  ),
  spark: <path d="M12 3.2 13.6 9l5.8 1.6-5.8 1.6L12 18l-1.6-5.8L4.6 10.6 10.4 9z" />,
  terminal: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="3" />
      <path d="m7 9 3 3-3 3M12.5 15H17" />
    </>
  ),
  wind: <path d="M3 8h9.5a2.5 2.5 0 1 0-2.5-2.5M3 12h13a2.5 2.5 0 1 1-2.5 2.5M3 16h6.5a2 2 0 1 1-2 2" />,
  droplet: <path d="M12 3.2s5.6 6.1 5.6 10a5.6 5.6 0 0 1-11.2 0c0-3.9 5.6-10 5.6-10Z" />,
  uv: (
    <>
      <circle cx="12" cy="12" r="3.4" />
      <path d="M12 2.8v2M12 19.2v2M2.8 12h2M19.2 12h2M5.5 5.5l1.4 1.4M17.1 17.1l1.4 1.4M18.5 5.5l-1.4 1.4M6.9 17.1l-1.4 1.4" />
    </>
  ),
  eye: (
    <>
      <path d="M2.8 12s3.4-5.6 9.2-5.6 9.2 5.6 9.2 5.6-3.4 5.6-9.2 5.6S2.8 12 2.8 12Z" />
      <circle cx="12" cy="12" r="2.5" />
    </>
  ),
  folder: <path d="M3 7.2a2 2 0 0 1 2-2h3.9l1.8 2H19a2 2 0 0 1 2 2v8.6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
  doc: (
    <>
      <path d="M6 2.9h7.2l4.8 4.8V21H6z" />
      <path d="M13.2 2.9V7.7H18M9 12h6M9 15.5h6" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  store: <path d="M4 9.5V20h16V9.5M3 9.5l2-5h14l2 5M3 9.5a2.4 2.4 0 0 0 4.5 0 2.4 2.4 0 0 0 4.5 0 2.4 2.4 0 0 0 4.5 0 2.4 2.4 0 0 0 4.5 0M9.5 20v-5.5h5V20" />,
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4v-.2a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.6-1H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.6v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z" />
    </>
  ),
};

export function Icon({
  name,
  size = 16,
  className = '',
}: {
  name: FinalIconName;
  size?: number;
  className?: string;
}) {
  return (
    <svg
      className={`fw-svg ${className}`.trim()}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {glyphs[name]}
    </svg>
  );
}

export function WeatherIcon({ kind, size = 18, className = '' }: { kind: WeatherKind; size?: number; className?: string }) {
  return (
    <svg
      className={`fw-svg ${className}`.trim()}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {kind === 'sun' && (
        <>
          <circle cx="12" cy="12" r="4" fill="currentColor" stroke="none" />
          <path d="M12 2.8v2M12 19.2v2M2.8 12h2M19.2 12h2M5.5 5.5l1.4 1.4M17.1 17.1l1.4 1.4M18.5 5.5l-1.4 1.4M6.9 17.1l-1.4 1.4" />
        </>
      )}
      {kind === 'cloud' && <path d="M7.2 18.5h9.3a3.7 3.7 0 0 0 0-7.4h-.7A5.2 5.2 0 0 0 6 11.7a3.5 3.5 0 0 0 1.2 6.8Z" />}
      {kind === 'rain' && (
        <>
          <path d="M7.2 14.8h9.3a3.6 3.6 0 0 0 0-7.2h-.7A5.1 5.1 0 0 0 6 8.1a3.4 3.4 0 0 0 1.2 6.7Z" />
          <path d="m8.2 18-1 2.5M12.2 18l-1 2.5M16.2 18l-1 2.5" />
        </>
      )}
      {kind === 'partly' && (
        <>
          <circle cx="8.2" cy="8" r="2.8" fill="currentColor" stroke="none" />
          <path d="M8.2 3v1.2M3.2 8H2M4.8 4.6 4 3.8M11.6 4.6l.8-.8" />
          <path d="M9.5 13.5h6.3a3 3 0 0 1 0 6H9.5a3 3 0 0 1 0-6Z" />
        </>
      )}
    </svg>
  );
}

export function weatherKind(code: number | null | undefined): WeatherKind {
  if (code == null) return 'partly';
  if (code === 0 || code === 1) return 'sun';
  if (code === 2) return 'partly';
  if (code >= 51 && code <= 82) return 'rain';
  if (code >= 85 && code <= 86) return 'cloud';
  if (code >= 95) return 'rain';
  return 'cloud';
}

export function weatherLabel(code: number | null | undefined): string {
  if (code == null) return 'Conditions unavailable';
  if (code === 0) return 'Clear';
  if (code === 1) return 'Mainly clear';
  if (code === 2) return 'Partly cloudy';
  if (code === 3) return 'Overcast';
  if (code === 45 || code === 48) return 'Fog';
  if (code >= 51 && code <= 57) return 'Drizzle';
  if (code >= 61 && code <= 67) return 'Rain';
  if (code >= 71 && code <= 77) return 'Snow';
  if (code >= 80 && code <= 82) return 'Rain showers';
  if (code >= 85 && code <= 86) return 'Snow showers';
  if (code >= 95) return 'Thunderstorm';
  return 'Conditions unavailable';
}

export function appGlyphFor(title: string): AppGlyphName {
  const value = title.toLowerCase();
  if (value.includes('code') || value.includes('cursor')) return 'code';
  if (value.includes('terminal') || value.includes('powershell') || value.includes('cmd')) return 'terminal';
  if (value.includes('slack')) return 'slack';
  if (value.includes('figma')) return 'figma';
  if (value.includes('music') || value.includes('spotify')) return 'music';
  if (value.includes('notes') || value.includes('notion')) return 'notes';
  if (value.includes('file') || value.includes('explorer')) return 'folders';
  return 'browser';
}

export function AppIcon({ kind, size = 18 }: { kind: AppGlyphName; size?: number }) {
  if (kind === 'code') {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path d="m18.4 2.6-9.7 8.2L3.6 7.4 1.5 8.7l4.8 3.9-4.8 3.9 2.1 1.3 5.1-3.4 9.7 8.2 4.4-2V4.6z" fill="#1f9cf0" />
        <path d="m18.4 2.6-7.8 7.3 7.8 7.3 4.4-2.2V4.6z" fill="#39a9f4" />
      </svg>
    );
  }
  if (kind === 'terminal') {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <rect x="2" y="3" width="20" height="18" rx="4" fill="#15181d" />
        <path d="m6.5 8 4 4-4 4M12.5 16H18" stroke="#6ee7b7" strokeWidth="1.8" fill="none" strokeLinecap="round" />
      </svg>
    );
  }
  if (kind === 'slack') {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <rect x="9.5" y="2" width="3.4" height="9" rx="1.7" fill="#36c5f0" />
        <rect x="13" y="9.5" width="9" height="3.4" rx="1.7" fill="#2eb67d" />
        <rect x="11" y="13" width="3.4" height="9" rx="1.7" fill="#ecb22e" />
        <rect x="2" y="11" width="9" height="3.4" rx="1.7" fill="#e01e5a" />
      </svg>
    );
  }
  if (kind === 'figma') {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path d="M8 2h4v7H8a3.5 3.5 0 0 1 0-7Z" fill="#f24e1e" />
        <path d="M12 2h4a3.5 3.5 0 0 1 0 7h-4z" fill="#ff7262" />
        <path d="M8 9h4v7H8a3.5 3.5 0 0 1 0-7Z" fill="#a259ff" />
        <circle cx="15.5" cy="12.5" r="3.5" fill="#1abcfe" />
        <path d="M8 16h4v3.5A3.5 3.5 0 1 1 8 16Z" fill="#0acf83" />
      </svg>
    );
  }
  if (kind === 'folders') {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path d="M3 6.5a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="#e6b566" />
        <path d="M3 10h18" stroke="#b68636" strokeWidth="1.4" />
      </svg>
    );
  }
  if (kind === 'music') {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <circle cx="12" cy="12" r="10" fill="#1ed760" />
        <path d="M7.5 9.4c3.2-1 6.8-.5 9.4 1.2M8 12.4c2.6-.7 5.4-.4 7.6 1M8.5 15.2c2-.5 4.1-.2 5.8.8" stroke="#0b1a11" strokeWidth="1.6" fill="none" strokeLinecap="round" />
      </svg>
    );
  }
  if (kind === 'notes') {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <rect x="4" y="3" width="16" height="18" rx="3" fill="#d6b46a" />
        <path d="M8 8h8M8 12h8M8 16h5" stroke="#4b3b1c" strokeWidth="1.5" strokeLinecap="round" />
      </svg>
    );
  }
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <circle cx="12" cy="12" r="10" fill="#e35d3c" />
      <circle cx="12" cy="12" r="6.2" fill="#f8f4ee" />
      <circle cx="12" cy="12" r="2.4" fill="#e35d3c" />
    </svg>
  );
}
