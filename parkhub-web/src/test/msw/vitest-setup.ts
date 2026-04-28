// SPDX-License-Identifier: MIT OR Apache-2.0
//
// MSW lifecycle bridge for vitest. Add to `vitest.config.ts` via
// `test.setupFiles` (alongside the existing `./src/test/setup.ts`). Keep this
// file free of test logic — it only owns the listen/reset/close transitions
// for the shared MSW server.

import { afterAll, afterEach, beforeAll } from 'vitest';
import { server } from './setup-server';

beforeAll(() => {
	// `warn` (vs. `error`) avoids exploding tests that legitimately exercise
	// real fetch in the future. Tighten to `error` once the fixture set is
	// considered exhaustive.
	server.listen({ onUnhandledRequest: 'warn' });
});

afterEach(() => {
	server.resetHandlers();
});

afterAll(() => {
	server.close();
});
