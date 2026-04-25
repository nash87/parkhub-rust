import { useV5SettingsOptional } from '../settings/SettingsProvider';
import { ColumnsSidebar } from './ColumnsSidebar';
import { MarbleSidebar, type SidebarProps } from './MarbleSidebar';
import { MinimalSidebar } from './MinimalSidebar';

/**
 * Variant-aware <Sidebar /> — picks layout based on user setting.
 *
 *   - marble  (default) → existing column with grouped sections
 *   - columns → port of sidebar-v3.jsx with live pass + lot occupancy + what's-next
 *   - minimal → 52px icon-only rail with hover tooltips
 *
 * Outside the SettingsProvider (e.g. legacy tests), we fall back to
 * MarbleSidebar so this component is always safe to mount.
 */
export function V5Sidebar(props: SidebarProps) {
  const ctx = useV5SettingsOptional();
  const variant = ctx?.settings.appearance.sidebar ?? 'marble';

  if (variant === 'columns') return <ColumnsSidebar {...props} />;
  if (variant === 'minimal') return <MinimalSidebar {...props} />;
  return <MarbleSidebar {...props} />;
}

export { MarbleSidebar, ColumnsSidebar, MinimalSidebar };
export type { SidebarProps };
