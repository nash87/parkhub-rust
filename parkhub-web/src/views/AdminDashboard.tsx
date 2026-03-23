import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ChartBar, CurrencyCircleDollar, CalendarCheck, UsersThree, Fire, Warning,
  Wrench, Lightning, Plus, Minus, GearSix, Question, ArrowsOutCardinal,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { api } from '../api/client';

interface WidgetPosition { x: number; y: number; w: number; h: number; }
interface WidgetEntry { id: string; widget_type: string; position: WidgetPosition; visible: boolean; }
interface WidgetLayout { user_id: string; widgets: WidgetEntry[]; }

const WIDGET_TYPES = [
  'occupancy_chart', 'revenue_summary', 'recent_bookings', 'user_growth',
  'booking_heatmap', 'active_alerts', 'maintenance_status', 'ev_charging_status',
] as const;

const widgetIcons: Record<string, React.ComponentType<any>> = {
  occupancy_chart: ChartBar,
  revenue_summary: CurrencyCircleDollar,
  recent_bookings: CalendarCheck,
  user_growth: UsersThree,
  booking_heatmap: Fire,
  active_alerts: Warning,
  maintenance_status: Wrench,
  ev_charging_status: Lightning,
};

export function AdminDashboardPage() {
  const { t } = useTranslation();
  const [layout, setLayout] = useState<WidgetLayout | null>(null);
  const [loading, setLoading] = useState(true);
  const [showCatalog, setShowCatalog] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [widgetData, setWidgetData] = useState<Record<string, any>>({});
  const [, setSaving] = useState(false);

  const loadLayout = useCallback(async () => {
    try {
      const res = await api.getWidgetLayout();
      if (res.success && res.data) setLayout(res.data);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadLayout(); }, [loadLayout]);

  // Load data for visible widgets
  useEffect(() => {
    if (!layout) return;
    for (const w of layout.widgets) {
      if (w.visible && !widgetData[w.widget_type]) {
        api.getWidgetData(w.widget_type).then(res => {
          if (res.success && res.data) {
            setWidgetData(prev => ({ ...prev, [w.widget_type]: res.data }));
          }
        }).catch(() => {});
      }
    }
  }, [layout, widgetData]);

  async function saveLayout(widgets: WidgetEntry[]) {
    setSaving(true);
    try {
      const res = await api.saveWidgetLayout(widgets);
      if (res.success) {
        setLayout(res.data!);
        toast.success(t('widgets.layoutSaved'));
      } else {
        toast.error(t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
    setSaving(false);
  }

  function toggleWidget(widgetType: string) {
    if (!layout) return;
    const existing = layout.widgets.find(w => w.widget_type === widgetType);
    let newWidgets: WidgetEntry[];
    if (existing) {
      newWidgets = layout.widgets.map(w =>
        w.widget_type === widgetType ? { ...w, visible: !w.visible } : w
      );
    } else {
      newWidgets = [...layout.widgets, {
        id: `w${layout.widgets.length + 1}`,
        widget_type: widgetType,
        position: { x: 0, y: (layout.widgets.length * 4), w: 6, h: 4 },
        visible: true,
      }];
    }
    saveLayout(newWidgets);
  }

  const visibleWidgets = layout?.widgets.filter(w => w.visible) ?? [];

  if (loading) return (
    <div className="space-y-4">
      <div className="h-8 w-48 skeleton rounded-lg" />
      <div className="grid grid-cols-2 gap-4">
        {[1,2,3,4].map(i => <div key={i} className="h-40 skeleton rounded-2xl" />)}
      </div>
    </div>
  );

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('widgets.title')}</h1>
          <p className="text-surface-500 dark:text-surface-400 text-sm">{t('widgets.subtitle')}</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => setShowHelp(!showHelp)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-500" title={t('widgets.helpLabel')}>
            <Question size={20} />
          </button>
          <button onClick={() => setShowCatalog(!showCatalog)} className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-primary-600 text-white text-sm font-medium hover:bg-primary-700">
            <GearSix size={16} /> {t('widgets.customize')}
          </button>
        </div>
      </div>

      {/* Help tooltip */}
      <AnimatePresence>
        {showHelp && (
          <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }}
            className="p-3 rounded-lg bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 text-sm text-primary-800 dark:text-primary-300 flex items-start gap-2">
            <ArrowsOutCardinal size={18} className="mt-0.5 shrink-0" />
            <span>{t('widgets.help')}</span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Widget catalog sidebar */}
      <AnimatePresence>
        {showCatalog && (
          <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }}
            className="bg-surface-50 dark:bg-surface-800 rounded-xl p-4 border border-surface-200 dark:border-surface-700">
            <h3 className="font-semibold text-surface-900 dark:text-surface-100 mb-3">{t('widgets.catalog')}</h3>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {WIDGET_TYPES.map(wt => {
                const Icon = widgetIcons[wt] || ChartBar;
                const isActive = layout?.widgets.some(w => w.widget_type === wt && w.visible);
                return (
                  <button key={wt} onClick={() => toggleWidget(wt)}
                    className={`flex items-center gap-2 p-3 rounded-lg border text-sm transition-colors ${
                      isActive
                        ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-300'
                        : 'border-surface-200 dark:border-surface-700 text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-700'
                    }`}>
                    <Icon size={18} />
                    <span className="truncate">{t(`widgets.types.${wt}`)}</span>
                    {isActive ? <Minus size={14} className="ml-auto shrink-0" /> : <Plus size={14} className="ml-auto shrink-0" />}
                  </button>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Widget grid */}
      {visibleWidgets.length === 0 ? (
        <div className="text-center py-12 text-surface-500 dark:text-surface-400">
          <GearSix size={40} className="mx-auto mb-3 opacity-40" />
          <p className="text-lg font-medium mb-1">{t('widgets.empty')}</p>
          <p className="text-sm">{t('widgets.emptyHint')}</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {visibleWidgets.map(w => {
            const Icon = widgetIcons[w.widget_type] || ChartBar;
            const data = widgetData[w.widget_type];
            return (
              <motion.div key={w.id} initial={{ opacity: 0, scale: 0.95 }} animate={{ opacity: 1, scale: 1 }}
                className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-4 hover:shadow-md transition-shadow">
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <div className="p-2 rounded-lg bg-primary-50 dark:bg-primary-900/20">
                      <Icon size={18} className="text-primary-600 dark:text-primary-400" />
                    </div>
                    <h3 className="font-semibold text-surface-900 dark:text-white text-sm">{t(`widgets.types.${w.widget_type}`)}</h3>
                  </div>
                  <button onClick={() => toggleWidget(w.widget_type)} className="p-1 rounded hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400" title={t('widgets.remove')}>
                    <Minus size={14} />
                  </button>
                </div>
                <div className="text-sm text-surface-600 dark:text-surface-400">
                  {data?.data ? (
                    <pre className="text-xs overflow-auto max-h-24">{JSON.stringify(data.data, null, 2)}</pre>
                  ) : (
                    <div className="h-16 skeleton rounded-lg" />
                  )}
                </div>
              </motion.div>
            );
          })}
        </div>
      )}
    </div>
  );
}
