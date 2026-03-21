import { type ReactNode } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { type FieldError, type UseFormRegisterReturn } from 'react-hook-form';

interface FormFieldProps {
  label: string;
  htmlFor: string;
  error?: FieldError;
  children: ReactNode;
  hint?: string;
  required?: boolean;
}

/** Accessible form field wrapper with animated error messages and ARIA. */
export function FormField({ label, htmlFor, error, children, hint, required }: FormFieldProps) {
  const errorId = `${htmlFor}-error`;
  const hintId = `${htmlFor}-hint`;

  return (
    <div>
      <label
        htmlFor={htmlFor}
        className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5"
      >
        {label}
        {required && <span className="text-danger ml-0.5" aria-hidden="true">*</span>}
      </label>
      <div
        aria-describedby={[error ? errorId : null, hint ? hintId : null].filter(Boolean).join(' ') || undefined}
        aria-invalid={!!error}
      >
        {children}
      </div>
      {hint && !error && (
        <p id={hintId} className="text-xs text-surface-400 mt-1">{hint}</p>
      )}
      <AnimatePresence mode="wait">
        {error?.message && (
          <motion.p
            key="error"
            id={errorId}
            initial={{ opacity: 0, y: -4, height: 0 }}
            animate={{ opacity: 1, y: 0, height: 'auto' }}
            exit={{ opacity: 0, y: -4, height: 0 }}
            transition={{ duration: 0.15 }}
            className="text-xs text-danger mt-1"
            role="alert"
          >
            {error.message}
          </motion.p>
        )}
      </AnimatePresence>
    </div>
  );
}

interface FormInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  registration: UseFormRegisterReturn;
  hasError?: boolean;
}

/** Styled input that integrates with react-hook-form register(). */
export function FormInput({ registration, hasError, className = '', ...props }: FormInputProps) {
  return (
    <input
      {...registration}
      {...props}
      className={`input ${hasError ? 'border-danger focus:border-danger focus:ring-danger/20' : ''} ${className}`}
    />
  );
}
