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

  it('back button is disabled on step 1', async () => {
    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('wizard-back')).toBeInTheDocument());
    expect(screen.getByTestId('wizard-back')).toBeDisabled();
  });

  it('back button navigates to previous step', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true }),
    });

    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-company-name')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => expect(screen.getByTestId('wizard-step-2')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('wizard-back'));
    await waitFor(() => expect(screen.getByTestId('wizard-step-1')).toBeInTheDocument());
  });

  it('step 2 validates lot name is required', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true }),
    });

    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-company-name')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => expect(screen.getByTestId('wizard-step-2')).toBeInTheDocument());

    // Don't fill lot name, click next
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-error')).toBeInTheDocument();
      expect(screen.getByText('Lot name is required')).toBeInTheDocument();
    });
  });

  it('step 3 shows invite textarea and submits', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true }),
    });

    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-company-name')).toBeInTheDocument());

    // Step 1
    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));
    await waitFor(() => expect(screen.getByTestId('wizard-step-2')).toBeInTheDocument());

    // Step 2
    fireEvent.change(screen.getByTestId('input-lot-name'), { target: { value: 'Garage' } });
    fireEvent.click(screen.getByTestId('wizard-next'));
    await waitFor(() => expect(screen.getByTestId('wizard-step-3')).toBeInTheDocument());

    // Fill invite emails
    fireEvent.change(screen.getByTestId('input-invite-emails'), { target: { value: 'alice@company.com, bob@company.com' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => expect(screen.getByTestId('wizard-step-4')).toBeInTheDocument());
  });

  it('step 4 complete setup navigates to home', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true }),
    });

    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-company-name')).toBeInTheDocument());

    // Step 1
    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));
    await waitFor(() => expect(screen.getByTestId('wizard-step-2')).toBeInTheDocument());

    // Step 2
    fireEvent.change(screen.getByTestId('input-lot-name'), { target: { value: 'Garage' } });
    fireEvent.click(screen.getByTestId('wizard-next'));
    await waitFor(() => expect(screen.getByTestId('wizard-step-3')).toBeInTheDocument());

    // Step 3
    fireEvent.click(screen.getByTestId('wizard-next'));
    await waitFor(() => expect(screen.getByTestId('wizard-step-4')).toBeInTheDocument());

    // Step 4 - select theme and complete
    fireEvent.click(screen.getByTestId('theme-neon'));
    expect(screen.getByText('Complete Setup')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/', { replace: true });
    });
  });

  it('network error shows error message', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: { completed: false } }) })
      .mockRejectedValue(new Error('Network error'));

    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-company-name')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-error')).toBeInTheDocument();
      expect(screen.getByText('Network error')).toBeInTheDocument();
    });
  });

  it('API failure shows error message', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: { completed: false } }) })
      .mockResolvedValue({ json: () => Promise.resolve({ success: false, error: { message: 'Server error' } }) });

    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-company-name')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));

    await waitFor(() => {
      expect(screen.getByTestId('wizard-error')).toBeInTheDocument();
      expect(screen.getByText('Server error')).toBeInTheDocument();
    });
  });

  it('logo upload triggers file reader', async () => {
    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-logo')).toBeInTheDocument());

    const file = new File(['test'], 'logo.png', { type: 'image/png' });
    const input = screen.getByTestId('input-logo');
    fireEvent.change(input, { target: { files: [file] } });

    // FileReader should process the file
    await waitFor(() => {
      // Just verify no crash
      expect(screen.getByTestId('wizard-step-1')).toBeInTheDocument();
    });
  });

  it('step 2 floor and slots per floor controls work', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true }),
    });

    render(<SetupWizardPage />);
    await waitFor(() => expect(screen.getByTestId('input-company-name')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('input-company-name'), { target: { value: 'TestCorp' } });
    fireEvent.click(screen.getByTestId('wizard-next'));
    await waitFor(() => expect(screen.getByTestId('wizard-step-2')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('input-floors'), { target: { value: '3' } });
    fireEvent.change(screen.getByTestId('input-slots-per-floor'), { target: { value: '20' } });

    expect(screen.getByText('Total: 60 slots')).toBeInTheDocument();
  });
});
