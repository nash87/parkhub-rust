import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockMe = vi.fn();
const mockUpdateMe = vi.fn();
const mockChangePassword = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    me: (...a: unknown[]) => mockMe(...a),
    updateMe: (...a: unknown[]) => mockUpdateMe(...a),
    changePassword: (...a: unknown[]) => mockChangePassword(...a),
  },
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

const mockSetMode = vi.fn();
vi.mock('../ThemeProvider', () => ({
  useV5Theme: () => ({ mode: 'marble_light', setMode: mockSetMode, isVoid: false, isDark: false }),
  V5_MODES: ['marble_light', 'marble_dark', 'void'] as const,
  V5_MODE_LABELS: { marble_light: '☀ Marble', marble_dark: '● Marble Dark', void: '◼ Void' },
}));

import { ProfilV5 } from './Profil';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ProfilV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const USER = {
  id: 'u-1',
  username: 'fk',
  email: 'florian@example.com',
  name: 'Florian Kaiser',
  role: 'user' as const,
  preferences: {},
  is_active: true,
  credits_balance: 10,
  credits_monthly_quota: 20,
  department: 'Engineering',
};

describe('ProfilV5', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders error state when me() fails', async () => {
    mockMe.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders user fields populated from me()', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    renderScreen();
    await waitFor(() => expect(screen.getByDisplayValue('Florian Kaiser')).toBeInTheDocument());
    expect(screen.getByDisplayValue('florian@example.com')).toBeInTheDocument();
    expect(screen.getByText('fk')).toBeInTheDocument();
  });

  it('enables save after edit and calls updateMe with trimmed values', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockUpdateMe.mockResolvedValue({ success: true, data: { ...USER, name: 'Florian K.' } });
    renderScreen();
    await waitFor(() => expect(screen.getByDisplayValue('Florian Kaiser')).toBeInTheDocument());
    const nameInput = screen.getByDisplayValue('Florian Kaiser') as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: 'Florian K.' } });
    const saveBtn = screen.getByTestId('profil-save');
    expect(saveBtn).not.toBeDisabled();
    fireEvent.click(saveBtn);
    await waitFor(() => {
      expect(mockUpdateMe).toHaveBeenCalledWith({ name: 'Florian K.', email: 'florian@example.com' });
      expect(mockToast).toHaveBeenCalledWith('Profil aktualisiert', 'success');
    });
  });

  it('shows error toast when updateMe rejects', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockUpdateMe.mockRejectedValue(new Error('boom'));
    renderScreen();
    await waitFor(() => expect(screen.getByDisplayValue('Florian Kaiser')).toBeInTheDocument());
    fireEvent.change(screen.getByDisplayValue('Florian Kaiser'), { target: { value: 'Elly' } });
    fireEvent.click(screen.getByTestId('profil-save'));
    // onError now propagates the thrown Error's message; falls back to 'Speichern fehlgeschlagen' only if empty
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('boom', 'error'));
  });

  it('submits password change when all fields valid', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockChangePassword.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Mein Profil')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('profil-pw-toggle'));
    const pwInputs = document.querySelectorAll<HTMLInputElement>('input[type="password"]');
    expect(pwInputs.length).toBe(3);
    const [currentPw, newPw, confirmPw] = pwInputs;
    fireEvent.change(currentPw, { target: { value: 'old-pass-12345' } });
    fireEvent.change(newPw, { target: { value: 'brand-new-pass-12345' } });
    fireEvent.change(confirmPw, { target: { value: 'brand-new-pass-12345' } });
    const submit = screen.getByTestId('profil-pw-submit');
    expect(submit).not.toBeDisabled();
    fireEvent.click(submit);
    await waitFor(() => {
      expect(mockChangePassword).toHaveBeenCalledWith(
        'old-pass-12345',
        'brand-new-pass-12345',
        'brand-new-pass-12345',
      );
      expect(mockToast).toHaveBeenCalledWith('Passwort geändert', 'success');
    });
  });

  it('changes language and theme mode', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Mein Profil')).toBeInTheDocument());
    const langBtns = screen.getAllByTestId('profil-lang');
    fireEvent.click(langBtns[1]); // English
    expect(mockToast).toHaveBeenCalledWith('Sprache aktualisiert', 'success');
    const themeBtns = screen.getAllByTestId('profil-theme');
    fireEvent.click(themeBtns[2]); // void
    expect(mockSetMode).toHaveBeenCalledWith('void');
  });

  it('surfaces query error when me() responds success:false', async () => {
    mockMe.mockResolvedValue({ success: false, data: null, error: { code: 'UNAUTHENTICATED', message: 'login required' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('calls onError (no success toast) when updateMe responds success:false', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockUpdateMe.mockResolvedValue({ success: false, data: null, error: { code: 'VALIDATION', message: 'email already in use' } });
    renderScreen();
    await waitFor(() => expect(screen.getByDisplayValue('Florian Kaiser')).toBeInTheDocument());
    fireEvent.change(screen.getByDisplayValue('Florian Kaiser'), { target: { value: 'Elly' } });
    fireEvent.click(screen.getByTestId('profil-save'));
    await waitFor(() => {
      expect(mockUpdateMe).toHaveBeenCalled();
      expect(mockToast).toHaveBeenCalledWith('email already in use', 'error');
    });
    expect(mockToast).not.toHaveBeenCalledWith('Profil aktualisiert', 'success');
  });

  it('calls onError (no success toast) when changePassword responds success:false', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockChangePassword.mockResolvedValue({ success: false, data: null, error: { code: 'INVALID_PASSWORD', message: 'current password wrong' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Mein Profil')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('profil-pw-toggle'));
    const pwInputs = document.querySelectorAll<HTMLInputElement>('input[type="password"]');
    const [currentPw, newPw, confirmPw] = pwInputs;
    fireEvent.change(currentPw, { target: { value: 'old-pass-12345' } });
    fireEvent.change(newPw, { target: { value: 'brand-new-pass-12345' } });
    fireEvent.change(confirmPw, { target: { value: 'brand-new-pass-12345' } });
    fireEvent.click(screen.getByTestId('profil-pw-submit'));
    await waitFor(() => {
      expect(mockChangePassword).toHaveBeenCalled();
      expect(mockToast).toHaveBeenCalledWith('current password wrong', 'error');
    });
    expect(mockToast).not.toHaveBeenCalledWith('Passwort geändert', 'success');
  });
});
