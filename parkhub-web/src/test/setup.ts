import '@testing-library/jest-dom/vitest';
import '../i18n';

// jsdom does not implement matchMedia — stub it on both `window` and the
// bare global so libraries that call `matchMedia(...)` directly (e.g. uPlot)
// do not explode at module load.
const matchMediaStub = vi.fn().mockImplementation((query: string) => ({
  matches: false,
  media: query,
  onchange: null,
  addListener: vi.fn(),
  removeListener: vi.fn(),
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
  dispatchEvent: vi.fn(),
}));

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: matchMediaStub,
});
// Bare-global access (used by uPlot). jsdom aliases window to globalThis for
// most surfaces but not matchMedia in all versions — pin it explicitly.
(globalThis as unknown as { matchMedia: typeof matchMediaStub }).matchMedia =
  matchMediaStub;
