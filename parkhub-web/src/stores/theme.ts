import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface ThemeStore {
  isDark: boolean;
  toggle: () => void;
  setDark: (dark: boolean) => void;
}

export const useTheme = create<ThemeStore>()(
  persist(
    (set) => ({
      isDark: window.matchMedia('(prefers-color-scheme: dark)').matches,
      toggle: () => set((state) => ({ isDark: !state.isDark })),
      setDark: (dark) => set({ isDark: dark }),
    }),
    { name: 'parkhub-theme' }
  )
);

// Apply theme to document
export function applyTheme(isDark: boolean) {
  if (isDark) {
    document.documentElement.classList.add('dark');
  } else {
    document.documentElement.classList.remove('dark');
  }
}
