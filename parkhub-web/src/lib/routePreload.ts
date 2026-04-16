type Loader = () => Promise<unknown>;

const registry = new Map<string, Loader>();
const loaded = new Set<string>();

export function registerRoute(path: string, loader: Loader) {
  registry.set(path, loader);
}

export function preloadRoute(path: string) {
  const key = '/' + path.replace(/^\//, '');
  if (loaded.has(key)) return;
  const loader = registry.get(key);
  if (loader) {
    loaded.add(key);
    loader();
  }
}

export function preloadRoutesIdle(paths: string[]) {
  const schedule = globalThis.requestIdleCallback ?? ((cb: () => void) => setTimeout(cb, 200));
  paths.forEach((p, i) =>
    schedule(() => preloadRoute(p), { timeout: 3000 + i * 500 }),
  );
}
