// Command registry — a plugin-native surface any module can contribute
// actions into. The Command Palette (Cmd+K) reads this registry and
// renders whatever is currently live. When a component unmounts it
// unregisters whatever it contributed, so the palette auto-prunes
// stale commands without the consumer needing to think about it.
//
// The registry is deliberately framework-free: plain TS types + a tiny
// subscription API. The React layer on top (CommandPalette.tsx) is the
// only thing that knows about React. Swap it for Solid/Vue/anything
// without touching what a module's `registerCommand(...)` call looks
// like.
//
// All state lives in memory in the browser. Nothing persisted server-
// side, nothing crossing the network. Search is a local fuzzy match.

import type React from 'react';

export type CommandGroup = 'navigation' | 'action' | 'data' | 'module' | 'admin';

export interface CommandContext {
  user: unknown;
  isAdmin: boolean;
  navigate: (path: string) => void;
}

export interface Command {
  /** Stable identifier — used to override keybindings and de-dupe
   *  duplicate registrations from hot-reloaded modules. */
  id: string;
  title: string;
  description?: string;
  /** Extra tokens that match against the search input. */
  keywords?: string[];
  group: CommandGroup;
  /** Visible only when this predicate returns true (auth, module active, etc.). */
  when?: (ctx: CommandContext) => boolean;
  /** What happens on Enter. Can be sync or async. */
  perform: (ctx: CommandContext) => void | Promise<void>;
  /** Optional small icon (Phosphor component or similar). Kept as
   *  ComponentType so consumers can pass anything renderable. */
  icon?: React.ComponentType<{ size?: number }>;
  /** Shortcut hint to show in the row. E.g. "G D" for "go to dashboard",
   *  "⌘K" for a reserved shortcut. Purely informational — wiring a real
   *  keybinding is the host's job. */
  shortcut?: string;
}

type Listener = () => void;

interface Registry {
  register(cmd: Command): () => void;
  registerMany(cmds: Command[]): () => void;
  all(): Command[];
  subscribe(listener: Listener): () => void;
  search(query: string, ctx: CommandContext): Command[];
  clear(): void;
}

/** Tiny bigram-friendly fuzzy score. Higher = better.
 *  Favors prefix matches on the title, then keyword hits, then
 *  substring matches. Cheap to run on a few hundred entries. */
function score(cmd: Command, query: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 1; // no query → everything passes with neutral score
  const title = cmd.title.toLowerCase();
  if (title.startsWith(q)) return 100;
  if (title.includes(q)) return 70;
  if (cmd.description?.toLowerCase().includes(q)) return 40;
  for (const kw of cmd.keywords ?? []) {
    if (kw.toLowerCase().includes(q)) return 30;
  }
  // Last resort: character run — every character of q appears in title
  // in order, with gaps allowed. Covers "bking" → "bookings".
  let ti = 0;
  for (const ch of q) {
    const idx = title.indexOf(ch, ti);
    if (idx < 0) return 0;
    ti = idx + 1;
  }
  return 10;
}

function createRegistry(): Registry {
  // Using Map keyed by id gives free dedupe on re-register (e.g. hot reload).
  const commands = new Map<string, Command>();
  const listeners = new Set<Listener>();

  const notify = () => {
    for (const l of listeners) l();
  };

  return {
    register(cmd) {
      commands.set(cmd.id, cmd);
      notify();
      return () => {
        if (commands.get(cmd.id) === cmd) {
          commands.delete(cmd.id);
          notify();
        }
      };
    },
    registerMany(cmds) {
      for (const c of cmds) commands.set(c.id, c);
      notify();
      return () => {
        for (const c of cmds) {
          if (commands.get(c.id) === c) commands.delete(c.id);
        }
        notify();
      };
    },
    all() {
      return [...commands.values()];
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    search(query, ctx) {
      return [...commands.values()]
        .filter((c) => (c.when ? c.when(ctx) : true))
        .map((c) => ({ c, s: score(c, query) }))
        .filter((x) => x.s > 0)
        .sort((a, b) => b.s - a.s || a.c.title.localeCompare(b.c.title))
        .map((x) => x.c);
    },
    clear() {
      commands.clear();
      notify();
    },
  };
}

/** Module-scoped singleton. Tests should instantiate their own via
 *  createRegistry() rather than importing this one. */
export const commandRegistry = createRegistry();

export { createRegistry };
