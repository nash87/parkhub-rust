import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

// ── Mocks ──

const mockNavigate = vi.fn();

vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'setup.title': 'Setup Wizard',
        'setup.step1': 'Company Info',
        'setup.step2': 'Create Lot',
        'setup.step3': 'User Setup',
        'setup.step4': 'Choose Theme',
        'setup.companyName': 'Company Name',
        'setup.companyNameRequired': 'Company name is required',
        'setup.timezone': 'Timezone',
        'setup.lotName': 'Lot Name',
        'setup.lotNameRequired': 'Lot name is required',
        'setup.floors': 'Floors',
        'setup.slotsPerFloor': 'Slots per Floor',
        'setup.inviteUsers': 'Invite Users (comma-separated emails)',
        'setup.inviteDesc': 'Your admin account is already set up.',
        'setup.chooseTheme': 'Choose Theme',
        'setup.complete': 'Complete Setup',
        'common.back': 'Back',
        'common.next': 'Next',
      };
      return map[key] || fallback || key;
    },
  }),
}));

import { SetupWizardPage } from './SetupWizard';

describe('SetupWizardPage', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    // Default: wizard not completed
    global.fetch = vi.fn()
      .mockResolvedValueOnce({
        json: () => Promise.resolve({ success: true, data: { completed: false, steps: [] } }),
      })
      .mockResolvedValue({
        json: () => Promise.resolve({ success: true, data: { step: 1, message: 'OK' } }),
      });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders wizard title and progress bar', async () => {
    render(<SetupWizardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('setup-wizard')).toBeInTheDocument();
    });
    expect(screen.getByText('Setup Wizard')).toBeInTheDocument();
    expect(screen.getByTestId('wizard-progress')).toBeInTheDocument();
  });

  it('shows step 1 (Company Info) by default', async () => {
    render(<SetupWizardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-1')).toBeInTheDocument();
    });
    expect(screen.getByTestId('input-company-name')).toBeInTheDocument();
    expect(screen.getByTestId('select-timezone')).toBeInTheDocument();
  });

  it('shows validation error when company name is empty', async () => {
    render(<SetupWizardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('wizard-next')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-error')).toBeInTheDocument();
    });
    expect(screen.getByText('Company name is required')).toBeInTheDocument();
  });

  it('advances to step 2 after valid step 1', async () => {
    render(<SetupWizardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('input-company-name')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-2')).toBeInTheDocument();
    });
    expect(screen.getByTestId('input-lot-name')).toBeInTheDocument();
    expect(screen.getByTestId('input-floors')).toBeInTheDocument();
  });

  it('shows 16 theme cards in step 4', async () => {
    // Mock all fetch calls to succeed
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true, data: { completed: false, steps: [] } }),
    });

    render(<SetupWizardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('setup-wizard')).toBeInTheDocument();
    });

    // Fill step 1 and advance
    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'Test' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-2')).toBeInTheDocument();
    });

    // Fill step 2 and advance
    fireEvent.change(screen.getByTestId('input-lot-name'), { target: { value: 'Garage' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-3')).toBeInTheDocument();
    });

    // Step 3 — skip (optional)
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-4')).toBeInTheDocument();
    });

    const themeGrid = screen.getByTestId('theme-grid');
    expect(themeGrid.children.length).toBe(16);
    expect(screen.getByTestId('theme-classic')).toBeInTheDocument();
    expect(screen.getByTestId('theme-neon')).toBeInTheDocument();
    expect(screen.getByTestId('theme-zen')).toBeInTheDocument();
  });

  it('redirects to home if wizard already completed', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      json: () => Promise.resolve({ success: true, data: { completed: true, steps: [] } }),
    });

    render(<SetupWizardPage />);

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/', { replace: true });
    });
  });
});
