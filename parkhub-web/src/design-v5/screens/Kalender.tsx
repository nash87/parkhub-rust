import { useMemo, useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery } from '@tanstack/react-query';
import { Badge, Card, SectionLabel, V5NamedIcon } from '../primitives';
import { api, type CalendarEvent } from '../../api/client';
import type { ScreenId } from '../nav';

const WEEKDAYS = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

function isSameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function isSameMonth(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth();
}

function formatDateKey(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function statusColor(status: string): string {
  switch (status) {
    case 'active':
    case 'confirmed':
      return 'var(--v5-ok)';
    case 'pending':
      return 'var(--v5-warn)';
    case 'cancelled':
      return 'var(--v5-err)';
    case 'completed':
      return 'var(--v5-acc)';
    default:
      return 'var(--v5-mut)';
  }
}

export function KalenderV5({ navigate }: { navigate: (id: ScreenId) => void }) {
  const [currentMonth, setCurrentMonth] = useState<Date>(() => {
    const d = new Date();
    d.setDate(1);
    d.setHours(0, 0, 0, 0);
    return d;
  });
  const [selectedDate, setSelectedDate] = useState<Date | null>(null);

  const rangeStart = useMemo(() => {
    const d = new Date(currentMonth);
    d.setDate(1);
    return formatDateKey(d);
  }, [currentMonth]);

  const rangeEnd = useMemo(() => {
    const d = new Date(currentMonth.getFullYear(), currentMonth.getMonth() + 1, 0);
    return formatDateKey(d);
  }, [currentMonth]);

  const { data: events = [], isLoading, isError } = useQuery({
    queryKey: ['kalender', rangeStart, rangeEnd],
    queryFn: async () => {
      const res = await api.calendarEvents(rangeStart, rangeEnd);
      if (!res.success) throw new Error(res.error?.message ?? 'Kalenderdaten konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const days = useMemo(() => {
    const year = currentMonth.getFullYear();
    const month = currentMonth.getMonth();
    const firstDay = new Date(year, month, 1);
    const lastDay = new Date(year, month + 1, 0);
    // Monday-start
    const startDow = firstDay.getDay() === 0 ? 6 : firstDay.getDay() - 1;
    const result: Date[] = [];
    for (let i = 0; i < startDow; i++) {
      result.push(new Date(year, month, 1 - startDow + i));
    }
    for (let d = 1; d <= lastDay.getDate(); d++) {
      result.push(new Date(year, month, d));
    }
    while (result.length % 7 !== 0) {
      result.push(new Date(year, month + 1, result.length - startDow - lastDay.getDate() + 1));
    }
    return result;
  }, [currentMonth]);

  const eventsByDay = useMemo(() => {
    const map = new Map<string, CalendarEvent[]>();
    for (const e of events) {
      const key = formatDateKey(new Date(e.start));
      const list = map.get(key);
      if (list) list.push(e);
      else map.set(key, [e]);
    }
    return map;
  }, [events]);

  const bookingCount = useMemo(() => events.filter((e) => e.type === 'booking').length, [events]);
  const absenceCount = useMemo(() => events.filter((e) => e.type === 'absence').length, [events]);

  const today = new Date();
  const selectedEvents = selectedDate ? eventsByDay.get(formatDateKey(selectedDate)) ?? [] : [];
  const monthLabel = currentMonth.toLocaleDateString('de-DE', { month: 'long', year: 'numeric' });

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <div style={{ height: 32, width: 220, borderRadius: 8, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        <div style={{ height: 420, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite' }} />
      </div>
    );
  }

  if (isError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Kalender konnte nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12, flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Kalender</span>
          <Badge variant="gray">
            <NumberFlow value={events.length} />
          </Badge>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <button
            type="button"
            aria-label="Vorheriger Monat"
            onClick={() => setCurrentMonth((d) => new Date(d.getFullYear(), d.getMonth() - 1, 1))}
            style={{ width: 32, height: 32, borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', transform: 'rotate(180deg)' }}
          >
            <V5NamedIcon name="chev" size={12} color="var(--v5-mut)" />
          </button>
          <div aria-live="polite" style={{ fontSize: 12, fontWeight: 600, color: 'var(--v5-txt)', minWidth: 140, textAlign: 'center' }}>
            {monthLabel}
          </div>
          <button
            type="button"
            aria-label="Nächster Monat"
            onClick={() => setCurrentMonth((d) => new Date(d.getFullYear(), d.getMonth() + 1, 1))}
            style={{ width: 32, height: 32, borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
          >
            <V5NamedIcon name="chev" size={12} color="var(--v5-mut)" />
          </button>
          <button
            type="button"
            onClick={() => navigate('buchen')}
            className="v5-btn"
            style={{ padding: '7px 14px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 5, marginLeft: 6 }}
          >
            <V5NamedIcon name="plus" size={12} />
            Platz buchen
          </button>
        </div>
      </div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, animationDelay: '0.06s' }}>
        <SummaryStat label="Einträge" value={events.length} icon="cal" />
        <SummaryStat label="Buchungen" value={bookingCount} icon="list" />
        <SummaryStat label="Abwesenheit" value={absenceCount} icon="users" />
      </div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 280px', gap: 12, animationDelay: '0.12s' }}>
        <Card style={{ overflow: 'hidden', padding: 0 }}>
          {/* WCAG `role="grid"` requires `role="row"` children, which in turn
              wrap `role="columnheader"` and `role="gridcell"` (axe rules
              aria-required-children / aria-required-parent). We keep the CSS
              grid layout for visuals and add the ARIA row wrappers so the
              semantics match the WAI-ARIA grid pattern. */}
          <div role="grid" aria-label={`Kalender ${monthLabel}`}>
            <div
              role="row"
              style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', borderBottom: '1px solid var(--v5-bor)' }}
            >
              {WEEKDAYS.map((label) => (
                <div
                  key={label}
                  role="columnheader"
                  className="v5-mono"
                  style={{ padding: '10px 0', fontSize: 10, letterSpacing: 1.2, textTransform: 'uppercase', textAlign: 'center', color: 'var(--v5-mut)' }}
                >
                  {label}
                </div>
              ))}
            </div>
            {Array.from({ length: Math.ceil(days.length / 7) }).map((_, weekIdx) => (
              <div
                key={`week-${weekIdx}`}
                role="row"
                style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)' }}
              >
                {days.slice(weekIdx * 7, weekIdx * 7 + 7).map((day, localIdx) => {
                  const idx = weekIdx * 7 + localIdx;
                  const inMonth = isSameMonth(day, currentMonth);
                  const isToday = isSameDay(day, today);
                  const selected = !!(selectedDate && isSameDay(day, selectedDate));
                  const dayEvents = eventsByDay.get(formatDateKey(day)) ?? [];
                  return (
                    <div role="gridcell" key={idx} style={{ display: 'flex' }}>
                      <button
                        type="button"
                        data-testid="kalender-day"
                        aria-label={day.toLocaleDateString('de-DE', { weekday: 'long', day: 'numeric', month: 'long' })}
                        aria-pressed={selected}
                        onClick={() => setSelectedDate(day)}
                        style={{
                          minHeight: 82,
                          width: '100%',
                          padding: 6,
                          background: selected ? 'var(--v5-acc-muted)' : 'transparent',
                          border: 'none',
                          borderRight: (localIdx + 1) % 7 === 0 ? 'none' : '1px solid var(--v5-bor)',
                          borderBottom: idx < days.length - 7 ? '1px solid var(--v5-bor)' : 'none',
                          textAlign: 'left',
                          cursor: 'pointer',
                          opacity: inMonth ? 1 : 0.4,
                          display: 'flex',
                          flexDirection: 'column',
                          gap: 4,
                          fontFamily: 'inherit',
                          color: 'inherit',
                        }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                          <span
                            className="v5-mono"
                            style={{
                              fontSize: 11,
                              fontWeight: isToday ? 700 : 500,
                              color: isToday ? 'var(--v5-accent-fg)' : selected ? 'var(--v5-acc)' : 'var(--v5-txt)',
                              background: isToday ? 'var(--v5-acc)' : 'transparent',
                              padding: isToday ? '2px 6px' : 0,
                              borderRadius: isToday ? 999 : 0,
                              minWidth: 18,
                              textAlign: 'center',
                            }}
                          >
                            {day.getDate()}
                          </span>
                          {dayEvents.length > 0 && (
                            <span className="v5-mono" style={{ fontSize: 9, color: 'var(--v5-mut)' }}>
                              {dayEvents.length}
                            </span>
                          )}
                        </div>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                          {dayEvents.slice(0, 2).map((e) => (
                            <div
                              key={e.id}
                              title={`${e.title} (${e.status})`}
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: 4,
                                fontSize: 10,
                                color: 'var(--v5-txt)',
                                background: 'var(--v5-sur2)',
                                borderRadius: 5,
                                padding: '2px 5px',
                              }}
                            >
                              <span style={{ width: 5, height: 5, borderRadius: '50%', background: statusColor(e.status), flexShrink: 0 }} />
                              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.title}</span>
                            </div>
                          ))}
                          {dayEvents.length > 2 && (
                            <div style={{ fontSize: 9, color: 'var(--v5-mut)' }}>+{dayEvents.length - 2}</div>
                          )}
                        </div>
                      </button>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        </Card>

        <Card style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10, alignSelf: 'start' }}>
          <SectionLabel>
            {selectedDate
              ? selectedDate.toLocaleDateString('de-DE', { weekday: 'long', day: 'numeric', month: 'long' })
              : 'Tag wählen'}
          </SectionLabel>
          {selectedDate ? (
            selectedEvents.length === 0 ? (
              <div style={{ padding: 18, textAlign: 'center', color: 'var(--v5-mut)', fontSize: 12 }}>
                Keine Einträge
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {selectedEvents.map((e) => (
                  <div
                    key={e.id}
                    data-testid="kalender-detail"
                    style={{
                      padding: 10,
                      borderRadius: 10,
                      background: 'var(--v5-sur2)',
                      border: '1px solid var(--v5-bor)',
                      display: 'flex',
                      gap: 8,
                      alignItems: 'flex-start',
                    }}
                  >
                    <span
                      aria-hidden="true"
                      style={{ width: 3, alignSelf: 'stretch', background: statusColor(e.status), borderRadius: 3 }}
                    />
                    <div style={{ minWidth: 0, flex: 1 }}>
                      <div style={{ fontSize: 12, fontWeight: 600, color: 'var(--v5-txt)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {e.title}
                      </div>
                      <div style={{ fontSize: 10, color: 'var(--v5-mut)', marginTop: 2 }}>
                        {new Date(e.start).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })}
                        {' – '}
                        {new Date(e.end).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })}
                        {e.lot_name ? ` · ${e.lot_name}` : ''}
                      </div>
                      <div style={{ marginTop: 6, display: 'flex', gap: 4 }}>
                        <Badge variant={e.type === 'booking' ? 'primary' : 'gray'}>
                          {e.type === 'booking' ? 'Buchung' : 'Abwesenheit'}
                        </Badge>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )
          ) : (
            <div style={{ padding: 18, textAlign: 'center', color: 'var(--v5-mut)', fontSize: 12 }}>
              Klicken Sie auf ein Datum, um Einträge anzuzeigen.
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}

function SummaryStat({ label, value, icon }: { label: string; value: number; icon: 'cal' | 'list' | 'users' }) {
  return (
    <Card style={{ padding: '12px 14px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.3, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>
            {label}
          </div>
          <div className="v5-mono" style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-txt)', marginTop: 4 }}>
            <NumberFlow value={value} />
          </div>
        </div>
        <div style={{ width: 26, height: 26, borderRadius: 8, background: 'var(--v5-sur2)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <V5NamedIcon name={icon} size={12} color="var(--v5-mut)" />
        </div>
      </div>
    </Card>
  );
}
