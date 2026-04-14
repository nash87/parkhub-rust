import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted API mocks ──
const { mockGet2FAStatus, mockSetup2FA, mockVerify2FA, mockDisable2FA } = vi.hoisted(() => ({
  mockGet2FAStatus: vi.fn(),
  mockSetup2FA: vi.fn(),
  mockVerify2FA: vi.fn(),
  mockDisable2FA: vi.fn(),
}));

vi.mock('../api/client', () => ({
  api: {
    get2FAStatus: mockGet2FAStatus,
    setup2FA: mockSetup2FA,
    verify2FA: mockVerify2FA,
    disable2FA: mockDisable2FA,
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const { mockToast } = vi.hoisted(() => ({
  mockToast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock('react-hot-toast', () => ({
  default: mockToast,
}));

vi.mock('@phosphor-icons/react', () => ({
  ShieldCheck: (props: any) => <span data-testid="icon-ShieldCheck" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-SpinnerGap" {...props} />,
  Lock: (props: any) => <span data-testid="icon-Lock" {...props} />,
  X: (props: any) => <span data-testid="icon-X" {...props} />,
  Check: (props: any) => <span data-testid="icon-Check" {...props} />,
  Warning: (props: any) => <span data-testid="icon-Warning" {...props} />,
}));

import { TwoFactorSetupComponent } from './TwoFactorSetup';

describe('TwoFactorSetupComponent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading state initially', () => {
    mockGet2FAStatus.mockReturnValue(new Promise(() => {}));
    render(<TwoFactorSetupComponent />);
    expect(screen.getByText('Loading 2FA status...')).toBeInTheDocument();
  });

  it('renders with 2FA disabled, shows Enable button', async () => {
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: false } });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading 2FA status...')).not.toBeInTheDocument();
    });

    expect(screen.getByText('Two-Factor Authentication')).toBeInTheDocument();
    expect(screen.getByText('Add an extra layer of security')).toBeInTheDocument();
    expect(screen.getByText('Enable')).toBeInTheDocument();
    expect(screen.queryByText('Disable')).not.toBeInTheDocument();
  });

  it('renders with 2FA enabled, shows Disable button', async () => {
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: true } });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading 2FA status...')).not.toBeInTheDocument();
    });

    expect(screen.getByText('Enabled — your account is protected')).toBeInTheDocument();
    expect(screen.getByText('Disable')).toBeInTheDocument();
    expect(screen.queryByText('Enable')).not.toBeInTheDocument();
  });

  it('shows QR code setup when Enable is clicked', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: false } });
    mockSetup2FA.mockResolvedValue({
      success: true,
      data: { secret: 'JBSWY3DPEHPK3PXP', otpauth_uri: 'otpauth://totp/test', qr_code_base64: 'iVBOR...' },
    });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Enable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Enable'));

    await waitFor(() => {
      expect(screen.getByText(/Scan this QR code/)).toBeInTheDocument();
    });

    expect(screen.getByAltText('2FA QR Code')).toBeInTheDocument();
    expect(screen.getByText('JBSWY3DPEHPK3PXP')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter 6-digit code')).toBeInTheDocument();
    expect(screen.getByText('Verify')).toBeInTheDocument();
  });

  it('shows error toast when setup fails', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: false } });
    mockSetup2FA.mockResolvedValue({ success: false, error: { message: 'Server error' } });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Enable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Enable'));

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Server error');
    });
  });

  it('verifies 2FA code successfully', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: false } });
    mockSetup2FA.mockResolvedValue({
      success: true,
      data: { secret: 'JBSWY3DPEHPK3PXP', otpauth_uri: 'otpauth://totp/test', qr_code_base64: 'abc' },
    });
    mockVerify2FA.mockResolvedValue({ success: true, data: { enabled: true } });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Enable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Enable'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Enter 6-digit code')).toBeInTheDocument();
    });

    const input = screen.getByPlaceholderText('Enter 6-digit code');
    await user.type(input, '123456');
    await user.click(screen.getByText('Verify'));

    await waitFor(() => {
      expect(mockVerify2FA).toHaveBeenCalledWith('123456');
    });
    expect(mockToast.success).toHaveBeenCalledWith('Two-factor authentication enabled!');
  });

  it('rejects non-6-digit codes', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: false } });
    mockSetup2FA.mockResolvedValue({
      success: true,
      data: { secret: 'ABC', otpauth_uri: 'otpauth://totp/test', qr_code_base64: 'abc' },
    });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Enable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Enable'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Enter 6-digit code')).toBeInTheDocument();
    });

    const input = screen.getByPlaceholderText('Enter 6-digit code');
    await user.type(input, '123');

    // Verify button should be disabled with less than 6 digits
    const verifyBtn = screen.getByText('Verify').closest('button');
    expect(verifyBtn).toBeDisabled();
  });

  it('shows disable confirmation flow', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: true } });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Disable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Disable'));

    expect(screen.getByText('Disable 2FA')).toBeInTheDocument();
    expect(screen.getByText('Enter your password to confirm:')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Current password')).toBeInTheDocument();
    expect(screen.getByText('Confirm')).toBeInTheDocument();
  });

  it('disables 2FA with password', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: true } });
    mockDisable2FA.mockResolvedValue({ success: true });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Disable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Disable'));

    const pwInput = screen.getByPlaceholderText('Current password');
    await user.type(pwInput, 'mypassword');
    await user.click(screen.getByText('Confirm'));

    await waitFor(() => {
      expect(mockDisable2FA).toHaveBeenCalledWith('mypassword');
    });
    expect(mockToast.success).toHaveBeenCalledWith('Two-factor authentication disabled');
  });

  it('shows error when disable fails', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: true } });
    mockDisable2FA.mockResolvedValue({ success: false, error: { message: 'Wrong password' } });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Disable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Disable'));

    const pwInput = screen.getByPlaceholderText('Current password');
    await user.type(pwInput, 'wrong');
    await user.click(screen.getByText('Confirm'));

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Wrong password');
    });
  });

  it('cancel button hides disable form', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: true } });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Disable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Disable'));
    expect(screen.getByText('Disable 2FA')).toBeInTheDocument();

    // Click the X button (third button in the disable form)
    const xButton = screen.getByTestId('icon-X').closest('button');
    expect(xButton).toBeTruthy();
    await user.click(xButton!);

    expect(screen.queryByText('Disable 2FA')).not.toBeInTheDocument();
  });

  it('handles API failure on status check', async () => {
    mockGet2FAStatus.mockRejectedValue(new Error('Network error'));

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading 2FA status...')).not.toBeInTheDocument();
    });

    // Should render without crashing, defaulting to disabled state
    expect(screen.getByText('Two-Factor Authentication')).toBeInTheDocument();
  });

  it('code input only accepts digits', async () => {
    const user = userEvent.setup();
    mockGet2FAStatus.mockResolvedValue({ success: true, data: { enabled: false } });
    mockSetup2FA.mockResolvedValue({
      success: true,
      data: { secret: 'ABC', otpauth_uri: 'otpauth://totp/test', qr_code_base64: 'abc' },
    });

    render(<TwoFactorSetupComponent />);

    await waitFor(() => {
      expect(screen.getByText('Enable')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Enable'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Enter 6-digit code')).toBeInTheDocument();
    });

    const input = screen.getByPlaceholderText('Enter 6-digit code');
    await user.type(input, 'abc123def456');

    // Only digits should remain, and max 6
    expect(input).toHaveValue('123456');
  });
});
