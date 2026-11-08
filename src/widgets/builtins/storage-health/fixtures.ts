import type { Snapshot } from '../../sdk/types';
import type { Data } from './types';
export const fixtures: Record<string, Snapshot<Data>> = { live: { status: 'live', data: { totalBytes: 512 * 1073741824, freeBytes: 128 * 1073741824, mountPoint: 'C:' }, updatedAt: 1 }, ...Object.fromEntries(['loading','empty','stale','disconnected','permission-denied','error'].map(status => [status, { status, updatedAt: 0 }])) } as Record<string, Snapshot<Data>>;
