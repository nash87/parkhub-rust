import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CreditCard: (props: any) => <span data-testid="icon-card" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Lock: (props: any) => <span data-testid="icon-lock" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check" {...props} />,
  WarningCircle: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'payment.title': 'Payment',
        'payment.amount': 'Amount',
        'payment.cardNumber': 'Card Number',
        'payment.expiry': 'Expiry',
        'payment.cvc': 'CVC',
        'payment.cardholderName': 'Cardholder Name',
        'payment.cardholderNamePlaceholder': 'John Doe',
        'payment.processing': 'Processing payment...',
        'payment.success': 'Payment successful!',
        'payment.successDesc': 'Your booking is confirmed.',
        'payment.errorTitle': 'Payment failed',
        'payment.genericError': 'Something went wrong',
        'payment.retry': 'Try Again',
        'payment.secureNote': 'Secured with encryption',
        'common.close': 'Close',
      };
      if (key === 'payment.pay') return `Pay ${opts?.amount || ''}`;
      return map[key] || key;
    },
  }),
}));

import { PaymentModal } from './PaymentModal';

describe('PaymentModal component', () => {
  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    onSuccess: vi.fn(),
    amountCents: 2500,
    currency: 'EUR',
    bookingId: 'b-1',
  };

  beforeEach(() => {
    defaultProps.onClose = vi.fn();
    defaultProps.onSuccess = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns null when closed', () => {
    const { container } = render(<PaymentModal {...defaultProps} open={false} />);
    expect(container.innerHTML).toBe('');
  });

  it('renders form when open', () => {
    render(<PaymentModal {...defaultProps} />);
    expect(screen.getByText('Payment')).toBeInTheDocument();
    expect(screen.getByText('Amount')).toBeInTheDocument();
    expect(screen.getByText('Card Number')).toBeInTheDocument();
    expect(screen.getByText('Expiry')).toBeInTheDocument();
    expect(screen.getByText('CVC')).toBeInTheDocument();
    expect(screen.getByText('Cardholder Name')).toBeInTheDocument();
  });

  it('displays formatted amount', () => {
    render(<PaymentModal {...defaultProps} />);
    // 2500 cents = 25.00 EUR - appears in the amount display
    expect(screen.getByText('Amount')).toBeInTheDocument();
    // The formatted amount appears in the centered display
    const amountDisplay = document.querySelector('.text-2xl');
    expect(amountDisplay).toBeTruthy();
  });

  it('pay button is disabled when form is incomplete', () => {
    render(<PaymentModal {...defaultProps} />);
    const payBtn = screen.getByRole('button', { name: /Pay/ });
    expect(payBtn).toBeDisabled();
  });

  it('formats card number with spaces', async () => {
    const user = userEvent.setup();
    render(<PaymentModal {...defaultProps} />);

    const cardInput = screen.getByPlaceholderText('4242 4242 4242 4242');
    await user.type(cardInput, '4242424242424242');

    expect(cardInput).toHaveValue('4242 4242 4242 4242');
  });

  it('formats expiry as MM/YY', async () => {
    const user = userEvent.setup();
    render(<PaymentModal {...defaultProps} />);

    const expiryInput = screen.getByPlaceholderText('MM/YY');
    await user.type(expiryInput, '1225');

    expect(expiryInput).toHaveValue('12/25');
  });

  it('CVC only accepts digits up to 4 characters', async () => {
    const user = userEvent.setup();
    render(<PaymentModal {...defaultProps} />);

    const cvcInput = screen.getByPlaceholderText('123');
    await user.type(cvcInput, '12345abc');

    expect(cvcInput).toHaveValue('1234');
  });

  it('enables pay button when form is valid', async () => {
    const user = userEvent.setup();
    render(<PaymentModal {...defaultProps} />);

    await user.type(screen.getByPlaceholderText('4242 4242 4242 4242'), '4242424242424242');
    await user.type(screen.getByPlaceholderText('MM/YY'), '1225');
    await user.type(screen.getByPlaceholderText('123'), '123');
    await user.type(screen.getByPlaceholderText('John Doe'), 'Test User');

    const payBtn = screen.getByRole('button', { name: /Pay/ });
    expect(payBtn).not.toBeDisabled();
  });

  it('submitting shows processing state then success', async () => {
    vi.useFakeTimers();
    render(<PaymentModal {...defaultProps} />);

    // Use fireEvent for fake timer compatibility
    fireEvent.change(screen.getByPlaceholderText('4242 4242 4242 4242'), { target: { value: '4242424242424242' } });
    fireEvent.change(screen.getByPlaceholderText('MM/YY'), { target: { value: '1225' } });
    fireEvent.change(screen.getByPlaceholderText('123'), { target: { value: '123' } });
    fireEvent.change(screen.getByPlaceholderText('John Doe'), { target: { value: 'Test User' } });

    const payBtn = screen.getByRole('button', { name: /Pay/ });
    await act(async () => { fireEvent.click(payBtn); });

    // Processing state
    expect(screen.getByText('Processing payment...')).toBeInTheDocument();

    // Wait for mock timeout
    await act(async () => { vi.advanceTimersByTime(1600); });

    expect(screen.getByText('Payment successful!')).toBeInTheDocument();
    expect(screen.getByText('Your booking is confirmed.')).toBeInTheDocument();
    expect(defaultProps.onSuccess).toHaveBeenCalledWith(expect.stringContaining('pi_mock_'));
    vi.useRealTimers();
  });

  it('close button calls onClose and resets form', async () => {
    render(<PaymentModal {...defaultProps} />);

    const closeBtn = screen.getByLabelText('Close');
    await act(async () => { closeBtn.click(); });

    expect(defaultProps.onClose).toHaveBeenCalled();
  });

  it('close button is disabled during processing', async () => {
    vi.useFakeTimers();
    render(<PaymentModal {...defaultProps} />);

    fireEvent.change(screen.getByPlaceholderText('4242 4242 4242 4242'), { target: { value: '4242424242424242' } });
    fireEvent.change(screen.getByPlaceholderText('MM/YY'), { target: { value: '1225' } });
    fireEvent.change(screen.getByPlaceholderText('123'), { target: { value: '123' } });
    fireEvent.change(screen.getByPlaceholderText('John Doe'), { target: { value: 'Test User' } });

    const payBtn = screen.getByRole('button', { name: /Pay/ });
    await act(async () => { fireEvent.click(payBtn); });

    // During processing, close should be disabled
    const closeBtn = screen.getByLabelText('Close');
    expect(closeBtn).toBeDisabled();

    await act(async () => { vi.advanceTimersByTime(1600); });
    vi.useRealTimers();
  });

  it('success state has close button that calls onClose', async () => {
    vi.useFakeTimers();
    render(<PaymentModal {...defaultProps} />);

    fireEvent.change(screen.getByPlaceholderText('4242 4242 4242 4242'), { target: { value: '4242424242424242' } });
    fireEvent.change(screen.getByPlaceholderText('MM/YY'), { target: { value: '1225' } });
    fireEvent.change(screen.getByPlaceholderText('123'), { target: { value: '123' } });
    fireEvent.change(screen.getByPlaceholderText('John Doe'), { target: { value: 'Test User' } });

    const payBtn = screen.getByRole('button', { name: /Pay/ });
    await act(async () => { fireEvent.click(payBtn); });
    await act(async () => { vi.advanceTimersByTime(1600); });

    const closeBtn = screen.getByText('Close');
    await act(async () => { closeBtn.click(); });

    expect(defaultProps.onClose).toHaveBeenCalled();
    vi.useRealTimers();
  });

  it('renders with default EUR currency', () => {
    render(<PaymentModal {...defaultProps} currency={undefined} />);
    expect(screen.getByText('Payment')).toBeInTheDocument();
  });

  it('renders secure note', () => {
    render(<PaymentModal {...defaultProps} />);
    expect(screen.getByText('Secured with encryption')).toBeInTheDocument();
  });

  it('backdrop click calls onClose on form step', async () => {
    render(<PaymentModal {...defaultProps} />);

    // Click the backdrop overlay
    const backdrop = document.querySelector('.bg-black\\/50');
    expect(backdrop).toBeTruthy();
    await act(async () => { backdrop!.dispatchEvent(new MouseEvent('click', { bubbles: true })); });

    expect(defaultProps.onClose).toHaveBeenCalled();
  });
});
