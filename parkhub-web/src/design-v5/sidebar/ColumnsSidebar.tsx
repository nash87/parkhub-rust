import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { NAV, type NavSection, type ScreenId } from '../nav';
import { V5NamedIcon } from '../primitives';
import { api } from '../../api/client';
import type { SidebarProps } from './MarbleSidebar';

/**
 * ColumnsSidebar — port of design/sidebar-v3.jsx with v5 tokens.
 *
 * Departures from the JSX prototype:
 *   - "Live mock data" (LIVE_LOT, fake checkout) is replaced with real API
 *     queries: api.me(), api.getBookings(), api.getLots(). When the data
 *     isn't available we degrade gracefully (skeleton bars / hidden card).
 *   - Hard-coded SV3 nav array replaced by the real NAV registry, grouped
 *     by section (main → fleet → admin) so the sidebar stays in sync with
 *     the rest of the app and Command Palette.
 *   - Numbered items use `n` from NavItem (01–26) instead of a parallel
 *     hand-curated numbering.
 *   - Always renders against the v5 `--v5-*` tokens — no `--color-*`
 *     references — so the sidebar respects the user's theme mode while
 *     keeping its own dark-leaning surface via tokens-aware overrides.
 *
 * Information density is deliberately low: signature cards (live pass +
 * lot occupancy + what's next) earn the real estate.
 */
export function ColumnsSidebar({ active, onNavigate, userLabel = 'Administrator', userEmail }: SidebarProps) {
  const meQ = useQuery({ queryKey: ['v5-cols-me'], queryFn: () => api.me(), staleTime: 60_000 });
  const bookingsQ = useQuery({
    queryKey: ['v5-cols-bookings'],
    queryFn: () => api.getBookings(),
    staleTime: 30_000,
  });
  const lotsQ = useQuery({ queryKey: ['v5-cols-lots'], queryFn: () => api.getLots(), staleTime: 60_000 });

  const me = meQ.data?.success ? meQ.data.data : undefined;
  const bookings = bookingsQ.data?.success ? bookingsQ.data.data ?? [] : [];
  const lots = lotsQ.data?.success ? lotsQ.data.data ?? [] : [];

  const userName = me?.name ?? userLabel;
  const userMail = me?.email ?? userEmail ?? 'admin@parkhub.test';
  const initials = userName
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((p) => p[0]?.toUpperCase())
    .join('') || 'A';

  // Live ticker — minutes until the user's next booking ends
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(t);
  }, []);

  // Find the current "active" booking (status = active or confirmed today).
  const activeBooking = useMemo(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);
    return bookings.find((b) => {
      if (!b.start_time) return false;
      const start = new Date(b.start_time);
      return start >= today && start < tomorrow;
    });
  }, [bookings]);

  const upcomingBooking = useMemo(() => {
    const sorted = [...bookings]
      .filter((b) => b.start_time && new Date(b.start_time).getTime() > now)
      .sort((a, b) => new Date(a.start_time!).getTime() - new Date(b.start_time!).getTime());
    return sorted[0];
  }, [bookings, now]);

  // Find current lot — first lot with active booking, else first lot.
  const currentLot = useMemo(() => {
    if (activeBooking?.lot_id) {
      const found = lots.find((l) => l.id === activeBooking.lot_id);
      if (found) return found;
    }
    return lots[0];
  }, [activeBooking, lots]);

  // Time until checkout
  let untilLabel: string | null = null;
  if (activeBooking?.end_time) {
    const end = new Date(activeBooking.end_time).getTime();
    const ms = Math.max(0, end - now);
    const h = Math.floor(ms / 3_600_000);
    const m = Math.floor((ms % 3_600_000) / 60_000);
    untilLabel = `${h}h ${String(m).padStart(2, '0')}m`;
  }

  return (
    <nav
      aria-label="Hauptnavigation"
      data-variant="columns"
      style={{
        width: 280,
        flexShrink: 0,
        height: '100%',
        background: 'var(--v5-nav-bg)',
        color: 'var(--v5-txt)',
        display: 'flex',
        flexDirection: 'column',
        position: 'relative',
        overflow: 'hidden',
        backgroundImage:
          'radial-gradient(ellipse 400px 300px at 0% 0%, color-mix(in oklch, var(--v5-acc) 12%, transparent), transparent 70%)',
        borderRight: '1px solid var(--v5-bor)',
      }}
    >
      {/* TOP: Brand + user chip */}
      <div
        style={{
          padding: '16px 18px 12px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <div
            style={{
              width: 24,
              height: 24,
              borderRadius: 6,
              background: 'linear-gradient(135deg, var(--v5-acc), oklch(0.4 0.1 175))',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: 11,
              fontWeight: 800,
              color: '#fff',
              letterSpacing: '-0.03em',
            }}
          >
            P
          </div>
          <div style={{ fontSize: 13, fontWeight: 700, letterSpacing: '-0.01em' }}>ParkHub</div>
        </div>
        <button
          type="button"
          aria-label={userName}
          title={`${userName} · ${userMail}`}
          style={{
            width: 28,
            height: 28,
            borderRadius: '50%',
            background: 'linear-gradient(135deg, var(--v5-acc), oklch(0.4 0.1 175))',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#fff',
            fontSize: 11,
            fontWeight: 700,
            border: 0,
            cursor: 'pointer',
          }}
        >
          {initials}
        </button>
      </div>

      {/* LIVE PASS CARD */}
      <div style={{ padding: '4px 14px 14px' }}>
        <div
          style={{
            position: 'relative',
            padding: '14px 14px 12px',
            borderRadius: 14,
            background:
              'linear-gradient(165deg, var(--v5-sur2) 0%, var(--v5-sur) 100%)',
            border: '1px solid var(--v5-bor)',
            overflow: 'hidden',
            boxShadow: 'var(--v5-shadow-card)',
          }}
        >
          <div
            aria-hidden="true"
            style={{
              position: 'absolute',
              top: -40,
              right: -40,
              width: 160,
              height: 160,
              borderRadius: '50%',
              background:
                'radial-gradient(circle, color-mix(in oklch, var(--v5-acc) 30%, transparent), transparent 65%)',
              pointerEvents: 'none',
            }}
          />

          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: 10,
              position: 'relative',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <span
                style={{
                  width: 7,
                  height: 7,
                  borderRadius: '50%',
                  background: activeBooking ? 'var(--v5-ok)' : 'var(--v5-mut)',
                  boxShadow: activeBooking ? '0 0 8px var(--v5-ok)' : 'none',
                }}
              />
              <span
                style={{
                  fontSize: 10,
                  fontWeight: 700,
                  letterSpacing: '0.14em',
                  textTransform: 'uppercase',
                  color: activeBooking ? 'var(--v5-ok)' : 'var(--v5-mut)',
                }}
              >
                {activeBooking ? 'Geparkt' : 'Frei'}
              </span>
            </div>
            <button
              type="button"
              onClick={() => onNavigate('einchecken')}
              data-testid="cols-pass-link"
              style={{
                fontSize: 10,
                fontWeight: 700,
                letterSpacing: '0.08em',
                textTransform: 'uppercase',
                color: 'var(--v5-mut)',
                padding: '3px 6px',
                borderRadius: 4,
                background: 'transparent',
                border: 0,
                cursor: 'pointer',
              }}
            >
              Pass →
            </button>
          </div>

          <div
            className="v5-mono"
            style={{ fontSize: 28, lineHeight: 1, marginBottom: 2, position: 'relative', fontWeight: 700 }}
          >
            {activeBooking?.slot_number ?? activeBooking?.slot_id?.slice(0, 4) ?? '—'}
          </div>
          <div style={{ fontSize: 11.5, color: 'var(--v5-mut)', marginBottom: 12, position: 'relative' }}>
            {activeBooking
              ? activeBooking.vehicle_plate ?? 'Buchung aktiv'
              : 'Keine aktive Buchung'}
          </div>

          {/* Time until checkout */}
          {untilLabel && activeBooking?.end_time && (
            <div style={{ position: 'relative' }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'baseline',
                  justifyContent: 'space-between',
                  marginBottom: 5,
                }}
              >
                <span
                  style={{
                    fontSize: 10.5,
                    color: 'var(--v5-mut)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    fontWeight: 600,
                  }}
                >
                  Verbleibend
                </span>
                <span
                  className="v5-mono"
                  style={{ fontSize: 12, fontWeight: 700, color: 'var(--v5-txt)' }}
                >
                  {untilLabel}
                </span>
              </div>
            </div>
          )}

          <div style={{ display: 'flex', gap: 6, marginTop: 12, position: 'relative' }}>
            <button
              type="button"
              onClick={() => onNavigate('einchecken')}
              style={{
                flex: 1,
                padding: '7px 10px',
                fontSize: 11.5,
                fontWeight: 700,
                borderRadius: 7,
                background: 'var(--v5-acc)',
                color: 'var(--v5-accent-fg)',
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 5,
                border: 0,
                cursor: 'pointer',
              }}
            >
              <V5NamedIcon name="check" size={12} color="var(--v5-accent-fg)" /> QR zeigen
            </button>
          </div>
        </div>
      </div>

      {/* SEARCH (visual placeholder — real palette is global) */}
      <div style={{ padding: '0 14px 10px' }}>
        <div
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            gap: 9,
            padding: '8px 10px',
            borderRadius: 8,
            background: 'var(--v5-sur2)',
            border: '1px solid var(--v5-bor)',
            color: 'var(--v5-mut)',
            fontSize: 12.5,
          }}
        >
          <V5NamedIcon name="search" size={13} color="var(--v5-mut)" />
          <span style={{ flex: 1, textAlign: 'left' }}>Suche</span>
          <kbd
            className="v5-mono"
            style={{
              padding: '1px 5px',
              fontSize: 10,
              fontWeight: 600,
              borderRadius: 3,
              background: 'var(--v5-bg)',
              color: 'var(--v5-mut)',
              border: '1px solid var(--v5-bor)',
            }}
          >
            ⌘K
          </kbd>
        </div>
      </div>

      {/* SCROLLABLE NAV — three sections, numbered */}
      <div style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden', padding: '0 6px' }}>
        {(['main', 'fleet', 'admin'] as NavSection[]).map((section) => (
          <div key={section} style={{ padding: '6px 8px 4px' }}>
            <SectionHeading section={section} />
            <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              {NAV.filter((n) => n.section === section).map((item) => (
                <NumberedNavItem
                  key={item.id}
                  num={item.n}
                  label={item.label}
                  iconName={item.icon}
                  isActive={active === item.id}
                  onClick={() => onNavigate(item.id as ScreenId)}
                />
              ))}
            </div>
          </div>
        ))}

        {/* Lot occupancy — signature card */}
        {currentLot && (
          <div style={{ padding: '14px 8px 8px' }}>
            <SectionHeading label="Aktueller Standort" />
            <LotOccupancyCard
              name={currentLot.name ?? 'Standort'}
              total={currentLot.total_slots ?? 0}
              free={currentLot.available_slots ?? 0}
              onOpen={() => onNavigate('karte')}
            />
          </div>
        )}
      </div>

      {/* FOOTER: What's next */}
      {upcomingBooking?.start_time && (
        <WhatsNextCard
          when={upcomingBooking.start_time}
          onClick={() => onNavigate('buchen')}
        />
      )}
    </nav>
  );
}

function SectionHeading({ section, label }: { section?: NavSection; label?: string }) {
  const text =
    label ??
    (section === 'main' ? 'Persönlich' : section === 'fleet' ? 'Flotte' : section === 'admin' ? 'Admin' : '');
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '4px 8px 8px',
      }}
    >
      <span
        style={{
          fontSize: 9.5,
          fontWeight: 700,
          color: 'var(--v5-mut)',
          textTransform: 'uppercase',
          letterSpacing: '0.14em',
        }}
      >
        {text}
      </span>
    </div>
  );
}

function NumberedNavItem({
  num,
  label,
  iconName,
  isActive,
  onClick,
}: {
  num: string;
  label: string;
  iconName: import('../icons').IconKey;
  isActive: boolean;
  onClick: () => void;
}) {
  const [hover, setHover] = useState(false);
  return (
    <button
      type="button"
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      aria-current={isActive ? 'page' : undefined}
      style={{
        width: '100%',
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: '9px 10px',
        borderRadius: 8,
        background: isActive ? 'var(--v5-acc-muted)' : hover ? 'var(--v5-row-hover)' : 'transparent',
        color: isActive ? 'var(--v5-acc)' : 'var(--v5-txt)',
        position: 'relative',
        textAlign: 'left',
        transition: 'background 120ms, color 120ms',
        border: 0,
        cursor: 'pointer',
      }}
    >
      {isActive && (
        <span
          aria-hidden="true"
          style={{
            position: 'absolute',
            left: 0,
            top: 10,
            bottom: 10,
            width: 2,
            borderRadius: 2,
            background: 'var(--v5-acc)',
          }}
        />
      )}
      <span
        className="v5-mono"
        style={{
          fontSize: 10,
          fontWeight: 700,
          color: isActive ? 'var(--v5-acc)' : 'var(--v5-mut)',
          width: 18,
          letterSpacing: '-0.02em',
        }}
      >
        {num}
      </span>
      <V5NamedIcon name={iconName} size={15} color={isActive ? 'var(--v5-acc)' : 'var(--v5-mut)'} />
      <span
        style={{
          flex: 1,
          fontSize: 13.5,
          fontWeight: isActive ? 600 : 500,
          letterSpacing: '-0.01em',
        }}
      >
        {label}
      </span>
    </button>
  );
}

function LotOccupancyCard({
  name,
  total,
  free,
  onOpen,
}: {
  name: string;
  total: number;
  free: number;
  onOpen: () => void;
}) {
  const freePct = total > 0 ? free / total : 0;
  const tone = freePct < 0.15 ? 'var(--v5-err)' : freePct < 0.3 ? 'var(--v5-warn)' : 'var(--v5-ok)';
  return (
    <div
      data-testid="cols-lot-card"
      style={{
        borderRadius: 12,
        padding: 12,
        border: '1px solid var(--v5-bor)',
        background: 'var(--v5-sur2)',
        position: 'relative',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'baseline',
          justifyContent: 'space-between',
          marginBottom: 10,
        }}
      >
        <div>
          <div className="v5-mono" style={{ fontSize: 20, lineHeight: 1, fontWeight: 700 }}>
            {free}
            <span
              style={{ fontSize: 12, color: 'var(--v5-mut)', fontWeight: 500, marginLeft: 4 }}
            >
              / {total}
            </span>
          </div>
          <div
            style={{
              fontSize: 10.5,
              color: 'var(--v5-mut)',
              textTransform: 'uppercase',
              letterSpacing: '0.08em',
              fontWeight: 600,
              marginTop: 2,
            }}
          >
            Plätze frei
          </div>
        </div>
        <div
          className="v5-mono"
          style={{
            fontSize: 11,
            fontWeight: 700,
            color: tone,
            padding: '2px 6px',
            borderRadius: 4,
            background: `color-mix(in oklch, ${tone} 18%, transparent)`,
          }}
        >
          {Math.round(freePct * 100)}%
        </div>
      </div>
      <div
        title={name}
        style={{
          fontSize: 11,
          color: 'var(--v5-mut)',
          marginBottom: 8,
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
      >
        {name}
      </div>
      <div
        style={{
          height: 6,
          borderRadius: 2,
          background: 'var(--v5-bg)',
          overflow: 'hidden',
        }}
      >
        <div
          style={{
            width: `${(1 - freePct) * 100}%`,
            height: '100%',
            background: tone,
          }}
        />
      </div>
      <button
        type="button"
        onClick={onOpen}
        style={{
          marginTop: 10,
          width: '100%',
          padding: '7px 10px',
          fontSize: 11.5,
          fontWeight: 600,
          borderRadius: 7,
          background: 'var(--v5-bg)',
          color: 'var(--v5-txt)',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 6,
          border: '1px solid var(--v5-bor)',
          cursor: 'pointer',
        }}
      >
        <V5NamedIcon name="map" size={11} color="var(--v5-txt)" /> Karte öffnen
      </button>
    </div>
  );
}

function WhatsNextCard({ when, onClick }: { when: string; onClick: () => void }) {
  let label = '';
  try {
    const d = new Date(when);
    const today = new Date();
    const tomorrow = new Date();
    tomorrow.setDate(today.getDate() + 1);
    const sameDay = d.toDateString() === today.toDateString();
    const isTomorrow = d.toDateString() === tomorrow.toDateString();
    const time = d.toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
    label = sameDay ? `Heute · ${time}` : isTomorrow ? `Morgen · ${time}` : d.toLocaleDateString('de-DE');
  } catch {
    label = when;
  }
  return (
    <div
      data-testid="cols-whats-next"
      style={{
        padding: '10px 14px 14px',
        borderTop: '1px solid var(--v5-bor)',
      }}
    >
      <button
        type="button"
        onClick={onClick}
        style={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '10px 12px',
          borderRadius: 10,
          background: 'var(--v5-sur2)',
          border: '1px solid var(--v5-bor)',
          textAlign: 'left',
          cursor: 'pointer',
        }}
      >
        <div
          style={{
            width: 30,
            height: 30,
            borderRadius: 8,
            background: 'var(--v5-acc-muted)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--v5-acc)',
          }}
        >
          <V5NamedIcon name="cal" size={14} color="var(--v5-acc)" />
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              fontSize: 10,
              fontWeight: 700,
              textTransform: 'uppercase',
              letterSpacing: '0.08em',
              color: 'var(--v5-mut)',
            }}
          >
            Als nächstes
          </div>
          <div
            style={{
              fontSize: 12.5,
              fontWeight: 600,
              color: 'var(--v5-txt)',
              marginTop: 1,
            }}
          >
            {label}
          </div>
        </div>
        <V5NamedIcon name="chev" size={12} color="var(--v5-mut)" />
      </button>
    </div>
  );
}
