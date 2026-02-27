import { useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Plus,
  Trash,
  Copy,
  ArrowUp,
  ArrowDown,
  FloppyDisk,
  Eye,
  Prohibit,
} from '@phosphor-icons/react';
import type { LotLayout, LotRow, SlotConfig } from '../api/client';
import { ParkingLotGrid } from './ParkingLotGrid';

interface LotLayoutEditorProps {
  initialLayout?: LotLayout;
  lotName?: string;
  onSave?: (layout: LotLayout, name: string) => void;
  onCancel?: () => void;
}

function generateId() {
  return `row-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
}

function buildSlots(count: number, startNum: number, existing?: SlotConfig[]): SlotConfig[] {
  return Array.from({ length: count }, (_, i) => {
    const ex = existing?.[i];
    return {
      id: ex?.id || `slot-${Date.now()}-${i}`,
      number: String(startNum + i),
      status: ex?.status || 'available',
    };
  });
}

interface RowEditorConfig {
  id: string;
  side: 'top' | 'bottom';
  label: string;
  slotCount: number;
  startNumber: number;
  slots: SlotConfig[];
}

function rowConfigToRow(cfg: RowEditorConfig): LotRow {
  return {
    id: cfg.id,
    side: cfg.side,
    label: cfg.label || undefined,
    slots: cfg.slots,
  };
}

export function LotLayoutEditor({ initialLayout, lotName: initName, onSave, onCancel }: LotLayoutEditorProps) {
  const [name, setName] = useState(initName || '');
  const [roadLabel, setRoadLabel] = useState(initialLayout?.roadLabel || 'Fahrweg');
  const [showPreview, setShowPreview] = useState(true);

  const initRows: RowEditorConfig[] = initialLayout?.rows.map((r) => ({
    id: r.id,
    side: r.side,
    label: r.label || '',
    slotCount: r.slots.length,
    startNumber: r.slots.length > 0 ? parseInt(r.slots[0].number) || 1 : 1,
    slots: r.slots,
  })) || [];

  const [rows, setRows] = useState<RowEditorConfig[]>(initRows);

  const updateRow = useCallback((idx: number, partial: Partial<RowEditorConfig>) => {
    setRows((prev) => {
      const next = [...prev];
      const row = { ...next[idx], ...partial };
      // Rebuild slots if count or startNumber changed
      if ('slotCount' in partial || 'startNumber' in partial) {
        row.slots = buildSlots(row.slotCount, row.startNumber, row.slots);
      }
      next[idx] = row;
      return next;
    });
  }, []);

  const addRow = (side: 'top' | 'bottom') => {
    setRows((prev) => [
      ...prev,
      {
        id: generateId(),
        side,
        label: '',
        slotCount: 6,
        startNumber: 1,
        slots: buildSlots(6, 1),
      },
    ]);
  };

  const removeRow = (idx: number) => setRows((prev) => prev.filter((_, i) => i !== idx));

  const duplicateRow = (idx: number) => {
    setRows((prev) => {
      const src = prev[idx];
      const dup: RowEditorConfig = {
        ...src,
        id: generateId(),
        label: src.label + ' (Kopie)',
        slots: buildSlots(src.slotCount, src.startNumber),
      };
      const next = [...prev];
      next.splice(idx + 1, 0, dup);
      return next;
    });
  };

  const moveRow = (idx: number, dir: -1 | 1) => {
    setRows((prev) => {
      const next = [...prev];
      const target = idx + dir;
      if (target < 0 || target >= next.length) return prev;
      [next[idx], next[target]] = [next[target], next[idx]];
      return next;
    });
  };

  const toggleSlotStatus = (rowIdx: number, slotIdx: number) => {
    setRows((prev) => {
      const next = [...prev];
      const row = { ...next[rowIdx] };
      const slots = [...row.slots];
      const slot = { ...slots[slotIdx] };
      const cycle: SlotConfig['status'][] = ['available', 'disabled', 'blocked'];
      const ci = cycle.indexOf(slot.status);
      slot.status = cycle[(ci + 1) % cycle.length];
      slots[slotIdx] = slot;
      row.slots = slots;
      next[rowIdx] = row;
      return next;
    });
  };

  const currentLayout: LotLayout = {
    roadLabel,
    rows: rows.map(rowConfigToRow),
  };

  const handleSave = () => {
    console.log('Saving layout:', { name, layout: currentLayout });
    onSave?.(currentLayout, name);
  };

  return (
    <div className="space-y-6">
      {/* Name & Road Label */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Parkplatz-Name
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="z.B. Firmenparkplatz"
            className="input"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Fahrweg-Beschriftung
          </label>
          <input
            type="text"
            value={roadLabel}
            onChange={(e) => setRoadLabel(e.target.value)}
            className="input"
          />
        </div>
      </div>

      {/* Rows */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300">Reihen</h3>
          <div className="flex gap-2">
            <button onClick={() => addRow('top')} className="btn btn-secondary btn-sm">
              <Plus weight="bold" className="w-3.5 h-3.5" />
              Obere Reihe
            </button>
            <button onClick={() => addRow('bottom')} className="btn btn-secondary btn-sm">
              <Plus weight="bold" className="w-3.5 h-3.5" />
              Untere Reihe
            </button>
          </div>
        </div>

        <AnimatePresence>
          {rows.map((row, idx) => (
            <motion.div
              key={row.id}
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="overflow-hidden"
            >
              <div className="p-4 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700 space-y-3">
                {/* Row header */}
                <div className="flex items-center justify-between gap-2 flex-wrap">
                  <div className="flex items-center gap-2">
                    <span className={`text-xs font-bold uppercase px-2 py-0.5 rounded ${
                      row.side === 'top'
                        ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
                        : 'bg-purple-100 text-purple-700 dark:bg-purple-900/40 dark:text-purple-300'
                    }`}>
                      {row.side === 'top' ? 'Oben' : 'Unten'}
                    </span>
                    <input
                      type="text"
                      value={row.label}
                      onChange={(e) => updateRow(idx, { label: e.target.value })}
                      placeholder="Reihe A"
                      className="input !py-1 !px-2 !text-sm w-32"
                    />
                  </div>
                  <div className="flex items-center gap-1">
                    <button onClick={() => moveRow(idx, -1)} className="p-1.5 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500" title="Nach oben">
                      <ArrowUp weight="bold" className="w-4 h-4" />
                    </button>
                    <button onClick={() => moveRow(idx, 1)} className="p-1.5 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500" title="Nach unten">
                      <ArrowDown weight="bold" className="w-4 h-4" />
                    </button>
                    <button onClick={() => duplicateRow(idx)} className="p-1.5 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500" title="Duplizieren">
                      <Copy weight="bold" className="w-4 h-4" />
                    </button>
                    <button onClick={() => removeRow(idx)} className="p-1.5 rounded-lg hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500" title="Löschen">
                      <Trash weight="bold" className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                {/* Slot config */}
                <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                  <div>
                    <label className="text-xs text-gray-500 dark:text-gray-400">Anzahl Plätze</label>
                    <input
                      type="number"
                      min={1}
                      max={50}
                      value={row.slotCount}
                      onChange={(e) => updateRow(idx, { slotCount: Math.max(1, parseInt(e.target.value) || 1) })}
                      className="input !py-1 !text-sm"
                    />
                  </div>
                  <div>
                    <label className="text-xs text-gray-500 dark:text-gray-400">Startnummer</label>
                    <input
                      type="number"
                      min={1}
                      value={row.startNumber}
                      onChange={(e) => updateRow(idx, { startNumber: Math.max(1, parseInt(e.target.value) || 1) })}
                      className="input !py-1 !text-sm"
                    />
                  </div>
                  <div className="col-span-2 sm:col-span-1 flex items-end">
                    <span className="text-xs text-gray-400 dark:text-gray-500 pb-2">
                      Start: {row.startNumber}, Ende: {row.startNumber + row.slotCount - 1} ({row.slotCount} Plätze)
                    </span>
                  </div>
                </div>

                {/* Individual slot overrides */}
                <div>
                  <label className="text-xs text-gray-500 dark:text-gray-400 mb-1 block">
                    Plätze (Klick = Status ändern)
                  </label>
                  <div className="flex flex-wrap gap-1">
                    {row.slots.map((slot, si) => (
                      <button
                        key={slot.id}
                        onClick={() => toggleSlotStatus(idx, si)}
                        className={`w-9 h-9 rounded-md text-xs font-bold border flex items-center justify-center transition-colors
                          ${slot.status === 'available' ? 'bg-emerald-100 border-emerald-300 text-emerald-700 dark:bg-emerald-900/40 dark:border-emerald-700 dark:text-emerald-300' : ''}
                          ${slot.status === 'disabled' ? 'bg-gray-200 border-gray-300 text-gray-400 dark:bg-gray-700 dark:border-gray-600 dark:text-gray-500' : ''}
                          ${slot.status === 'blocked' ? 'bg-red-100 border-red-300 text-red-500 dark:bg-red-900/40 dark:border-red-700 dark:text-red-400' : ''}
                        `}
                        title={`Platz ${slot.number}: ${slot.status}`}
                      >
                        {slot.status === 'disabled' ? <Prohibit weight="bold" className="w-3 h-3" /> : slot.number}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            </motion.div>
          ))}
        </AnimatePresence>

        {rows.length === 0 && (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500 text-sm">
            Noch keine Reihen angelegt. Fügen Sie oben eine Reihe hinzu.
          </div>
        )}
      </div>

      {/* Preview toggle */}
      {rows.length > 0 && (
        <div>
          <button
            onClick={() => setShowPreview((p) => !p)}
            className="btn btn-secondary btn-sm mb-3"
          >
            <Eye weight="bold" className="w-4 h-4" />
            {showPreview ? 'Vorschau ausblenden' : 'Vorschau anzeigen'}
          </button>

          <AnimatePresence>
            {showPreview && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                exit={{ opacity: 0, height: 0 }}
                className="overflow-hidden"
              >
                <div className="card p-4">
                  <ParkingLotGrid layout={currentLayout} interactive={false} />
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}

      {/* Actions */}
      <div className="flex items-center justify-end gap-3 pt-2 border-t border-gray-200 dark:border-gray-700">
        {onCancel && (
          <button onClick={onCancel} className="btn btn-secondary">
            Abbrechen
          </button>
        )}
        <button onClick={handleSave} className="btn btn-primary">
          <FloppyDisk weight="bold" className="w-4 h-4" />
          Speichern
        </button>
      </div>
    </div>
  );
}
