import { useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CreditCard, X, Lock, SpinnerGap, CheckCircle, WarningCircle } from '@phosphor-icons/react';

type PaymentStep = 'form' | 'processing' | 'success' | 'error';

interface Props {
  open: boolean;
  onClose: () => void;
  onSuccess?: (paymentIntentId: string) => void;
  amountCents: number;
  currency?: string;
  bookingId: string;
}

export function PaymentModal({ open, onClose, onSuccess, amountCents, currency = 'EUR', bookingId }: Props) {
  const { t } = useTranslation();
  const [step, setStep] = useState<PaymentStep>('form');
  const [cardNumber, setCardNumber] = useState('');
  const [expiry, setExpiry] = useState('');
  const [cvc, setCvc] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState('');

  void bookingId;

  const formatAmount = useCallback(() => {
    return new Intl.NumberFormat(undefined, {
      style: 'currency',
      currency,
    }).format(amountCents / 100);
  }, [amountCents, currency]);

  const formatCardNumber = (value: string) => {
    const digits = value.replace(/\D/g, '').slice(0, 16);
    return digits.replace(/(.{4})/g, '$1 ').trim();
  };

  const formatExpiry = (value: string) => {
    const digits = value.replace(/\D/g, '').slice(0, 4);
    if (digits.length >= 3) return `${digits.slice(0, 2)}/${digits.slice(2)}`;
    return digits;
  };

  const isFormValid = cardNumber.replace(/\s/g, '').length === 16
    && expiry.length === 5
    && cvc.length >= 3
    && name.trim().length > 0;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!isFormValid) return;

    setStep('processing');
    setError('');

    try {
      await new Promise(resolve => setTimeout(resolve, 1500));
      const mockIntentId = `pi_mock_${Date.now()}`;
      setStep('success');
      onSuccess?.(mockIntentId);
    } catch {
      setError(t('payment.genericError'));
      setStep('error');
    }
  };

  const handleClose = () => {
    if (step === 'processing') return;
    setStep('form');
    setCardNumber('');
    setExpiry('');
    setCvc('');
    setName('');
    setError('');
    onClose();
  };

  if (!open) return null;

  return (
    <AnimatePresence>
      {open && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 bg-black/50 backdrop-blur-sm"
            onClick={handleClose}
          />
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 20 }}
            className="relative w-full max-w-md bg-white/80 dark:bg-gray-900/80 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/20 dark:border-gray-700/30 overflow-hidden"
          >
            <div className="flex items-center justify-between p-5 border-b border-gray-200 dark:border-gray-700">
              <div className="flex items-center gap-2">
                <CreditCard size={24} weight="duotone" className="text-indigo-500" />
                <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                  {t('payment.title')}
                </h2>
              </div>
              <button
                onClick={handleClose}
                disabled={step === 'processing'}
                className="p-1.5 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors disabled:opacity-50"
                aria-label={t('common.close')}
              >
                <X size={20} className="text-gray-500" />
              </button>
            </div>
            <div className="p-5">
              {step === 'form' && (
                <form onSubmit={handleSubmit} className="space-y-4">
                  <div className="text-center py-3 bg-gray-50 dark:bg-gray-800 rounded-xl">
                    <p className="text-sm text-gray-500 dark:text-gray-400">{t('payment.amount')}</p>
                    <p className="text-2xl font-bold text-gray-900 dark:text-white">{formatAmount()}</p>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                      {t('payment.cardNumber')}
                    </label>
                    <input
                      type="text"
                      value={cardNumber}
                      onChange={e => setCardNumber(formatCardNumber(e.target.value))}
                      placeholder="4242 4242 4242 4242"
                      className="w-full px-3 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent transition-shadow"
                      autoComplete="cc-number"
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        {t('payment.expiry')}
                      </label>
                      <input
                        type="text"
                        value={expiry}
                        onChange={e => setExpiry(formatExpiry(e.target.value))}
                        placeholder="MM/YY"
                        className="w-full px-3 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent transition-shadow"
                        autoComplete="cc-exp"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        {t('payment.cvc')}
                      </label>
                      <input
                        type="text"
                        value={cvc}
                        onChange={e => setCvc(e.target.value.replace(/\D/g, '').slice(0, 4))}
                        placeholder="123"
                        className="w-full px-3 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent transition-shadow"
                        autoComplete="cc-csc"
                      />
                    </div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                      {t('payment.cardholderName')}
                    </label>
                    <input
                      type="text"
                      value={name}
                      onChange={e => setName(e.target.value)}
                      placeholder={t('payment.cardholderNamePlaceholder')}
                      className="w-full px-3 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent transition-shadow"
                      autoComplete="cc-name"
                    />
                  </div>
                  <button
                    type="submit"
                    disabled={!isFormValid}
                    className="w-full flex items-center justify-center gap-2 py-3 px-4 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <Lock size={16} weight="bold" />
                    {t('payment.pay', { amount: formatAmount() })}
                  </button>
                  <p className="text-xs text-center text-gray-400 dark:text-gray-500 flex items-center justify-center gap-1">
                    <Lock size={12} />
                    {t('payment.secureNote')}
                  </p>
                </form>
              )}
              {step === 'processing' && (
                <div className="flex flex-col items-center py-12 gap-4">
                  <motion.div animate={{ rotate: 360 }} transition={{ repeat: Infinity, duration: 1, ease: 'linear' }}>
                    <SpinnerGap size={48} className="text-indigo-500" />
                  </motion.div>
                  <p className="text-gray-600 dark:text-gray-400">{t('payment.processing')}</p>
                </div>
              )}
              {step === 'success' && (
                <div className="flex flex-col items-center py-12 gap-4">
                  <motion.div initial={{ scale: 0 }} animate={{ scale: 1 }} transition={{ type: 'spring' }}>
                    <CheckCircle size={64} weight="duotone" className="text-green-500" />
                  </motion.div>
                  <p className="text-lg font-semibold text-gray-900 dark:text-white">{t('payment.success')}</p>
                  <p className="text-gray-500 dark:text-gray-400 text-sm">{t('payment.successDesc')}</p>
                  <button
                    onClick={handleClose}
                    className="mt-2 px-6 py-2.5 rounded-xl bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-900 dark:text-white font-medium transition-colors"
                  >
                    {t('common.close')}
                  </button>
                </div>
              )}
              {step === 'error' && (
                <div className="flex flex-col items-center py-12 gap-4">
                  <WarningCircle size={64} weight="duotone" className="text-red-500" />
                  <p className="text-lg font-semibold text-gray-900 dark:text-white">{t('payment.errorTitle')}</p>
                  <p className="text-gray-500 dark:text-gray-400 text-sm">{error || t('payment.genericError')}</p>
                  <button
                    onClick={() => setStep('form')}
                    className="mt-2 px-6 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white font-medium transition-colors"
                  >
                    {t('payment.retry')}
                  </button>
                </div>
              )}
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
}
