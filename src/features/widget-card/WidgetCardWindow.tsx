import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { isNative, native, type WidgetCardState } from '../../services/native';
import { performWidgetAction } from '../../components/dock/actions';
import { WidgetCard } from '../../widgets/final/WidgetCard';
import { getWidgetMeta, isFinalWidgetId } from '../../widgets/final/meta';
import { useFinalWidgetData } from '../../widgets/final/useFinalWidgetData';
import type { FinalWidgetSettings } from '../../widgets/final/types';

function previewMode(): boolean {
  return !isNative() && new URLSearchParams(window.location.search).get('preview') === '1';
}

function fromQuery(): WidgetCardState | null {
  const params = new URLSearchParams(window.location.search);
  const id = params.get('id') ?? '';
  const size = params.get('size') ?? 'standard';
  if (!isFinalWidgetId(id)) return null;
  if (size !== 'compact' && size !== 'standard' && size !== 'wide') return null;
  return { id, size };
}

export function WidgetCardWindow() {
  const [card, setCard] = useState<WidgetCardState | null>(null);
  const preview = previewMode();
  const settings = card?.settings as FinalWidgetSettings | undefined;
  const views = useFinalWidgetData(card ? { [card.id]: settings } : {}, { preview });
  const view = card ? views[card.id] : undefined;

  useEffect(() => {
    document.body.dataset.surface = 'widget';
    return () => { delete document.body.dataset.surface; };
  }, []);

  useEffect(() => {
    if (!isNative()) {
      setCard(fromQuery());
      return;
    }
    let disposed = false;
    void native.widgetCardState().then((value) => {
      if (!disposed && value && isFinalWidgetId(value.id)) setCard(value);
    }).catch(() => {});
    let unlisten: (() => void) | undefined;
    void listen<WidgetCardState>('widget-card:show', (event) => {
      if (event.payload && isFinalWidgetId(event.payload.id)) setCard(event.payload);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') void native.hideWidgetCard().catch(() => {});
    };
    window.addEventListener('keydown', onKey);
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener('keydown', onKey);
    };
  }, []);

  const close = () => {
    if (isNative()) {
      void native.hideWidgetCard().catch(() => {});
    } else {
      const params = new URLSearchParams({ window: 'dock', preview: '1' });
      window.location.assign(`?${params.toString()}`);
    }
  };

  if (!card) return <div className="wc-window wc-window--empty" aria-busy="true" />;

  return (
    <div className="wc-window">
      <button type="button" className="wc-close" aria-label={`Close ${getWidgetMeta(card.id).title} card`} onClick={close}>
        <span aria-hidden="true">×</span>
      </button>
      <div className="wc-card">
        <WidgetCard
          id={card.id}
          settings={settings}
          data={view?.data}
          preview={preview}
          status={view?.status}
          onAction={(action) => { void performWidgetAction(action).catch(() => {}); }}
        />
      </div>
    </div>
  );
}

export default WidgetCardWindow;
