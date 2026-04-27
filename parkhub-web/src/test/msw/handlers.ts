// SPDX-License-Identifier: MIT OR Apache-2.0
//
// MSW (Mock Service Worker) request handlers for vitest unit tests.
//
// Goal: keep unit tests deterministic and offline. Anything that touches the
// parkhub-server REST surface (`/api/v1/...`) gets intercepted here. E2E
// tests still hit the live stack via Playwright — MSW is unit-test scope only.
//
// Add a handler here whenever a test needs to lock down a network call.
// Per-test overrides go through `server.use(...)` in the test file itself;
// `afterEach(server.resetHandlers)` rolls them back.
//
// TODO(parkhub-rust contracts): mirror the actual response shapes from
// parkhub-server's utoipa OpenAPI snapshot at `docs/openapi/rust.json` once
// the relevant endpoints stabilize. Current fixtures are deliberately minimal
// so tests can opt in via `server.use(...)` overrides.

import { http, HttpResponse } from 'msw';

/**
 * Minimal lot fixture — matches the loose shape expected by listing widgets.
 * Extend once the `/api/v1/lots` contract is locked into the OpenAPI snapshot.
 */
const lotsFixture = [
	{
		id: 'l1',
		name: 'Lot 1',
		capacity: 100,
		occupied: 42,
		address: '123 Demo Way',
	},
];

const bookingsFixture = [
	{
		id: 'b1',
		lot_id: 'l1',
		user_id: 'u1',
		status: 'confirmed',
		start: '2026-04-26T08:00:00Z',
		end: '2026-04-26T17:00:00Z',
	},
];

const notificationsFixture = {
	items: [],
	unread_count: 0,
};

export const handlers = [
	// Health probe — many widgets ping this; default to OK.
	http.get('*/health', () => HttpResponse.json({ status: 'ok' })),
	http.get('*/health/ready', () => HttpResponse.json({ status: 'ready' })),

	// Lots — the headline aggregator that powers the demo dashboard.
	http.get('*/api/v1/lots', () => HttpResponse.json(lotsFixture)),

	// Bookings — used by the demo home / "your bookings" widget.
	http.get('*/api/v1/bookings', () => HttpResponse.json(bookingsFixture)),

	// Notifications drawer.
	http.get('*/api/v1/notifications', () => HttpResponse.json(notificationsFixture)),

	// Observability ingestion — web-vitals POSTs land here in tests; accept
	// and discard so the sendBeacon fallback never leaks to the network.
	http.post('*/api/observability/web-vitals', () => HttpResponse.json({ ok: true })),

	// Generic v1 fallback — returns an empty payload so unmocked endpoints
	// don't accidentally hit the network during unit tests. Tests that need
	// a specific fixture override via `server.use(...)`.
	http.get('*/api/v1/:resource', ({ params }) =>
		HttpResponse.json({ resource: params.resource, items: [], mocked: true })
	),
];

/** Re-export raw fixtures so tests can assert against them without re-typing. */
export const fixtures = {
	lots: lotsFixture,
	bookings: bookingsFixture,
	notifications: notificationsFixture,
};
