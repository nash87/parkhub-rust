// SPDX-License-Identifier: MIT OR Apache-2.0
//
// MSW Node server (vitest jsdom environment).
// `setupServer` intercepts `fetch` calls inside the Node test process; the
// browser-mode service-worker (`setupWorker`) is intentionally NOT used here —
// vitest unit tests run under jsdom, not a real browser. The browser-mode
// config in `vitest.config.browser.ts` brings up real Chromium and uses MSW
// only via the same Node-side `setupServer` (vitest 4 + msw/node interop).

import { setupServer } from 'msw/node';
import { handlers } from './handlers';

/**
 * Singleton MSW server for the unit-test process.
 *
 * Lifecycle is wired in `src/test/msw/vitest-setup.ts`:
 *   beforeAll  -> server.listen({ onUnhandledRequest: 'warn' })
 *   afterEach  -> server.resetHandlers()
 *   afterAll   -> server.close()
 */
export const server = setupServer(...handlers);
