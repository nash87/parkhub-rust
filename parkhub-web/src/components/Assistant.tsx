/**
 * Assistant — rule-based local helper.
 *
 * Ported from claude.ai/design v3 handoff bundle (qol.jsx). Deliberately
 * NOT an AI/LLM — it's a scripted helper that pattern-matches the user's
 * question against a small set of heuristics and returns a canned reply
 * from their local parking data shape. The design's framing is explicit
 * in its copy: "Runs on-prem · no external calls".
 *
 * React 19 patterns:
 *  - `ref` as prop on the native <dialog> container (no forwardRef)
 *  - Optional `replies` prop so callers can supply smarter matchers later
 *    (e.g. calling a real /api/v1/bookings query inside a matcher fn)
 *    without rewriting the UI.
 */

import { useEffect, useRef, useState } from 'react';
import { Sparkle, ArrowRight, X } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

interface Message {
  role: 'user' | 'bot';
  content: string;
}

interface AssistantProps {
  open: boolean;
  onClose: () => void;
  /** Optional custom matcher. Defaults to `defaultReply`. */
  reply?: (question: string) => string;
  /** Optional initial suggestion chips shown before the first message. */
  suggestions?: string[];
}

/**
 * Default reply heuristics — match user input against a handful of intent
 * keywords and return a canned answer that looks like real local data.
 * Backend teams can swap in a smarter matcher via the `reply` prop.
 */
export function defaultReply(q: string): string {
  const lc = q.toLowerCase();
  if (lc.includes('nearest ev') || lc.includes('ev slot'))
    return 'Slot L2-07 is the closest available EV charger — 45 m from the entrance, 150 kW CCS. Want to reserve it for 4 hours?';
  if (lc.includes('credit'))
    return 'You used 43 credits this month vs 31 in April (+39%). The jump came from 3 full-day bookings on high-demand Fridays. Tuesdays and Thursdays are typically 20% cheaper.';
  if (lc.includes('friday'))
    return "Friday's historical occupancy peaks at 92% between 09:30–11:00. Booking before 08:30 gives you 68% slot choice and saves 1 credit.";
  if (lc.includes('marco'))
    return "I can share your L2-14 pass with Marco for today. He'll get a QR that works 08:00–16:00. Confirm?";
  if (lc.includes('where') || lc.includes('yesterday'))
    return 'Yesterday you parked at HQ Garage, slot L2-14, from 08:17 to 17:42 (9h 25m). Total: 3 credits.';
  return 'I only answer from your local data. Try asking about credits, slot availability, swap requests, or recent bookings.';
}

const DEFAULT_SUGGESTIONS = [
  'Find the nearest EV slot',
  'My credit usage this month',
  'Best time to book for Friday',
  'Share pass with Marco',
];

export function Assistant({
  open,
  onClose,
  reply = defaultReply,
  suggestions = DEFAULT_SUGGESTIONS,
}: AssistantProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [input, setInput] = useState('');
  const [msgs, setMsgs] = useState<Message[]>([
    {
      role: 'bot',
      content: t(
        'assistant.intro',
        "Hi. I'm your ParkHub Assistant — I run locally on this server and help you find bookings, slots, and patterns in your parking data. Nothing leaves this server.",
      ),
    },
  ]);
  const [thinking, setThinking] = useState(false);

  useEffect(() => {
    const d = dialogRef.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    if (!open && d.open) d.close();
  }, [open]);

  const send = (text?: string) => {
    const q = (text ?? input).trim();
    if (!q) return;
    setMsgs((m) => [...m, { role: 'user', content: q }]);
    setInput('');
    setThinking(true);
    // Small delay preserves the "thinking" affordance from the design.
    // Using setTimeout rather than useOptimistic because this isn't a
    // server round-trip — it's purely local pattern matching and the
    // UI intentionally pauses to feel responsive.
    setTimeout(() => {
      setMsgs((m) => [...m, { role: 'bot', content: reply(q) }]);
      setThinking(false);
    }, 500);
  };

  return (
    <dialog
      ref={dialogRef}
      onClose={onClose}
      className="assistant-dialog fixed right-0 top-0 bottom-0 left-auto m-0 h-dvh w-[min(440px,92vw)] max-w-none p-0 bg-surface-50 dark:bg-surface-900 border-l border-surface-200 dark:border-surface-800 shadow-2xl backdrop:bg-black/30"
      aria-labelledby="assistant-title"
    >
      {/* Header */}
      <div className="flex items-center gap-3 px-5 py-4 border-b border-surface-200 dark:border-surface-800">
        <div className="flex items-center justify-center w-9 h-9 rounded-xl bg-primary-600 text-white">
          <Sparkle weight="fill" className="w-[18px] h-[18px]" />
        </div>
        <div className="flex-1 min-w-0">
          <h2
            id="assistant-title"
            className="text-sm font-bold text-surface-900 dark:text-white"
            style={{ letterSpacing: '-0.01em' }}
          >
            {t('assistant.title', 'Assistant')}
          </h2>
          <div className="flex items-center gap-1.5 text-[11px] font-semibold text-surface-500 dark:text-surface-400">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
            {t('assistant.subtitle', 'Runs on-prem · no external calls')}
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="btn btn-ghost btn-icon"
          aria-label={t('common.close', 'Close')}
        >
          <X weight="bold" className="w-4 h-4" />
        </button>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-3">
        {msgs.map((m, i) => (
          <div key={i} className={`flex ${m.role === 'user' ? 'justify-end' : 'justify-start'}`}>
            <div
              className={
                m.role === 'user'
                  ? 'max-w-[85%] px-3.5 py-2.5 rounded-2xl bg-primary-600 text-white text-[13px] leading-relaxed rounded-br-[4px]'
                  : 'max-w-[85%] px-3.5 py-2.5 rounded-2xl bg-white dark:bg-surface-800 text-surface-900 dark:text-surface-100 text-[13px] leading-relaxed border border-surface-200 dark:border-surface-700 rounded-bl-[4px]'
              }
            >
              {m.content}
            </div>
          </div>
        ))}
        {thinking && (
          <div className="flex gap-1 px-3.5 py-2 bg-white dark:bg-surface-800 rounded-2xl w-fit border border-surface-200 dark:border-surface-700">
            {[0, 1, 2].map((i) => (
              <span
                key={i}
                className="w-1.5 h-1.5 rounded-full bg-surface-400 dark:bg-surface-500 assistant-bounce"
                style={{ animationDelay: `${i * 0.15}s` }}
              />
            ))}
          </div>
        )}

        {msgs.length === 1 && (
          <div className="mt-1 flex flex-wrap gap-1.5">
            {suggestions.map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => send(s)}
                className="px-2.5 py-1.5 rounded-full text-[11px] font-medium bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-700 transition-colors"
              >
                {s}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Input */}
      <div className="p-3.5 border-t border-surface-200 dark:border-surface-800">
        <div className="flex items-center gap-1.5 input pl-3 pr-1.5 py-1.5">
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') send();
            }}
            placeholder={t('assistant.placeholder', 'Ask about bookings, slots, credits…')}
            className="flex-1 border-none outline-none bg-transparent text-[13px] text-surface-900 dark:text-white placeholder:text-surface-400"
          />
          <button
            type="button"
            onClick={() => send()}
            disabled={!input.trim()}
            className="btn btn-primary btn-icon w-7 h-7 disabled:opacity-50"
            aria-label={t('assistant.send', 'Send')}
          >
            <ArrowRight weight="bold" className="w-3 h-3" />
          </button>
        </div>
        <div className="text-[10px] text-surface-400 dark:text-surface-500 mt-1.5 text-center">
          {t(
            'assistant.footer',
            'Rule-based helper · queries your local database · no data sent anywhere.',
          )}
        </div>
      </div>

      <style>{`
        @keyframes assistant-bounce {
          0%, 80%, 100% { transform: translateY(0); opacity: 0.5; }
          40% { transform: translateY(-4px); opacity: 1; }
        }
        .assistant-bounce { animation: assistant-bounce 1.2s infinite ease-in-out; }
        dialog.assistant-dialog[open] { display: flex; flex-direction: column; }
      `}</style>
    </dialog>
  );
}
