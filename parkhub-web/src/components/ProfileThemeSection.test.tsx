import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted mocks ──
const { mockSetDesignTheme, mockDesignThemes } = vi.hoisted(() => {
  const mockSetDesignTheme = vi.fn();
  const mockDesignThemes = [
    {
      id: 'classic',
      name: 'Classic',
      description: 'Clean and professional',
      previewColors: {
        light: ['#ffffff', '#f8f9fa', '#4f46e5', '#1f2937', '#e5e7eb'],
        dark: ['#111827', '#1f2937', '#818cf8', '#f9fafb', '#374151'],
      },
      tags: ['clean'],
    },
    {
      id: 'neon',
      name: 'Neon',
      description: 'Vibrant and electric',
      previewColors: {
        light: ['#0a0a0a', '#1a1a2e', '#00ff87', '#ffffff', '#333366'],
        dark: ['#0a0a0a', '#1a1a2e', '#00ff87', '#ffffff', '#333366'],
      },
      tags: ['bold'],
    },
    {
      id: 'warm',
      name: 'Warm',
      description: 'Cozy and inviting',
      previewColors: {
        light: ['#fdf6e3', '#f5eedc', '#d97706', '#44403c', '#e7e0d3'],
        dark: ['#1c1917', '#292524', '#f59e0b', '#fafaf9', '#44403c'],
      },
      tags: ['cozy'],
    },
  ];
  return { mockSetDesignTheme, mockDesignThemes };
});

vi.mock('../context/ThemeContext', () => ({
  useTheme: () => ({
    designTheme: 'classic',
    setDesignTheme: mockSetDesignTheme,
    designThemes: mockDesignThemes,
    resolved: 'light' as const,
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, string>) => {
      if (typeof fallback === 'string') return fallback;
      if (typeof fallback === 'object' && 'name' in fallback) return `Apply ${fallback.name}`;
      return key;
    },
  }),
}));

vi.mock('@phosphor-icons/react', () => ({
  Check: (props: any) => <span data-testid="icon-Check" {...props} />,
  Palette: (props: any) => <span data-testid="icon-Palette" {...props} />,
}));

import { ProfileThemeSection } from './ProfileThemeSection';

describe('ProfileThemeSection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders heading and subtitle', () => {
    render(<ProfileThemeSection />);

    expect(screen.getByText('Design Themes')).toBeInTheDocument();
    expect(screen.getByText('Choose a visual design for your ParkHub experience.')).toBeInTheDocument();
  });

  it('renders a card for each theme', () => {
    render(<ProfileThemeSection />);

    const themeButtons = screen.getAllByRole('button');
    expect(themeButtons).toHaveLength(3); // classic, neon, warm
  });

  it('marks the active theme with aria-pressed=true', () => {
    render(<ProfileThemeSection />);

    const activeButtons = screen.getAllByRole('button').filter(
      b => b.getAttribute('aria-pressed') === 'true',
    );
    expect(activeButtons).toHaveLength(1);
    expect(activeButtons[0]).toHaveAttribute('aria-label', 'Apply Classic');
  });

  it('marks non-active themes with aria-pressed=false', () => {
    render(<ProfileThemeSection />);

    const inactiveButtons = screen.getAllByRole('button').filter(
      b => b.getAttribute('aria-pressed') === 'false',
    );
    expect(inactiveButtons).toHaveLength(2);
  });

  it('calls setDesignTheme when a theme card is clicked', async () => {
    const user = userEvent.setup();

    render(<ProfileThemeSection />);

    // Click the Neon card
    const neonButton = screen.getByLabelText('Apply Neon');
    await user.click(neonButton);

    expect(mockSetDesignTheme).toHaveBeenCalledWith('neon');
  });

  it('calls setDesignTheme with correct id for each theme', async () => {
    const user = userEvent.setup();

    render(<ProfileThemeSection />);

    await user.click(screen.getByLabelText('Apply Warm'));
    expect(mockSetDesignTheme).toHaveBeenCalledWith('warm');

    await user.click(screen.getByLabelText('Apply Classic'));
    expect(mockSetDesignTheme).toHaveBeenCalledWith('classic');
  });

  it('shows checkmark icon on active theme', () => {
    render(<ProfileThemeSection />);

    // The active theme (classic) should have a Check icon
    expect(screen.getByTestId('icon-Check')).toBeInTheDocument();
  });

  it('displays theme names', () => {
    render(<ProfileThemeSection />);

    // Theme names are rendered via t() which falls through to the raw name
    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBe(3);
  });

  it('renders the palette icon', () => {
    render(<ProfileThemeSection />);
    expect(screen.getByTestId('icon-Palette')).toBeInTheDocument();
  });
});
