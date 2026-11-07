import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { isNative, native } from '../../services/native';
import { WidgetManager } from '../settings/WidgetManager';
import DockSurface from './DockSurface';
import { defaultLayout, layoutFingerprint, parseLayout, type DockLayout } from './layout';

const STORAGE_KEY = 'protondock-layout';

export interface DockShellProps {
  settingsOnly?: boolean;
}

function readLocal(): unknown {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw == null ? null : JSON.parse(raw);
  } catch {
    return undefined;
  }
}

async function readSaved(): Promise<{ value: unknown; present: boolean }> {
  if (isNative()) {
    const value = await native.loadSettings();
    return { value, present: value != null };
  }
  const value = readLocal();
  return { value, present: value !== null };
}

async function writeSaved(layout: DockLayout): Promise<void> {
  if (isNative()) {
    await native.saveSettings(layout);
    return;
  }
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
}

export function DockShell({ settingsOnly = false }: DockShellProps) {
  const [layout, setLayout] = useState<DockLayout | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saveBlocked, setSaveBlocked] = useState(false);
  const lastSaved = useRef<string>('');
  const loaded = useRef(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const { value, present } = await readSaved();
        if (cancelled) return;
        if (!present) {
          const initial = defaultLayout();
          setLayout(initial);
          lastSaved.current = '';
          loaded.current = true;
          return;
        }
        const parsed = parseLayout(value);
        if (!parsed) {
          setLayout(defaultLayout());
          setError('Saved dock settings could not be read. Using defaults without overwriting your saved layout.');
          setSaveBlocked(true);
          loaded.current = true;
          return;
        }
        setLayout(parsed);
        lastSaved.current = layoutFingerprint(parsed);
        loaded.current = true;
      } catch (cause) {
        if (cancelled) return;
        setLayout(defaultLayout());
        setError(`Could not load dock settings: ${String(cause)}`);
        setSaveBlocked(true);
        loaded.current = true;
      }
    })();
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    if (!layout || !loaded.current || saveBlocked) return;
    const serialized = layoutFingerprint(layout);
    if (serialized === lastSaved.current) return;
    const timer = window.setTimeout(() => {
      void writeSaved(layout).then(() => {
        lastSaved.current = serialized;
        setError(null);
      }).catch((cause) => {
        setError(`Could not save dock settings: ${String(cause)}`);
      });
    }, 350);
    return () => window.clearTimeout(timer);
  }, [layout, saveBlocked]);

  useEffect(() => {
    if (!isNative()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen('settings-updated', () => {
      void native.loadSettings().then((value) => {
        if (disposed) return;
        const parsed = parseLayout(value);
        if (parsed) {
          lastSaved.current = layoutFingerprint(parsed);
          setLayout(parsed);
          setError(null);
        }
      }).catch(() => {});
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => { disposed = true; unlisten?.(); };
  }, []);

  useEffect(() => {
    document.body.dataset.surface = settingsOnly ? 'settings' : 'dock';
    return () => { delete document.body.dataset.surface; };
  }, [settingsOnly]);

  const customize = useCallback(() => {
    if (isNative()) {
      void native.openSettings().catch(() => {});
      return;
    }
    const params = new URLSearchParams({ window: 'settings', preview: '1' });
    window.location.assign(`?${params.toString()}`);
  }, []);

  const closeSettings = useCallback(() => {
    if (isNative()) {
      void native.hideSettings().catch(() => {});
      return;
    }
    const params = new URLSearchParams({ window: 'dock', preview: '1' });
    window.location.assign(`?${params.toString()}`);
  }, []);

  if (!layout) {
    return <div className="dock-final-loading" aria-busy="true" />;
  }

  if (settingsOnly) {
    return (
      <WidgetManager
        layout={layout}
        onChange={setLayout}
        onClose={closeSettings}
        error={saveBlocked ? error : null}
      />
    );
  }

  return (
    <>
      <DockSurface layout={layout} onChange={setLayout} onCustomize={customize} />
      {error ? <div className="native-error dock-final-error" role="alert">{error}</div> : null}
    </>
  );
}

export default DockShell;
