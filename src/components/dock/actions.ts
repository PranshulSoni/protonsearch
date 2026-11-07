import { isNative, native, type WidgetActionId } from '../../services/native';
import { refreshProvider } from '../../widgets/sdk/provider';

const NATIVE_ACTIONS = new Set<string>([
  'media:toggle',
  'media:next',
  'media:previous',
  'volume:up',
  'volume:down',
  'volume:mute',
  'trash:empty',
  'settings:sound',
  'settings:network',
  'settings:bluetooth',
  'settings:notifications',
]);

function refreshFor(action: string) {
  if (action.startsWith('media:')) {
    refreshProvider('media');
    refreshProvider('volume');
  } else if (action.startsWith('volume:')) {
    refreshProvider('volume');
    refreshProvider('media');
  } else if (action === 'trash:empty') {
    refreshProvider('trash');
  } else if (action.startsWith('folder:open:')) {
    refreshProvider('known-folders');
  }
}

export async function performWidgetAction(action: string): Promise<void> {
  if (!isNative()) return;
  if (action === 'start') {
    await native.openStart();
    return;
  }
  if (action === 'search') {
    await native.openSearch();
    return;
  }
  if (NATIVE_ACTIONS.has(action) || action.startsWith('folder:open:') || action.startsWith('volume:set:')) {
    await native.widgetAction(action as WidgetActionId);
    refreshFor(action);
    return;
  }
  await native.activateRunningApp(action);
  refreshProvider('running-apps');
}
