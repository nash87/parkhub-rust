import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { WebhooksLogo, Plus, Trash, Pencil, Question, PaperPlaneTilt, ListChecks } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

const EVENTS = [
  'booking.created',
  'booking.cancelled',
  'user.registered',
  'lot.full',
  'payment.completed',
];

interface WebhookV2 {
  id: string;
  url: string;
  secret: string;
  events: string[];
  active: boolean;
  description: string | null;
  created_at: string;
  updated_at: string;
}

interface Delivery {
  id: string;
  event_type: string;
  status_code: number | null;
  success: boolean;
  attempt: number;
  error: string | null;
  delivered_at: string;
}

export function AdminWebhooksPage() {
  const { t } = useTranslation();
  const [webhooks, setWebhooks] = useState<WebhookV2[]>([]);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [formUrl, setFormUrl] = useState('');
  const [formEvents, setFormEvents] = useState<string[]>([]);
  const [formDesc, setFormDesc] = useState('');
  const [deliveries, setDeliveries] = useState<Delivery[]>([]);
  const [showDeliveries, setShowDeliveries] = useState<string | null>(null);

  const loadWebhooks = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/admin/webhooks-v2').then(r => r.json());
      if (res.success) {
        setWebhooks(res.data || []);
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    loadWebhooks();
  }, [loadWebhooks]);

  function resetForm() {
    setFormUrl('');
    setFormEvents([]);
    setFormDesc('');
    setEditId(null);
    setShowForm(false);
  }

  function toggleEvent(ev: string) {
    setFormEvents(prev => prev.includes(ev) ? prev.filter(e => e !== ev) : [...prev, ev]);
  }

  async function handleSave() {
    if (!formUrl.trim() || formEvents.length === 0) {
      toast.error(t('webhooksV2.requiredFields'));
      return;
    }

    try {
      const method = editId ? 'PUT' : 'POST';
      const url = editId ? `/api/v1/admin/webhooks-v2/${editId}` : '/api/v1/admin/webhooks-v2';
      const res = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          url: formUrl.trim(),
          events: formEvents,
          description: formDesc.trim() || null,
          active: true,
        }),
      }).then(r => r.json());

      if (res.success) {
        toast.success(editId ? t('webhooksV2.updated') : t('webhooksV2.created'));
        resetForm();
        loadWebhooks();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  async function handleDelete(id: string) {
    try {
      const res = await fetch(`/api/v1/admin/webhooks-v2/${id}`, { method: 'DELETE' }).then(r => r.json());
      if (res.success) {
        toast.success(t('webhooksV2.deleted'));
        loadWebhooks();
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  async function handleTest(id: string) {
    try {
      const res = await fetch(`/api/v1/admin/webhooks-v2/${id}/test`, { method: 'POST' }).then(r => r.json());
      if (res.success) {
        toast.success(res.data?.success ? t('webhooksV2.testSuccess') : t('webhooksV2.testFailed'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  async function loadDeliveries(id: string) {
    try {
      const res = await fetch(`/api/v1/admin/webhooks-v2/${id}/deliveries`).then(r => r.json());
      if (res.success) {
        setDeliveries(res.data || []);
        setShowDeliveries(id);
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  return (
    <div className="space-y-6 max-w-4xl">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-surface-900 dark:text-surface-100 flex items-center gap-2">
            <WebhooksLogo size={24} weight="bold" className="text-primary-500" />
            {t('webhooksV2.title')}
          </h2>
          <p className="text-sm text-surface-500 mt-1">{t('webhooksV2.subtitle')}</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => setShowHelp(!showHelp)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-500" aria-label={t('webhooksV2.helpLabel')}>
            <Question size={20} />
          </button>
          <button onClick={() => { resetForm(); setShowForm(true); }} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary-500 text-white hover:bg-primary-600 text-sm font-medium">
            <Plus size={16} weight="bold" />
            {t('webhooksV2.create')}
          </button>
        </div>
      </div>

      <AnimatePresence>
        {showHelp && (
          <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }} className="bg-primary-50 dark:bg-primary-950/30 border border-primary-200 dark:border-primary-800 rounded-lg p-4 text-sm text-surface-700 dark:text-surface-300">
            {t('webhooksV2.help')}
          </motion.div>
        )}
      </AnimatePresence>

      {/* Form */}
      <AnimatePresence>
        {showForm && (
          <motion.div initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-6 space-y-4">
            <h3 className="font-semibold text-surface-900 dark:text-surface-100">
              {editId ? t('webhooksV2.editWebhook') : t('webhooksV2.newWebhook')}
            </h3>

            <div>
              <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('webhooksV2.url')}</label>
              <input type="text" value={formUrl} onChange={e => setFormUrl(e.target.value)} placeholder="https://example.com/webhook" className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm" />
            </div>

            <div>
              <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('webhooksV2.events')}</label>
              <div className="flex flex-wrap gap-2">
                {EVENTS.map(ev => (
                  <button key={ev} onClick={() => toggleEvent(ev)} className={`px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors ${formEvents.includes(ev) ? 'bg-primary-500 text-white border-primary-500' : 'bg-white dark:bg-surface-900 text-surface-600 border-surface-300 dark:border-surface-600'}`}>
                    {ev}
                  </button>
                ))}
              </div>
            </div>

            <div>
              <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('webhooksV2.description')}</label>
              <input type="text" value={formDesc} onChange={e => setFormDesc(e.target.value)} placeholder={t('webhooksV2.descriptionPlaceholder')} className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm" />
            </div>

            <div className="flex justify-end gap-2 pt-2">
              <button onClick={resetForm} className="px-4 py-2 rounded-lg text-sm text-surface-600 hover:bg-surface-100 dark:hover:bg-surface-700">{t('common.cancel')}</button>
              <button onClick={handleSave} className="px-4 py-2 rounded-lg text-sm font-medium bg-primary-500 text-white hover:bg-primary-600">{t('webhooksV2.save')}</button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* List */}
      {loading ? (
        <div className="text-center py-8 text-surface-400">{t('common.loading')}</div>
      ) : webhooks.length === 0 ? (
        <div className="text-center py-12 text-surface-400">
          <WebhooksLogo size={48} className="mx-auto mb-3 opacity-30" />
          <p>{t('webhooksV2.empty')}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {webhooks.map(wh => (
            <motion.div key={wh.id} initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium text-surface-900 dark:text-surface-100 text-sm">{wh.url}</p>
                  <p className="text-xs text-surface-500 mt-1">{wh.events.join(', ')}</p>
                  {wh.description && <p className="text-xs text-surface-400 mt-0.5">{wh.description}</p>}
                </div>
                <div className="flex items-center gap-1">
                  <button onClick={() => handleTest(wh.id)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 text-surface-500" aria-label={t('webhooksV2.test')}>
                    <PaperPlaneTilt size={16} />
                  </button>
                  <button onClick={() => loadDeliveries(wh.id)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 text-surface-500" aria-label={t('webhooksV2.deliveries')}>
                    <ListChecks size={16} />
                  </button>
                  <button onClick={() => { setEditId(wh.id); setFormUrl(wh.url); setFormEvents(wh.events); setFormDesc(wh.description || ''); setShowForm(true); }} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 text-surface-500" aria-label={t('webhooksV2.edit')}>
                    <Pencil size={16} />
                  </button>
                  <button onClick={() => handleDelete(wh.id)} className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-950/30 text-red-500" aria-label={t('webhooksV2.delete')}>
                    <Trash size={16} />
                  </button>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      )}

      {/* Delivery log */}
      <AnimatePresence>
        {showDeliveries && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-semibold text-surface-900 dark:text-surface-100">{t('webhooksV2.deliveryLog')}</h3>
              <button onClick={() => setShowDeliveries(null)} className="text-xs text-surface-500 hover:text-surface-700">{t('common.close')}</button>
            </div>
            {deliveries.length === 0 ? (
              <p className="text-sm text-surface-400">{t('webhooksV2.noDeliveries')}</p>
            ) : (
              <div className="space-y-2 max-h-64 overflow-y-auto">
                {deliveries.map(d => (
                  <div key={d.id} className={`flex items-center justify-between p-2 rounded-lg text-xs ${d.success ? 'bg-green-50 dark:bg-green-950/20' : 'bg-red-50 dark:bg-red-950/20'}`}>
                    <span className="font-medium">{d.event_type}</span>
                    <span>{d.status_code || 'N/A'} &middot; {t('webhooksV2.attempt')} {d.attempt}</span>
                    <span className="text-surface-400">{new Date(d.delivered_at).toLocaleString()}</span>
                  </div>
                ))}
              </div>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
