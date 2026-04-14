import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockAdminGetSettings = vi.fn();
const mockAdminUpdateSettings = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminGetSettings: (...args: any[]) => mockAdminGetSettings(...args),
    adminUpdateSettings: (...args: any[]) => mockAdminUpdateSettings(...args),
  },
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    p: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <p ref={ref} {...props}>{children}</p>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: (...args: any[]) => mockToastSuccess(...args), error: (...args: any[]) => mockToastError(...args) },
}));

import { AdminSettingsPage } from './AdminSettings';

describe('AdminSettingsPage', () => {
  beforeEach(() => {
    mockAdminGetSettings.mockClear();
    mockAdminUpdateSettings.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders loading spinner initially', () => {
    mockAdminGetSettings.mockReturnValue(new Promise(() => {}));

    render(<AdminSettingsPage />);
    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('loads settings from API and displays them', async () => {
    mockAdminGetSettings.mockResolvedValue({
      success: true,
      data: { company_name: 'Acme Parking', use_case: 'residential' },
    });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Acme Parking')).toBeInTheDocument();
    });
    // Select shows the option text for display value
    expect(screen.getByDisplayValue('Residential')).toBeInTheDocument();
  });

  it('renders section headings', async () => {
    mockAdminGetSettings.mockResolvedValue({ success: true, data: {} });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('System Settings')).toBeInTheDocument();
    });
    expect(screen.getByText('General')).toBeInTheDocument();
    expect(screen.getByText('Booking Rules')).toBeInTheDocument();
    expect(screen.getByText('Auto-Release')).toBeInTheDocument();
    expect(screen.getByText('Waitlist')).toBeInTheDocument();
    expect(screen.getByText('Credits System')).toBeInTheDocument();
    expect(screen.getByText('License Plate')).toBeInTheDocument();
  });

  it('toggle switch changes value on click', async () => {
    const user = userEvent.setup();
    mockAdminGetSettings.mockResolvedValue({
      success: true,
      data: { self_registration: 'true' },
    });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Self Registration')).toBeInTheDocument();
    });

    // Find the toggle switch for Self Registration
    const selfRegToggle = screen.getByText('Self Registration')
      .closest('div.flex')!
      .querySelector('[role="switch"]') as HTMLElement;
    expect(selfRegToggle).toHaveAttribute('aria-checked', 'true');

    await user.click(selfRegToggle);

    expect(selfRegToggle).toHaveAttribute('aria-checked', 'false');
  });

  it('save button triggers API call', async () => {
    const user = userEvent.setup();
    mockAdminGetSettings.mockResolvedValue({
      success: true,
      data: { company_name: 'TestCo' },
    });
    mockAdminUpdateSettings.mockResolvedValue({ success: true });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('TestCo')).toBeInTheDocument();
    });

    const saveBtn = screen.getByRole('button', { name: /Save Settings/ });
    await user.click(saveBtn);

    await waitFor(() => {
      expect(mockAdminUpdateSettings).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Settings saved');
    });
  });

  it('save button shows error on failure', async () => {
    const user = userEvent.setup();
    mockAdminGetSettings.mockResolvedValue({ success: true, data: {} });
    mockAdminUpdateSettings.mockResolvedValue({
      success: false,
      error: { message: 'Permission denied' },
    });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('System Settings')).toBeInTheDocument();
    });

    const saveBtn = screen.getByRole('button', { name: /Save Settings/ });
    await user.click(saveBtn);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Permission denied');
    });
  });

  it('use case dropdown has 5 options', async () => {
    mockAdminGetSettings.mockResolvedValue({ success: true, data: {} });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Use Case')).toBeInTheDocument();
    });

    const useCaseSelect = screen.getByDisplayValue('Company');
    const options = useCaseSelect.querySelectorAll('option');
    expect(options).toHaveLength(5);

    const values = Array.from(options).map(o => o.getAttribute('value'));
    expect(values).toEqual(['company', 'residential', 'shared', 'rental', 'personal']);
  });

  it('company name input can be edited', async () => {
    const user = userEvent.setup();
    mockAdminGetSettings.mockResolvedValue({
      success: true,
      data: { company_name: 'Old Name' },
    });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Old Name')).toBeInTheDocument();
    });

    const input = screen.getByDisplayValue('Old Name');
    await user.clear(input);
    await user.type(input, 'New Name');

    expect(screen.getByDisplayValue('New Name')).toBeInTheDocument();
  });

  it('renders toggle labels for all boolean settings', async () => {
    mockAdminGetSettings.mockResolvedValue({ success: true, data: {} });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Self Registration')).toBeInTheDocument();
    });
    expect(screen.getByText('Allow Guest Bookings')).toBeInTheDocument();
    expect(screen.getByText('Require Vehicle')).toBeInTheDocument();
    expect(screen.getByText('Enable Waitlist')).toBeInTheDocument();
    expect(screen.getByText('Enable Credits')).toBeInTheDocument();
  });

  it('shows error toast when settings fail to load', async () => {
    mockAdminGetSettings.mockRejectedValue(new Error('Network error'));

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Failed to load settings');
    });
  });

  it('shows generic error toast when save throws', async () => {
    const user = userEvent.setup();
    mockAdminGetSettings.mockResolvedValue({ success: true, data: {} });
    mockAdminUpdateSettings.mockRejectedValue(new Error('save failed'));

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('System Settings')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /Save Settings/ }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Failed to save settings');
    });
  });

  it('updates select and numeric fields in both columns before saving', async () => {
    mockAdminGetSettings.mockResolvedValue({
      success: true,
      data: {
        use_case: 'company',
        max_bookings_per_day: '3',
        auto_release_minutes: '30',
        license_plate_mode: 'optional',
      },
    });
    mockAdminUpdateSettings.mockResolvedValue({ success: true });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Company')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Use Case'), { target: { value: 'rental' } });
    fireEvent.change(screen.getByLabelText('Max Bookings per Day'), { target: { value: '7' } });
    fireEvent.change(screen.getByLabelText('Auto-release after (minutes)'), { target: { value: '45' } });
    fireEvent.change(screen.getByLabelText('License Plate Mode'), { target: { value: 'required' } });

    fireEvent.click(screen.getByRole('button', { name: /Save Settings/ }));

    await waitFor(() => {
      expect(mockAdminUpdateSettings).toHaveBeenCalledWith(expect.objectContaining({
        use_case: 'rental',
        max_bookings_per_day: '7',
        auto_release_minutes: '45',
        license_plate_mode: 'required',
      }));
    });
  });

  it('shows credits per booking field when credits are enabled', async () => {
    const user = userEvent.setup();
    mockAdminGetSettings.mockResolvedValue({
      success: true,
      data: { credits_enabled: 'false' },
    });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Enable Credits')).toBeInTheDocument();
    });

    expect(screen.queryByLabelText('Credits per Booking')).not.toBeInTheDocument();

    const creditsToggle = screen.getByText('Enable Credits')
      .closest('div.flex')!
      .querySelector('[role="switch"]') as HTMLElement;
    await user.click(creditsToggle);

    expect(screen.getByLabelText('Credits per Booking')).toBeInTheDocument();
  });

  it('updates waitlist and credits settings before saving', async () => {
    const user = userEvent.setup();
    mockAdminGetSettings.mockResolvedValue({ success: true, data: {} });
    mockAdminUpdateSettings.mockResolvedValue({ success: true });

    render(<AdminSettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Enable Waitlist')).toBeInTheDocument();
    });

    const waitlistToggle = screen.getByText('Enable Waitlist')
      .closest('div.flex')!
      .querySelector('[role="switch"]') as HTMLElement;
    const creditsToggle = screen.getByText('Enable Credits')
      .closest('div.flex')!
      .querySelector('[role="switch"]') as HTMLElement;

    await user.click(waitlistToggle);
    await user.click(creditsToggle);
    await user.clear(screen.getByLabelText('Credits per Booking'));
    await user.type(screen.getByLabelText('Credits per Booking'), '4');

    await user.click(screen.getByRole('button', { name: /Save Settings/ }));

    await waitFor(() => {
      expect(mockAdminUpdateSettings).toHaveBeenCalledWith(expect.objectContaining({
        waitlist_enabled: 'false',
        credits_enabled: 'true',
        credits_per_booking: '4',
      }));
    });
  });
});
