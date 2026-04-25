/**
 * Backwards-compatibility shim — the original Sidebar.tsx implementation
 * has moved to ./sidebar/MarbleSidebar.tsx. The variant-aware export
 * <V5Sidebar /> now lives in ./sidebar/index.tsx and picks Marble /
 * Columns / Minimal based on the user's setting.
 *
 * Existing imports (`import { V5Sidebar } from './Sidebar'`) keep
 * working. New code should `import { V5Sidebar } from './sidebar'`.
 */
export { V5Sidebar, MarbleSidebar, MinimalSidebar, ColumnsSidebar } from './sidebar';
export type { SidebarProps } from './sidebar';
