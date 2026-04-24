import { describe, expect, it, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { V5SettingsProvider } from '../settings/SettingsProvider';
import { V5ThemeProvider } from '../ThemeProvider';
import { V5Sidebar } from './index';
import { DEFAULT_SETTINGS } from '../settings/settings';

function withProviders(ui: React.ReactNode, sidebar: 'marble' | 'columns' | 'minimal') {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return (
    <QueryClientProvider client={qc}>
      <V5ThemeProvider>
        <V5SettingsProvider
          initialOverride={{
            ...DEFAULT_SETTINGS,
            appearance: { ...DEFAULT_SETTINGS.appearance, sidebar },
          }}
        >
          {ui}
        </V5SettingsProvider>
      </V5ThemeProvider>
    </QueryClientProvider>
  );
}

describe('<V5Sidebar variant=...>', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('renders MarbleSidebar by default', () => {
    render(
      withProviders(<V5Sidebar active="dashboard" onNavigate={() => {}} />, 'marble'),
    );
    const nav = screen.getByLabelText('Hauptnavigation');
    expect(nav.getAttribute('data-variant')).toBe('marble');
  });

  it('renders MinimalSidebar when sidebar=minimal', () => {
    render(
      withProviders(<V5Sidebar active="dashboard" onNavigate={() => {}} />, 'minimal'),
    );
    const nav = screen.getByLabelText('Hauptnavigation');
    expect(nav.getAttribute('data-variant')).toBe('minimal');
    // Minimal renders 26 nav buttons via aria-label
    const buttons = screen.getAllByRole('button', { name: /Dashboard|Buchungen|Kalender/ });
    expect(buttons.length).toBeGreaterThan(0);
  });

  it('renders ColumnsSidebar when sidebar=columns', () => {
    render(
      withProviders(<V5Sidebar active="dashboard" onNavigate={() => {}} />, 'columns'),
    );
    const nav = screen.getByLabelText('Hauptnavigation');
    expect(nav.getAttribute('data-variant')).toBe('columns');
    // Columns renders the section heading "Persönlich"
    expect(screen.getByText('Persönlich')).toBeInTheDocument();
  });

  it('falls back to MarbleSidebar when no provider is mounted', () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <V5ThemeProvider>
          <V5Sidebar active="dashboard" onNavigate={() => {}} />
        </V5ThemeProvider>
      </QueryClientProvider>,
    );
    const nav = screen.getByLabelText('Hauptnavigation');
    expect(nav.getAttribute('data-variant')).toBe('marble');
  });

  it('MinimalSidebar exposes a compact 52px width and visible icons', () => {
    render(
      withProviders(<V5Sidebar active="dashboard" onNavigate={() => {}} />, 'minimal'),
    );
    const nav = screen.getByLabelText('Hauptnavigation');
    const width = (nav as HTMLElement).style.width;
    expect(width).toBe('52px');
  });
});
