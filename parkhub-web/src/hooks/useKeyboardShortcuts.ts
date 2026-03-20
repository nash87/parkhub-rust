import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

export function useKeyboardShortcuts({
  onToggleCommandPalette,
}: {
  onToggleCommandPalette: () => void;
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
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [navigate, onToggleCommandPalette]);
}
