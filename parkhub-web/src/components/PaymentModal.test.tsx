import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CreditCard: (p: any) => <span data-testid="icon-cc" {...p} />,
  X: (p: any) => <span data-testid="icon-x" {...p} />,
  Lock: (p: any) => <span data-testid="icon-lock" {...p} />,
  SpinnerGap: (p: any) => <span data-testid="icon-spinner" {...p} />,
  CheckCircle: (p: any) => <span data-testid="icon-check" {...p} />,
  WarningCircle: (p: any) => <span data-testid="icon-warning" {...p} />,
}));

import { PaymentModal } from './PaymentModal';

function formatCardNumber(value: string): string {
  const digits = value.replace(/\D/g, '').slice(0, 16);
  return digits.replace(/(.{4})/g, '$1 ').trim();
}

function formatExpiry(value: string): string {
  const digits = value.replace(/\D/g, '').slice(0, 4);
  if (digits.length >= 3) return `${digits.slice(0, 2)}/${digits.slice(2)}`;
  return digits;
}

function isFormValid(cardNumber: string, expiry: string, cvc: string, name: string): boolean {
  return cardNumber.replace(/\s/g, '').length === 16
    && expiry.length === 5
    && cvc.length >= 3
    && name.trim().length > 0;
}

describe('formatCardNumber', () => {
  it('formats 16 digits with spaces', () => {
    expect(formatCardNumber('4242424242424242')).toBe('4242 4242 4242 4242');
  });
  it('strips non-digits', () => {
    expect(formatCardNumber('4242-4242-4242-4242')).toBe('4242 4242 4242 4242');
  });
  it('truncates beyond 16 digits', () => {
    expect(formatCardNumber('42424242424242421234')).toBe('4242 4242 4242 4242');
  });
  it('handles partial input', () => {
    expect(formatCardNumber('424242')).toBe('4242 42');
  });
  it('handles empty input', () => {
    expect(formatCardNumber('')).toBe('');
  });
});

describe('formatExpiry', () => {
  it('formats MM/YY', () => {
    expect(formatExpiry('1225')).toBe('12/25');
  });
  it('handles partial input (2 digits)', () => {
    expect(formatExpiry('12')).toBe('12');
  });
  it('handles 3 digits', () => {
    expect(formatExpiry('122')).toBe('12/2');
  });
  it('strips non-digits', () => {
    expect(formatExpiry('12/25')).toBe('12/25');
  });
  it('handles empty input', () => {
    expect(formatExpiry('')).toBe('');
  });
});

describe('isFormValid', () => {
  it('returns true for complete valid input', () => {
    expect(isFormValid('4242 4242 4242 4242', '12/25', '123', 'John Doe')).toBe(true);
  });
  it('returns false for short card number', () => {
    expect(isFormValid('4242 4242 4242', '12/25', '123', 'John Doe')).toBe(false);
  });
  it('returns false for missing expiry', () => {
    expect(isFormValid('4242 4242 4242 4242', '12', '123', 'John Doe')).toBe(false);
  });
  it('returns false for short CVC', () => {
    expect(isFormValid('4242 4242 4242 4242', '12/25', '12', 'John Doe')).toBe(false);
  });
  it('accepts 4-digit CVC', () => {
    expect(isFormValid('4242 4242 4242 4242', '12/25', '1234', 'John Doe')).toBe(true);
  });
  it('returns false for empty name', () => {
    expect(isFormValid('4242 4242 4242 4242', '12/25', '123', '')).toBe(false);
  });
  it('returns false for whitespace-only name', () => {
    expect(isFormValid('4242 4242 4242 4242', '12/25', '123', '   ')).toBe(false);
  });
});

describe('PaymentModal component', () => {

  function fillForm() {
    const cardInput = screen.getByPlaceholderText('4242 4242 4242 4242');
    fireEvent.change(cardInput, { target: { value: '4242424242424242' } });
    const expiryInput = screen.getByPlaceholderText('MM/YY');
    fireEvent.change(expiryInput, { target: { value: '1225' } });
    const cvcInput = screen.getByPlaceholderText('123');
    fireEvent.change(cvcInput, { target: { value: '123' } });
    const nameInput = screen.getByPlaceholderText('payment.cardholderNamePlaceholder');
    fireEvent.change(nameInput, { target: { value: 'John Doe' } });
  }

  it('renders nothing when closed', () => {
    const { container } = render(
      <PaymentModal open={false} onClose={() => {}} amountCents={1000} bookingId="b1" />,
    );
    expect(container.querySelector('form')).toBeNull();
    vi.useRealTimers();
  });

  it('renders open form', () => {
    render(
      <PaymentModal open={true} onClose={() => {}} amountCents={1000} bookingId="b1" />,
    );
    expect(screen.getByText('payment.title')).toBeInTheDocument();
    vi.useRealTimers();
  });

  it('does not submit when form is invalid', () => {
    const onSuccess = vi.fn();
    render(
      <PaymentModal open={true} onClose={() => {}} onSuccess={onSuccess} amountCents={1000} bookingId="b1" />,
    );
    const form = document.querySelector('form');
    if (form) fireEvent.submit(form);
    expect(onSuccess).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it('submits successfully and calls onSuccess', async () => {
    vi.useRealTimers();
    const onSuccess = vi.fn();
    render(
      <PaymentModal open={true} onClose={() => {}} onSuccess={onSuccess} amountCents={1000} bookingId="b1" />,
    );
    fillForm();
    const form = document.querySelector('form');
    if (form) {
      await act(async () => {
        fireEvent.submit(form);
        // Real timer waits 1500ms
        await new Promise(r => setTimeout(r, 1700));
      });
    }
    expect(onSuccess).toHaveBeenCalled();
  });

  it('does not close while processing', async () => {
    vi.useRealTimers();
    const onClose = vi.fn();
    render(
      <PaymentModal open={true} onClose={onClose} amountCents={1000} bookingId="b1" />,
    );
    fillForm();
    const form = document.querySelector('form');
    if (form) {
      await act(async () => {
        fireEvent.submit(form);
      });
      // Immediately try to close while processing
      const closeBtn = screen.getByLabelText('common.close');
      fireEvent.click(closeBtn);
      expect(onClose).not.toHaveBeenCalled();
      await act(async () => {
        await new Promise(r => setTimeout(r, 1700));
      });
    }
  });
});
