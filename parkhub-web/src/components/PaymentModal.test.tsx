import { describe, it, expect } from 'vitest';

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
