/**
 * CommandPaletteProvider — seeds the default command set on mount and
 * keeps the registry in sync with the live `/api/v1/modules` response so
 * the palette can deep-link into every enabled feature module.
 *
 * Stays narrowly scoped: it does NOT own the open/close state of the
 * palette itself (that lives in Layout) — it just hydrates the registry.
 * Mounting it inside the authenticated subtree means module commands are
 * only registered once a user is logged in, which is the same visibility
 * contract as the palette hotkey (`Cmd+K` / `Ctrl+K` / `/`).
 */
import { useEffect, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { commandRegistry, type Command } from '../lib/commandRegistry';
import { useAuth } from '../context/AuthContext';

interface ModuleInfo {
  name: string;
  category: string;
  description: string;
  enabled: boolean;
  runtime_enabled?: boolean;
  ui_route: string | null;
}

interface ModulesEnvelope {
  modules?: Record<string, boolean>;
  module_info?: ModuleInfo[];
  data?: ModuleInfo[];
}

function defaultCommands(
  navigate: (p: string) => void,
  t: (k: string, d?: string) => string,
  onLogout: () => void,
): Command[] {
  const go = (path: string) => () => navigate(path);
  return [
    { id: 'nav.dashboard', title: t('nav.dashboard', 'Dashboard'), group: 'navigation', perform: go('/'), keywords: ['home'] },
    { id: 'nav.bookings', title: t('nav.bookings', 'Bookings'), group: 'navigation', perform: go('/bookings') },
    { id: 'nav.book', title: t('dashboard.bookSpot', 'Book a Spot'), group: 'navigation', perform: go('/book'), shortcut: 'Ctrl+B' },
    { id: 'nav.vehicles', title: t('nav.vehicles', 'Vehicles'), group: 'navigation', perform: go('/vehicles') },
    { id: 'nav.calendar', title: t('nav.calendar', 'Calendar'), group: 'navigation', perform: go('/calendar') },
    { id: 'nav.map', title: t('nav.map', 'Map'), group: 'navigation', perform: go('/map') },
    { id: 'nav.profile', title: t('nav.profile', 'Profile'), group: 'navigation', perform: go('/profile') },
    { id: 'nav.credits', title: t('nav.credits', 'Credits'), group: 'navigation', perform: go('/credits') },
    { id: 'nav.team', title: t('nav.team', 'Team'), group: 'navigation', perform: go('/team') },
    { id: 'nav.admin', title: t('nav.admin', 'Admin'), group: 'navigation', perform: go('/admin'), when: (c) => c.isAdmin },
    { id: 'nav.admin.modules', title: t('admin.modules.title', 'Modules'), group: 'admin', perform: go('/admin/modules'), when: (c) => c.isAdmin, keywords: ['features', 'plugins'] },

    { id: 'action.new-booking', title: t('command.newBooking', 'New Booking'), group: 'action', perform: go('/book'), keywords: ['reserve', 'park'] },
    { id: 'action.checkin', title: t('command.checkin', 'Check-in'), group: 'action', perform: go('/checkin'), keywords: ['qr', 'scan'] },
    { id: 'action.logout', title: t('command.logout', 'Logout'), group: 'action', perform: onLogout },
  ];
}

export function CommandPaletteProvider({ children }: { children: ReactNode }) {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user, logout } = useAuth();

  // Seed the core default commands whenever user/locale identity shifts.
  useEffect(() => {
    const unregister = commandRegistry.registerMany(
      defaultCommands((p) => navigate(p), t, () => logout()),
    );
    return unregister;
    // isAdmin is captured via the `when` predicate read at search time,
    // so we only need to resync on user id / locale changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate, t, logout, user?.id]);

  // Fetch enriched module registry and register one command per active
  // module with a `ui_route`. Backwards-compat: consume both the new
  // `module_info` envelope field and the legacy `data` array.
  useEffect(() => {
    if (!user) return;
    let alive = true;
    const controller = new AbortController();
    fetch('/api/v1/modules', { credentials: 'include', signal: controller.signal })
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
      .then((j: ModulesEnvelope) => {
        if (!alive) return;
        const modules = j.module_info ?? j.data ?? [];
        const cmds: Command[] = modules
          .filter((m) => m.ui_route && (m.runtime_enabled ?? m.enabled))
          .map((m) => ({
            id: `module.${m.name}`,
            title: t('command.moduleGoto', 'Open {{name}}').replace('{{name}}', m.name.replace(/-/g, ' ')),
            description: m.description,
            keywords: [m.name, m.category],
            group: 'module' as const,
            perform: () => navigate(m.ui_route as string),
          }));
        commandRegistry.registerMany(cmds);
      })
      .catch(() => {
        // Endpoint unavailable → module commands silently stay out. The
        // static navigation commands seeded above are still usable.
      });
    return () => {
      alive = false;
      controller.abort();
    };
  }, [user, navigate, t]);

  return <>{children}</>;
}
