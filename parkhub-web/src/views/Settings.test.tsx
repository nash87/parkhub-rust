import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';

const mockSetTheme = vi.fn();
const mockSetDesignTheme = vi.fn();
const mockSetNavLayout = vi.fn();
const mockSetDensity = vi.fn();

vi.mock('../context/ThemeContext', () => ({
  useTheme: () => ({
    designTheme: 'aurora',
    setDesignTheme: mockSetDesignTheme,
    setTheme: mockSetTheme,
    resolved: 'light',
    designThemes: [
      { id: 'aurora', name: 'Aurora', previewColors: { light: ['#fff', '#eee', '#22c55e', '#0ea5e9'] } },
      { id: 'metro', name: 'Metro', previewColors: { light: ['#fff', '#eee', '#f97316', '#ef4444'] } },
    ],
  }),
}));

vi.mock('../hooks/useNavLayout', () => ({
  useNavLayout: () => ['focus', mockSetNavLayout],
}));

vi.mock('../hooks/useDensity', () => ({
  useDensity: () => ['cozy', mockSetDensity],
}));

vi.mock('../components/ui/SettingsPrimitives', () => ({
  SCard: ({ title, subtitle, children }: any) => (
    <section>
      <h2>{title}</h2>
      <p>{subtitle}</p>
      {children}
    </section>
  ),
  SRow: ({ title, description, children }: any) => (
    <div>
      <h3>{title}</h3>
      <p>{description}</p>
      {children}
    </div>
  ),
  SSeg: ({ options, onChange }: any) => (
    <div>
      {options.map((option: any) => (
        <button key={option.value} type="button" onClick={() => onChange(option.value)}>
          {option.label}
        </button>
      ))}
    </div>
  ),
  SToggle: ({ value, onChange, label }: any) => (
    <button type="button" aria-pressed={value} onClick={() => onChange(!value)}>
      {label}
    </button>
  ),
  ThemeSwatches: ({ options, onChange }: any) => (
    <div>
      {options.map((option: any) => (
        <button key={option.value} type="button" onClick={() => onChange(option.value)}>
          {option.label}
        </button>
      ))}
    </div>
  ),
  NavLayoutGrid: ({ onChange }: any) => (
    <button type="button" onClick={() => onChange('classic')}>
      Choose classic
    </button>
  ),
}));

import { SettingsPage } from './Settings';

describe('SettingsPage', () => {
  beforeEach(() => {
    mockSetTheme.mockReset();
    mockSetDesignTheme.mockReset();
    mockSetNavLayout.mockReset();
    mockSetDensity.mockReset();
  });

  it('renders the settings hub and personal links by default', () => {
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );

    expect(screen.getByText('Settings')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Profile/i })).toHaveAttribute('href', '/profile');
    expect(screen.getByRole('link', { name: /Notifications/i })).toHaveAttribute('href', '/notifications');
    expect(screen.getByRole('link', { name: /Vehicles/i })).toHaveAttribute('href', '/vehicles');
  });

  it('switches to workspace links and forwards appearance changes', async () => {
    const user = userEvent.setup();

    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );

    await user.click(screen.getByRole('tab', { name: /Workspace/i }));
    expect(screen.getByRole('link', { name: /Organization/i })).toHaveAttribute('href', '/admin/settings');
    expect(screen.getByRole('link', { name: /SSO & roles/i })).toHaveAttribute('href', '/admin/sso');

    await user.click(screen.getByRole('button', { name: 'Metro' }));
    await user.click(screen.getAllByRole('button', { name: 'Dark mode' })[0]);
    await user.click(screen.getByRole('button', { name: 'Compact' }));
    await user.click(screen.getByRole('button', { name: 'Choose classic' }));

    expect(mockSetDesignTheme).toHaveBeenCalledWith('metro');
    expect(mockSetTheme).toHaveBeenCalledWith('dark');
    expect(mockSetDensity).toHaveBeenCalledWith('compact');
    expect(mockSetNavLayout).toHaveBeenCalledWith('classic');
  });
});
