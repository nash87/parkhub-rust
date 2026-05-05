import { describe, it, expect, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { V5ThemeProvider } from './ThemeProvider';
import { V5TopBar } from './TopBar';

function renderTopBar(overrides: Partial<{ assistantOpen: boolean; onOpenNavigation: () => void }> = {}) {
  return render(
    <V5ThemeProvider>
      <V5TopBar
        title="Dashboard"
        breadcrumb="HOME"
        onOpenNavigation={overrides.onOpenNavigation}
        onOpenCommand={vi.fn()}
        onToggleAssistant={vi.fn()}
        assistantOpen={overrides.assistantOpen ?? false}
      />
    </V5ThemeProvider>,
  );
}

describe('V5TopBar — assistant toggle branding', () => {
  it('uses Assistent aria-label and visible label without AI/KI text', () => {
    renderTopBar();
    expect(
      screen.getByRole('button', { name: 'Assistent umschalten' }),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText(/KI-Assistent/)).toBeNull();
    expect(screen.queryByText(/^AI$/)).toBeNull();
    expect(screen.queryByText(/^KI$/)).toBeNull();
  });

  it('exposes a stable visible screen title for mobile smoke tests', () => {
    renderTopBar();

    expect(screen.getByTestId('v5-screen-title')).toHaveTextContent('Dashboard');
  });

  it('calls the mobile navigation opener from the compact nav trigger', () => {
    const onOpenNavigation = vi.fn();
    renderTopBar({ onOpenNavigation });

    fireEvent.click(screen.getByRole('button', { name: 'Navigation öffnen' }));

    expect(onOpenNavigation).toHaveBeenCalledTimes(1);
  });
});
