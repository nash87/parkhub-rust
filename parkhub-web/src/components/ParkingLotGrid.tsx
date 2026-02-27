import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Car, Prohibit, Lock } from '@phosphor-icons/react';
import type { LotLayout, LotRow, SlotConfig } from '../api/client';

interface ParkingLotGridProps {
  layout: LotLayout;
  selectedSlotId?: string;
  onSlotSelect?: (slot: SlotConfig) => void;
  interactive?: boolean;
}

const statusColors: Record<SlotConfig['status'], { bg: string; border: string; text: string; icon?: string }> = {
  available: {
    bg: 'bg-emerald-100 dark:bg-emerald-900/40',
    border: 'border-emerald-300 dark:border-emerald-700',
    text: 'text-emerald-700 dark:text-emerald-300',
  },
  occupied: {
    bg: 'bg-red-100 dark:bg-red-900/40',
    border: 'border-red-300 dark:border-red-700',
    text: 'text-red-700 dark:text-red-300',
  },
  reserved: {
    bg: 'bg-amber-100 dark:bg-amber-900/40',
    border: 'border-amber-300 dark:border-amber-700',
    text: 'text-amber-700 dark:text-amber-300',
  },
  disabled: {
    bg: 'bg-gray-100 dark:bg-gray-800',
    border: 'border-gray-300 dark:border-gray-600 border-dashed',
    text: 'text-gray-400 dark:text-gray-500',
  },
  blocked: {
    bg: 'bg-gray-200 dark:bg-gray-700',
    border: 'border-gray-400 dark:border-gray-500',
    text: 'text-gray-500 dark:text-gray-400',
  },
};

function SlotBox({
  slot,
  side,
  selected,
  interactive,
  onSelect,
}: {
  slot: SlotConfig;
  side: 'top' | 'bottom';
  selected: boolean;
  interactive: boolean;
  onSelect?: (slot: SlotConfig) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const colors = statusColors[slot.status];
  const clickable = interactive && (slot.status === 'available' || slot.status === 'reserved');

  return (
    <motion.div
      className="relative flex-shrink-0"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      whileHover={clickable ? { scale: 1.05 } : {}}
      whileTap={clickable ? { scale: 0.97 } : {}}
    >
      <button
        disabled={!clickable}
        onClick={() => clickable && onSelect?.(slot)}
        className={`
          w-16 h-20 sm:w-20 sm:h-24 rounded-lg border-2 flex flex-col items-center justify-center gap-1 transition-all
          ${colors.bg} ${colors.border} ${colors.text}
          ${clickable ? 'cursor-pointer hover:shadow-lg' : 'cursor-default'}
          ${selected ? 'ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-gray-900' : ''}
          ${slot.status === 'disabled' ? 'opacity-60' : ''}
        `}
      >
        {/* Car icon pointing toward road */}
        {slot.status === 'occupied' && (
          <Car
            weight="fill"
            className={`w-5 h-5 sm:w-6 sm:h-6 ${side === 'top' ? 'rotate-180' : ''}`}
          />
        )}
        {slot.status === 'disabled' && <Prohibit weight="bold" className="w-4 h-4" />}
        {slot.status === 'blocked' && <Lock weight="fill" className="w-4 h-4" />}
        {(slot.status === 'available' || slot.status === 'reserved') && (
          <Car
            weight="regular"
            className={`w-5 h-5 sm:w-6 sm:h-6 opacity-30 ${side === 'top' ? 'rotate-180' : ''}`}
          />
        )}
        <span className="text-xs sm:text-sm font-bold">{slot.number}</span>
      </button>

      {/* Tooltip */}
      <AnimatePresence>
        {hovered && slot.vehiclePlate && (
          <motion.div
            initial={{ opacity: 0, y: side === 'top' ? 4 : -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className={`absolute z-10 left-1/2 -translate-x-1/2 px-2 py-1 rounded-md bg-gray-900 dark:bg-gray-100 text-white dark:text-gray-900 text-xs font-mono whitespace-nowrap shadow-lg
              ${side === 'top' ? 'top-full mt-1' : 'bottom-full mb-1'}
            `}
          >
            {slot.vehiclePlate}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

function RowSlots({
  row,
  selectedSlotId,
  interactive,
  onSlotSelect,
}: {
  row: LotRow;
  selectedSlotId?: string;
  interactive: boolean;
  onSlotSelect?: (slot: SlotConfig) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      {row.label && (
        <span className="text-xs font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider px-1">
          {row.label}
        </span>
      )}
      <div className="flex gap-1.5 sm:gap-2">
        {row.slots.map((slot) => (
          <SlotBox
            key={slot.id}
            slot={slot}
            side={row.side}
            selected={slot.id === selectedSlotId}
            interactive={interactive}
            onSelect={onSlotSelect}
          />
        ))}
      </div>
    </div>
  );
}

export function ParkingLotGrid({
  layout,
  selectedSlotId,
  onSlotSelect,
  interactive = false,
}: ParkingLotGridProps) {
  const topRows = layout.rows.filter((r) => r.side === 'top');
  const bottomRows = layout.rows.filter((r) => r.side === 'bottom');

  return (
    <div className="space-y-3">
      {/* Grid */}
      <div className="overflow-x-auto pb-2">
        <div className="inline-flex flex-col gap-0 min-w-fit">
          {/* Top rows */}
          {topRows.map((row) => (
            <RowSlots
              key={row.id}
              row={row}
              selectedSlotId={selectedSlotId}
              interactive={interactive}
              onSlotSelect={onSlotSelect}
            />
          ))}

          {/* Road */}
          <div className="flex items-center gap-3 my-3">
            <div className="flex-1 border-t-2 border-dashed border-gray-300 dark:border-gray-600" />
            <span className="text-xs font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-widest">
              {layout.roadLabel || 'Fahrweg'}
            </span>
            <div className="flex-1 border-t-2 border-dashed border-gray-300 dark:border-gray-600" />
          </div>

          {/* Bottom rows */}
          {bottomRows.map((row) => (
            <RowSlots
              key={row.id}
              row={row}
              selectedSlotId={selectedSlotId}
              interactive={interactive}
              onSlotSelect={onSlotSelect}
            />
          ))}
        </div>
      </div>

      {/* Legend */}
      <div className="flex flex-wrap gap-4 text-xs text-gray-500 dark:text-gray-400 pt-2 border-t border-gray-100 dark:border-gray-800">
        {[
          { status: 'available' as const, label: 'Frei' },
          { status: 'occupied' as const, label: 'Belegt' },
          { status: 'reserved' as const, label: 'Reserviert' },
          { status: 'disabled' as const, label: 'Gesperrt' },
        ].map(({ status, label }) => (
          <div key={status} className="flex items-center gap-1.5">
            <div
              className={`w-3 h-3 rounded-sm border ${statusColors[status].bg} ${statusColors[status].border}`}
            />
            <span>{label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
