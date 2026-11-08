// @vitest-environment jsdom
import { afterEach, expect, test } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import { Widget } from './Widget';
import { fixtures } from './fixtures';
afterEach(cleanup);
for (const [state, snapshot] of Object.entries(fixtures)) for (const size of ['compact','standard','wide'] as const) test(state + ' at ' + size, () => { const {container} = render(<Widget snapshot={snapshot} settings={{}} size={size} onAction={() => {}} />); expect(container.textContent).toBeTruthy(); if (state === 'live') expect(container.textContent).toContain('128 GB free'); else expect(container.textContent).not.toContain('128 GB free'); });
