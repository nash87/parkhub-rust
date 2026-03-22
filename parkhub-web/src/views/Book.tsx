import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'react-i18next';
import {
  ArrowLeft, MapPin, Clock, Car, SpinnerGap, Check,
  Lightning, Wheelchair, Motorcycle, Star,
} from '@phosphor-icons/react';
import { api, type ParkingLot, type ParkingSlot, type Vehicle, type CreateBookingPayload } from '../api/client';
import { SkeletonCard } from '../components/Skeleton';
import toast from 'react-hot-toast';

type Step = 1 | 2 | 3;

const DURATIONS = [
  { label: '1h', hours: 1 },
  { label: '2h', hours: 2 },
  { label: '4h', hours: 4 },
  { label: '8h', hours: 8 },
];

export function BookPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const [step, setStep] = useState<Step>(1);
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [slots, setSlots] = useState<ParkingSlot[]>([]);
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [loadingLots, setLoadingLots] = useState(true);
  const [loadingSlots, setLoadingSlots] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [confirmed, setConfirmed] = useState(false);

  const [selectedLot, setSelectedLot] = useState<ParkingLot | null>(null);
  const [selectedSlot, setSelectedSlot] = useState<ParkingSlot | null>(null);
  const [selectedVehicle, setSelectedVehicle] = useState<string>('');
  const [startDate, setStartDate] = useState(() => {
    const now = new Date();
    now.setMinutes(0, 0, 0);
    now.setHours(now.getHours() + 1);
    return now.toISOString().slice(0, 16);
  });
  const [duration, setDuration] = useState(2);

  useEffect(() => {
    Promise.all([api.getLots(), api.getVehicles()]).then(([lRes, vRes]) => {
      if (lRes.success && lRes.data) setLots(lRes.data.filter(l => l.status === 'open'));
      if (vRes.success && vRes.data) {
        setVehicles(vRes.data);
        const def = vRes.data.find(v => v.is_default);
        if (def) setSelectedVehicle(def.id);
      }
    }).finally(() => setLoadingLots(false));
  }, []);

  async function selectLot(lot: ParkingLot) {
    setSelectedLot(lot);
    setSelectedSlot(null);
    setLoadingSlots(true);
    setStep(2);
    const res = await api.getLotSlots(lot.id);
    if (res.success && res.data) setSlots(res.data);
    else { toast.error(t('common.error')); setSlots([]); }
    setLoadingSlots(false);
  }

  function goToConfirm() { if (selectedSlot) setStep(3); }

  function goBack() {
    if (step === 2) { setStep(1); setSelectedLot(null); setSelectedSlot(null); }
    else if (step === 3) setStep(2);
  }

  async function handleConfirm() {
    if (!selectedLot || !selectedSlot) return;
    setSubmitting(true);
    const start = new Date(startDate);
    const end = new Date(start.getTime() + duration * 60 * 60 * 1000);
    const payload: CreateBookingPayload = {
      lot_id: selectedLot.id, slot_id: selectedSlot.id,
      start_time: start.toISOString(), end_time: end.toISOString(),
      vehicle_id: selectedVehicle || undefined,
    };
    const res = await api.createBooking(payload);
    if (res.success) {
      setConfirmed(true);
      toast.success(t('book.success'));
      navigate('/bookings');
    } else {
      const msg = res.error?.code === 'INSUFFICIENT_CREDITS'
        ? t('bookings.insufficientCredits') : res.error?.message || t('common.error');
      toast.error(msg);
    }
    setSubmitting(false);
  }

  const start = new Date(startDate);
  const end = new Date(start.getTime() + duration * 60 * 60 * 1000);
  const estimatedCost = selectedLot?.hourly_rate ? (selectedLot.hourly_rate * duration).toFixed(2) : null;

  const slideVariants = {
    enter: (dir: number) => ({ x: dir > 0 ? 80 : -80, opacity: 0 }),
    center: { x: 0, opacity: 1 },
    exit: (dir: number) => ({ x: dir > 0 ? -80 : 80, opacity: 0 }),
  };
  const direction = step;

  return (
    <div className="space-y-6">
      {confirmed && <ConfettiOverlay />}

      <div className="flex items-center gap-3">
        {step > 1 && (
          <button onClick={goBack} className="btn btn-ghost btn-sm p-1.5" aria-label={t('common.back', 'Go back')}>
            <ArrowLeft weight="bold" className="w-5 h-5" aria-hidden="true" />
          </button>
        )}
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.02em' }}>
            {t('book.title')}
          </h1>
          <p className="text-sm text-surface-500 dark:text-surface-400 mt-0.5">{t(`book.step${step}Label`)}</p>
        </div>
      </div>

      {/* Step indicator — animated progress */}
      <nav aria-label={t('book.progress', 'Booking progress')} className="relative">
        <div className="flex items-center gap-0 text-sm">
          {[1, 2, 3].map((s, i) => (
            <div key={s} className="flex items-center flex-1">
              <div className="flex items-center gap-2 flex-shrink-0">
                <motion.div
                  className={`w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold transition-colors ${
                    s === step ? 'bg-primary-600 text-white shadow-lg shadow-primary-500/25'
                    : s < step ? 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300'
                    : 'bg-surface-100 dark:bg-surface-800 text-surface-400 dark:text-surface-500'
                  }`}
                  animate={s === step ? { scale: [1, 1.1, 1] } : {}}
                  transition={{ duration: 0.3 }}
                >
                  {s < step ? <Check weight="bold" className="w-3.5 h-3.5" aria-hidden="true" /> : s}
                </motion.div>
                <span
                  aria-current={s === step ? 'step' : undefined}
                  className={`font-medium hidden sm:inline ${
                    s === step ? 'text-primary-700 dark:text-primary-300'
                    : s < step ? 'text-surface-900 dark:text-white'
                    : 'text-surface-400 dark:text-surface-500'
                  }`}
                >
                  {t(`book.stepName${s}`)}
                </span>
              </div>
              {i < 2 && (
                <div className="flex-1 h-0.5 mx-3 rounded-full bg-surface-200 dark:bg-surface-700 overflow-hidden">
                  <motion.div
                    className="h-full bg-primary-500 rounded-full"
                    initial={{ width: '0%' }}
                    animate={{ width: s < step ? '100%' : '0%' }}
                    transition={{ duration: 0.4, ease: 'easeOut' }}
                  />
                </div>
              )}
            </div>
          ))}
        </div>
      </nav>

      <AnimatePresence mode="wait" custom={direction}>
        {step === 1 && (
          <motion.div key="step1" custom={1} variants={slideVariants} initial="enter" animate="center" exit="exit" transition={{ duration: 0.2 }}>
            <StepSelectLot lots={lots} loading={loadingLots} onSelect={selectLot} t={t} />
          </motion.div>
        )}
        {step === 2 && (
          <motion.div key="step2" custom={2} variants={slideVariants} initial="enter" animate="center" exit="exit" transition={{ duration: 0.2 }}>
            <StepSelectSlot lot={selectedLot!} slots={slots} loading={loadingSlots} selectedSlot={selectedSlot}
              onSelectSlot={setSelectedSlot} startDate={startDate} onStartDateChange={setStartDate}
              duration={duration} onDurationChange={setDuration} vehicles={vehicles}
              selectedVehicle={selectedVehicle} onVehicleChange={setSelectedVehicle} onContinue={goToConfirm} t={t} />
          </motion.div>
        )}
        {step === 3 && (
          <motion.div key="step3" custom={3} variants={slideVariants} initial="enter" animate="center" exit="exit" transition={{ duration: 0.2 }}>
            <StepConfirm lot={selectedLot!} slot={selectedSlot!} start={start} end={end} duration={duration}
              estimatedCost={estimatedCost} vehicle={vehicles.find(v => v.id === selectedVehicle)}
              submitting={submitting} onConfirm={handleConfirm} t={t} />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function StepSelectLot({ lots, loading, onSelect, t }: {
  lots: ParkingLot[]; loading: boolean; onSelect: (lot: ParkingLot) => void; t: TFunction;
}) {
  if (loading) return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {[1, 2, 3].map(i => <SkeletonCard key={i} height="h-36" />)}
    </div>
  );
  if (lots.length === 0) return (
    <div className="glass-card p-8"><p className="text-surface-500 dark:text-surface-400">{t('book.noLots')}</p></div>
  );
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {lots.map((lot, i) => (
        <motion.button key={lot.id}
          initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }}
          transition={{ delay: i * 0.05, type: 'spring', stiffness: 300, damping: 24 }}
          whileHover={{ scale: 1.02, y: -2 }} whileTap={{ scale: 0.98 }}
          onClick={() => onSelect(lot)}
          className="text-left glass-card p-5 group transition-all hover:shadow-md hover:border-primary-400/50 dark:hover:border-primary-600/50"
        >
          <p className="font-semibold text-surface-900 dark:text-white">{lot.name}</p>
          {lot.address && (
            <p className="text-sm text-surface-500 dark:text-surface-400 mt-1 flex items-center gap-1">
              <MapPin weight="regular" className="w-3.5 h-3.5 shrink-0" />{lot.address}
            </p>
          )}
          <div className="mt-3 flex items-center justify-between">
            <span className="text-sm text-surface-600 dark:text-surface-300">
              {t('book.availableSlots', { count: lot.available_slots, total: lot.total_slots })}
            </span>
            {lot.available_slots === 0 && <span className="text-xs font-medium text-red-600 dark:text-red-400">{t('book.full')}</span>}
          </div>
          {lot.hourly_rate != null && (
            <p className="text-xs text-surface-500 dark:text-surface-400 mt-1">{lot.currency || '€'}{lot.hourly_rate.toFixed(2)}/h</p>
          )}
        </motion.button>
      ))}
    </div>
  );
}

const SLOT_TYPE_ICON: Record<string, React.ComponentType<{ weight?: string; className?: string }>> = {
  electric: Lightning, handicap: Wheelchair, motorcycle: Motorcycle, vip: Star,
};

function StepSelectSlot({ lot, slots, loading, selectedSlot, onSelectSlot,
  startDate, onStartDateChange, duration, onDurationChange,
  vehicles, selectedVehicle, onVehicleChange, onContinue, t }: {
  lot: ParkingLot; slots: ParkingSlot[]; loading: boolean; selectedSlot: ParkingSlot | null;
  onSelectSlot: (s: ParkingSlot) => void; startDate: string; onStartDateChange: (v: string) => void;
  duration: number; onDurationChange: (v: number) => void; vehicles: Vehicle[];
  selectedVehicle: string; onVehicleChange: (v: string) => void; onContinue: () => void; t: TFunction;
}) {
  const available = slots.filter(s => s.status === 'available');
  return (
    <div className="space-y-6">
      <div className="glass-card p-4">
        <p className="font-semibold text-surface-900 dark:text-white">{lot.name}</p>
        <p className="text-sm text-surface-500 dark:text-surface-400 mt-0.5">
          {t('book.availableSlots', { count: available.length, total: slots.length })}
        </p>
      </div>
      <div>
        <h3 className="text-sm font-medium text-surface-700 dark:text-surface-300 mb-3">{t('book.selectSlot')}</h3>
        {loading ? (
          <div className="grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 gap-2">
            {Array.from({ length: 12 }, (_, i) => <div key={i} className="h-12 skeleton rounded-lg" />)}
          </div>
        ) : (
          <div className="grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 gap-2">
            {slots.map(slot => {
              const isAvailable = slot.status === 'available';
              const isSelected = selectedSlot?.id === slot.id;
              const Icon = slot.slot_type ? SLOT_TYPE_ICON[slot.slot_type] : null;
              return (
                <motion.button key={slot.id} disabled={!isAvailable} onClick={() => onSelectSlot(slot)}
                  whileHover={isAvailable ? { scale: 1.08 } : {}} whileTap={isAvailable ? { scale: 0.95 } : {}}
                  aria-pressed={isSelected}
                  aria-label={`${t('book.slot', 'Slot')} ${slot.slot_number}${slot.slot_type ? ` (${slot.slot_type})` : ''} — ${slot.status}`}
                  className={`relative h-12 rounded-lg border text-sm font-medium transition-all ${
                    isSelected ? 'bg-primary-600 border-primary-500 text-white shadow-lg shadow-primary-500/25 glow-primary'
                    : isAvailable ? 'bg-white dark:bg-surface-800 border-surface-200 dark:border-surface-700 text-surface-700 dark:text-surface-300 hover:border-primary-400 dark:hover:border-primary-600 hover:shadow-sm'
                    : 'bg-surface-100 dark:bg-surface-800/50 border-surface-200 dark:border-surface-700 text-surface-400 cursor-not-allowed opacity-50'
                  }`}
                  title={`${slot.slot_number}${slot.slot_type ? ` (${slot.slot_type})` : ''} — ${slot.status}`}
                >
                  {slot.slot_number}
                  {Icon && <Icon weight="bold" className="absolute top-0.5 right-0.5 w-3 h-3 opacity-60" />}
                </motion.button>
              );
            })}
          </div>
        )}
        {!loading && available.length === 0 && <p className="text-sm text-surface-500 dark:text-surface-400 mt-2">{t('book.noAvailableSlots')}</p>}
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label htmlFor="book-start-time" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
            <Clock weight="regular" className="inline w-4 h-4 mr-1" aria-hidden="true" />{t('book.startTime')}
          </label>
          <input id="book-start-time" type="datetime-local" value={startDate} onChange={e => onStartDateChange(e.target.value)} className="input text-sm" />
        </div>
        <div>
          <span id="duration-label" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t('book.duration')}</span>
          <div className="flex gap-2" role="group" aria-labelledby="duration-label">
            {DURATIONS.map(d => (
              <motion.button key={d.hours} onClick={() => onDurationChange(d.hours)}
                whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }} aria-pressed={duration === d.hours}
                className={`flex-1 py-2 rounded-lg border text-sm font-medium transition-all ${
                  duration === d.hours ? 'bg-teal-600 border-teal-600 text-white shadow-md shadow-teal-500/20'
                  : 'bg-white dark:bg-surface-800 border-surface-200 dark:border-surface-700 text-surface-700 dark:text-surface-300 hover:border-teal-400'
                }`}
              >{d.label}</motion.button>
            ))}
          </div>
        </div>
      </div>
      {vehicles.length > 0 && (
        <div>
          <label htmlFor="book-vehicle" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
            <Car weight="regular" className="inline w-4 h-4 mr-1" aria-hidden="true" />{t('book.vehicle')}
          </label>
          <select id="book-vehicle" value={selectedVehicle} onChange={e => onVehicleChange(e.target.value)} className="input text-sm">
            <option value="">{t('book.noVehicle')}</option>
            {vehicles.map(v => <option key={v.id} value={v.id}>{v.plate}{v.make ? ` — ${v.make} ${v.model || ''}` : ''}</option>)}
          </select>
        </div>
      )}
      <motion.button onClick={onContinue} disabled={!selectedSlot}
        whileHover={selectedSlot ? { scale: 1.02 } : {}} whileTap={selectedSlot ? { scale: 0.98 } : {}}
        className="btn btn-primary w-full sm:w-auto shadow-lg shadow-primary-500/15"
      >{t('book.continue')}</motion.button>
    </div>
  );
}

function StepConfirm({ lot, slot, start, end, duration, estimatedCost, vehicle, submitting, onConfirm, t }: {
  lot: ParkingLot; slot: ParkingSlot; start: Date; end: Date; duration: number;
  estimatedCost: string | null; vehicle?: Vehicle; submitting: boolean; onConfirm: () => void; t: TFunction;
}) {
  const fmt = (d: Date) => d.toLocaleString(undefined, { weekday: 'short', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  return (
    <div className="space-y-6 max-w-lg">
      <div className="glass-card divide-y divide-surface-100 dark:divide-surface-800" role="region" aria-label={t('book.summary', 'Booking summary')}>
        <SummaryRow label={t('book.lot')} value={lot.name} />
        <SummaryRow label={t('book.slot')} value={slot.slot_number} />
        <SummaryRow label={t('book.from')} value={fmt(start)} />
        <SummaryRow label={t('book.to')} value={fmt(end)} />
        <SummaryRow label={t('book.duration')} value={`${duration}h`} />
        {vehicle && <SummaryRow label={t('book.vehicle')} value={`${vehicle.plate}${vehicle.make ? ` (${vehicle.make})` : ''}`} />}
        {estimatedCost && <SummaryRow label={t('book.estimatedCost')} value={`${lot.currency || '€'}${estimatedCost}`} bold />}
      </div>
      <motion.button onClick={onConfirm} disabled={submitting}
        whileHover={!submitting ? { scale: 1.02 } : {}} whileTap={!submitting ? { scale: 0.98 } : {}}
        className={`btn btn-primary w-full sm:w-auto shadow-lg shadow-primary-500/15 ${submitting ? 'btn-shimmer' : ''}`}
      >
        {submitting ? <><SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> {t('book.confirming')}</>
          : <><Check weight="bold" className="w-4 h-4" /> {t('book.confirm')}</>}
      </motion.button>
    </div>
  );
}

function SummaryRow({ label, value, bold }: { label: string; value: string; bold?: boolean }) {
  return (
    <div className="flex items-center justify-between px-5 py-3">
      <span className="text-sm text-surface-500 dark:text-surface-400">{label}</span>
      <span className={`text-sm ${bold ? 'font-bold text-surface-900 dark:text-white' : 'font-medium text-surface-900 dark:text-white'}`}>{value}</span>
    </div>
  );
}

function ConfettiOverlay() {
  const colors = ['#14b8a6', '#f59e0b', '#6366f1', '#ec4899', '#22c55e', '#3b82f6'];
  const pieces = Array.from({ length: 24 }, (_, i) => ({
    id: i, color: colors[i % colors.length], left: `${Math.random() * 100}%`,
    delay: Math.random() * 0.5, duration: 1 + Math.random() * 1.5, size: 4 + Math.random() * 6,
  }));
  return (
    <div className="fixed inset-0 pointer-events-none z-50 overflow-hidden" aria-hidden="true">
      {pieces.map(p => (
        <motion.div key={p.id} className="absolute rounded-sm"
          style={{ left: p.left, top: -10, width: p.size, height: p.size, backgroundColor: p.color }}
          initial={{ y: -20, rotate: 0, opacity: 1 }}
          animate={{ y: '100vh', rotate: 720, opacity: 0 }}
          transition={{ duration: p.duration, delay: p.delay, ease: 'easeIn' }}
        />
      ))}
    </div>
  );
}
