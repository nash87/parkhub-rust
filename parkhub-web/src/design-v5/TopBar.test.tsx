import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { V5ThemeProvider } from './ThemeProvider';
import { V5TopBar } from './TopBar';

function renderTopBar(overrides: Partial<{ assistantOpen: boolean }> = {}) {
  return render(
    <V5ThemeProvider>
      <V5TopBar
        title="Dashboard"
        breadcrumb="HOME"
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
});
