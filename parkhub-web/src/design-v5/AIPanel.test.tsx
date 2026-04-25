import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { V5AIPanel } from './AIPanel';
import { V5ToastProvider } from './Toast';

function renderPanel(open = true) {
  return render(
    <V5ToastProvider>
      <V5AIPanel open={open} />
    </V5ToastProvider>,
  );
}

describe('V5AIPanel — privacy-first design bundle', () => {
  it('renders nothing when closed', () => {
    renderPanel(false);
    expect(screen.queryByText('KI-Assistent')).not.toBeInTheDocument();
    expect(screen.queryByText('Lokal')).not.toBeInTheDocument();
  });

  describe('header', () => {
    it('renders the Assistent title', () => {
      renderPanel();
      expect(screen.getByText('KI-Assistent')).toBeInTheDocument();
    });

    it('renders the "Lokal" privacy badge in the header', () => {
      renderPanel();
      expect(screen.getByText('Lokal')).toBeInTheDocument();
    });
  });

  describe('section eyebrows', () => {
    it('renders the "Vorschläge" eyebrow above the suggestions', () => {
      renderPanel();
      const eyebrow = screen.getByText('Vorschläge');
      expect(eyebrow).toBeInTheDocument();
      expect(eyebrow).toHaveStyle({ textTransform: 'uppercase' });
    });

    it('renders the "Statistiken" eyebrow above the stats', () => {
      renderPanel();
      const eyebrow = screen.getByText('Statistiken');
      expect(eyebrow).toBeInTheDocument();
      expect(eyebrow).toHaveStyle({ textTransform: 'uppercase' });
    });
  });

  describe('privacy footer', () => {
    it('anchors a "Keine Daten verlassen den Server" footer at the bottom', () => {
      renderPanel();
      expect(
        screen.getByText(/Keine Daten verlassen den Server/i),
      ).toBeInTheDocument();
    });

    it('footer is rendered with uppercase styling', () => {
      renderPanel();
      const footer = screen.getByText(/Keine Daten verlassen den Server/i);
      expect(footer).toHaveStyle({ textTransform: 'uppercase' });
    });
  });
});
