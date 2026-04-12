import { describe, it, expect } from 'vitest';
import { z } from 'zod';

// ── Schemas (mirrored from view components) ──
// These replicate the exact Zod schemas used in Login.tsx and Register.tsx
// to verify validation logic independently from the UI layer.

const loginSchema = z.object({
  username: z.string().min(1, 'Required'),
  password: z.string().min(1, 'Required'),
});

const registerSchema = z.object({
  name: z.string().min(1, 'Required'),
  email: z.string().email('Invalid email'),
  password: z
    .string()
    .min(8, '8+ characters')
    .regex(/[a-z]/, 'Needs lowercase')
    .regex(/[A-Z]/, 'Needs uppercase')
    .regex(/[0-9]/, 'Needs digit'),
  password_confirmation: z.string().min(1, 'Required'),
}).refine(data => data.password === data.password_confirmation, {
  message: 'Passwords do not match',
  path: ['password_confirmation'],
});

describe('Login schema', () => {
  it('accepts valid credentials', () => {
    const result = loginSchema.safeParse({ username: 'admin', password: 'demo' });
    expect(result.success).toBe(true);
  });

  it('rejects empty username', () => {
    const result = loginSchema.safeParse({ username: '', password: 'demo' });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0].path).toEqual(['username']);
      expect(result.error.issues[0].message).toBe('Required');
    }
  });

  it('rejects empty password', () => {
    const result = loginSchema.safeParse({ username: 'admin', password: '' });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0].path).toEqual(['password']);
    }
  });

  it('rejects both fields empty', () => {
    const result = loginSchema.safeParse({ username: '', password: '' });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues.length).toBe(2);
    }
  });

  it('rejects missing fields entirely', () => {
    const result = loginSchema.safeParse({});
    expect(result.success).toBe(false);
  });

  it('accepts whitespace-only username (min(1) only checks length)', () => {
    const result = loginSchema.safeParse({ username: ' ', password: 'demo' });
    expect(result.success).toBe(true);
  });
});

describe('Register schema', () => {
  const validData = {
    name: 'Florian Test',
    email: 'florian@test.com',
    password: 'StrongP4ss',
    password_confirmation: 'StrongP4ss',
  };

  it('accepts valid registration data', () => {
    const result = registerSchema.safeParse(validData);
    expect(result.success).toBe(true);
  });

  it('rejects empty name', () => {
    const result = registerSchema.safeParse({ ...validData, name: '' });
    expect(result.success).toBe(false);
    if (!result.success) {
      const nameIssue = result.error.issues.find(i => i.path.includes('name'));
      expect(nameIssue).toBeDefined();
    }
  });

  it('rejects invalid email format', () => {
    const result = registerSchema.safeParse({ ...validData, email: 'not-an-email' });
    expect(result.success).toBe(false);
    if (!result.success) {
      const emailIssue = result.error.issues.find(i => i.path.includes('email'));
      expect(emailIssue?.message).toBe('Invalid email');
    }
  });

  it('rejects email without domain', () => {
    const result = registerSchema.safeParse({ ...validData, email: 'user@' });
    expect(result.success).toBe(false);
  });

  it('rejects email without @', () => {
    const result = registerSchema.safeParse({ ...validData, email: 'user.example.com' });
    expect(result.success).toBe(false);
  });

  it('rejects password shorter than 8 characters', () => {
    const result = registerSchema.safeParse({
      ...validData,
      password: 'Ab1',
      password_confirmation: 'Ab1',
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      const pwIssue = result.error.issues.find(i => i.path.includes('password'));
      expect(pwIssue?.message).toBe('8+ characters');
    }
  });

  it('rejects password without lowercase letter', () => {
    const result = registerSchema.safeParse({
      ...validData,
      password: 'UPPERCASE1',
      password_confirmation: 'UPPERCASE1',
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      const pwIssue = result.error.issues.find(i => i.message === 'Needs lowercase');
      expect(pwIssue).toBeDefined();
    }
  });

  it('rejects password without uppercase letter', () => {
    const result = registerSchema.safeParse({
      ...validData,
      password: 'lowercase1',
      password_confirmation: 'lowercase1',
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      const pwIssue = result.error.issues.find(i => i.message === 'Needs uppercase');
      expect(pwIssue).toBeDefined();
    }
  });

  it('rejects password without digit', () => {
    const result = registerSchema.safeParse({
      ...validData,
      password: 'StrongPass',
      password_confirmation: 'StrongPass',
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      const pwIssue = result.error.issues.find(i => i.message === 'Needs digit');
      expect(pwIssue).toBeDefined();
    }
  });

  it('rejects mismatched password confirmation', () => {
    const result = registerSchema.safeParse({
      ...validData,
      password: 'StrongP4ss',
      password_confirmation: 'Different1',
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      const mismatch = result.error.issues.find(i =>
        i.path.includes('password_confirmation') && i.message === 'Passwords do not match'
      );
      expect(mismatch).toBeDefined();
    }
  });

  it('rejects empty password confirmation', () => {
    const result = registerSchema.safeParse({
      ...validData,
      password_confirmation: '',
    });
    expect(result.success).toBe(false);
  });

  it('accepts password with special characters', () => {
    const result = registerSchema.safeParse({
      ...validData,
      password: 'Str0ng!@#$',
      password_confirmation: 'Str0ng!@#$',
    });
    expect(result.success).toBe(true);
  });

  it('accepts very long valid password', () => {
    const longPw = 'Aa1' + 'x'.repeat(100);
    const result = registerSchema.safeParse({
      ...validData,
      password: longPw,
      password_confirmation: longPw,
    });
    expect(result.success).toBe(true);
  });

  it('rejects all fields missing', () => {
    const result = registerSchema.safeParse({});
    expect(result.success).toBe(false);
  });

  it('accepts email with subdomain', () => {
    const result = registerSchema.safeParse({
      ...validData,
      email: 'user@mail.company.co.uk',
    });
    expect(result.success).toBe(true);
  });

  it('accepts email with plus addressing', () => {
    const result = registerSchema.safeParse({
      ...validData,
      email: 'florian+test@example.com',
    });
    expect(result.success).toBe(true);
  });
});
