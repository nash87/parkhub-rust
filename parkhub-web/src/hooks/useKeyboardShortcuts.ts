import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

export function useKeyboardShortcuts({
  onToggleCommandPalette,
  onToggleShortcutsHelp,
  onToggleAssistant,
}: {
  onToggleCommandPalette: () => void;
  /**
   * Optional. Ctrl/Cmd + / toggles the keyboard-shortcuts cheat-sheet.
   * Kept optional so Layout can opt-in without forcing older callers to pass it.
   */
  onToggleShortcutsHelp?: () => void;
  /**
   * Optional. Ctrl/Cmd + . toggles the Assistant side panel (v3 design).
   */
  onToggleAssistant?: () => void;
}) {
  const navigate = useNavigate();

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;

      // Ctrl/Cmd+B → navigate to /book
      if (mod && e.key === 'b') {
        e.preventDefault();
        navigate('/book');
        return;
      }

      // Ctrl/Cmd+K → toggle command palette
      if (mod && e.key === 'k') {
        e.preventDefault();
        onToggleCommandPalette();
        return;
      }

      // Ctrl/Cmd+/ → toggle shortcuts cheat-sheet (v3 design addition).
      if (mod && e.key === '/' && onToggleShortcutsHelp) {
        e.preventDefault();
        onToggleShortcutsHelp();
        return;
      }

      // Ctrl/Cmd+. → toggle Assistant side panel (v3 design addition).
      if (mod && e.key === '.' && onToggleAssistant) {
        e.preventDefault();
        onToggleAssistant();
        return;
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [navigate, onToggleCommandPalette, onToggleShortcutsHelp, onToggleAssistant]);
}
