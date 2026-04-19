/**
 * SidebarV3 — ParkHub-native opinionated navigation shell.
 *
 * Ported from `/tmp/parkhub-design-2/design/sidebar-v3.jsx` (claude.ai
 * design v4 series). Dark surface always — even on light-mode pages —
 * because the sidebar is its own visual thing, not tinted page chrome.
 *
 * Signature moves:
 *   - Top band: brand + notification bell + user chip
 *   - Live Pass card: the current active booking with ticker + QR / +1h
 *   - Floor heatmap: stack of occupancy bars synthesised from slot_number
 *     prefixes ("L1-17" → floor "L1"), swapped out for a lot picker on
 *     demand
 *   - "Up next" footer: the soonest future booking
 *
 * Data contract with `api.ts`:
 *   - Booking.slot_number may encode "floor-slot" as "L1-17", "L1·17",
 *     "L1/17", "B2-03" — we split on non-alphanumeric. If there's no
 *     separator we fall back to treating the whole string as the slot
 *     label with no floor info.
 *   - api.getLotSlots(lotId) is only called for the active booking's lot
 *     to keep the payload cheap. If it fails or returns empty the floor
 *     heatmap is hidden (we degrade to the free/total big number only).
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import {
  ArrowsClockwise,
  Bell,
  CalendarPlus,
  Car,
  ChartLine,
  GearSix,
  House,
  MagnifyingGlass,
  MapPin,
  QrCode,
  ShieldCheck,
  SignOut,
  Sparkle,
  SquaresFour,
  Trophy,
  User,
  Users,
} from '@phosphor-icons/react';
import { api, type Booking, type ParkingLot, type ParkingSlot } from '../../api/client';
import { useAuth } from '../../context/AuthContext';
import { isActivePath } from './navActive';

type IconComponent = React.ComponentType<{ size?: number; weight?: 'regular' | 'bold' | 'fill' | 'duotone' | 'thin' | 'light' }>;

interface NumberedItem {
  num: string;
  Icon: IconComponent;
  label: string;
  to: string;
  shortcut?: string;
}

interface FlatItem {
  Icon: IconComponent;
  label: string;
  to: string;
  badge?: number;
}

const PRIMARY_NAV: NumberedItem[] = [
  { num: '01', Icon: House, label: 'Today', to: '/', shortcut: 'D' },
  { num: '02', Icon: CalendarPlus, label: 'Book', to: '/book', shortcut: 'B' },
  { num: '03', Icon: QrCode, label: 'My pass', to: '/guest-pass', shortcut: 'P' },
  { num: '04', Icon: MapPin, label: 'Live map', to: '/map' },
];

const WORKSPACE_NAV: FlatItem[] = [
  { Icon: Car, label: 'Vehicles', to: '/vehicles' },
  { Icon: ArrowsClockwise, label: 'Swap requests', to: '/swap-requests' },
  { Icon: Users, label: 'Team', to: '/team' },
  { Icon: Trophy, label: 'Leaderboard', to: '/leaderboard' },
];

const ADMIN_NAV: FlatItem[] = [
  { Icon: SquaresFour, label: 'Lots & floors', to: '/admin/lots' },
  { Icon: ShieldCheck, label: 'Roles', to: '/admin/roles' },
  { Icon: ChartLine, label: 'Analytics', to: '/admin/analytics' },
];

// ----------------------------------------------------------------------
// Derivation helpers
// ----------------------------------------------------------------------

function parseSlotFloor(slotNumber: string): { floor: string; number: string } | null {
  if (!slotNumber) return null;
  const parts = slotNumber.split(/[^A-Za-z0-9]+/).filter(Boolean);
  if (parts.length < 2) return null;
  return { floor: parts[0], number: parts.slice(1).join('-') };
}

interface FloorSummary {
  label: string;
  occupancy: number;
  youLabel?: string;
}

/**
 * Bucket slots by floor prefix, compute occupancy from non-available
 * status. We sort floors in the conventional basement-first ordering
 * ("B2" < "B1" < "L1" < "L2"…) which matches the design mock.
 */
function summariseFloors(slots: ParkingSlot[], mySlotNumber?: string): FloorSummary[] {
  const buckets = new Map<string, { total: number; taken: number }>();
  for (const slot of slots) {
    const parsed = parseSlotFloor(slot.slot_number);
    if (!parsed) continue;
    const bucket = buckets.get(parsed.floor) ?? { total: 0, taken: 0 };
    bucket.total += 1;
    if (slot.status && slot.status !== 'available' && slot.status !== 'free') {
      bucket.taken += 1;
    }
    buckets.set(parsed.floor, bucket);
  }

  const mySlot = mySlotNumber ? parseSlotFloor(mySlotNumber) : null;

  const floors: FloorSummary[] = Array.from(buckets.entries())
    .filter(([, b]) => b.total > 0)
    .map(([label, b]) => ({
      label,
      occupancy: b.taken / b.total,
      youLabel: mySlot?.floor === label ? mySlotNumber : undefined,
    }));

  floors.sort((a, b) => {
    const aBasement = a.label.toUpperCase().startsWith('B');
    const bBasement = b.label.toUpperCase().startsWith('B');
    // Basements first, deeper to shallower; then ground/above, lower to higher.
    if (aBasement && !bBasement) return -1;
    if (!aBasement && bBasement) return 1;
    const an = parseInt(a.label.replace(/\D/g, ''), 10) || 0;
    const bn = parseInt(b.label.replace(/\D/g, ''), 10) || 0;
    if (aBasement && bBasement) return bn - an; // B2 before B1
    return an - bn;
  });

  return floors.slice(0, 8);
}

function formatHoursMinutes(msUntil: number): { h: number; m: number } {
  const clamped = Math.max(0, msUntil);
  return {
    h: Math.floor(clamped / 3600000),
    m: Math.floor((clamped % 3600000) / 60000),
  };
}

function formatUpNext(booking: Booking): string {
  const start = new Date(booking.start_time);
  if (Number.isNaN(start.getTime())) return 'Scheduled';
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const startDay = new Date(start.getFullYear(), start.getMonth(), start.getDate());
  const dayMs = 86_400_000;
  const diffDays = Math.round((startDay.getTime() - today.getTime()) / dayMs);
  const hhmm = `${String(start.getHours()).padStart(2, '0')}:${String(start.getMinutes()).padStart(2, '0')}`;
  if (diffDays === 0) return `Today · ${hhmm}`;
  if (diffDays === 1) return `Tomorrow · ${hhmm}`;
  if (diffDays > 1 && diffDays < 7) {
    return `${start.toLocaleDateString(undefined, { weekday: 'short' })} · ${hhmm}`;
  }
  return `${start.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} · ${hhmm}`;
}

// ----------------------------------------------------------------------
// Component
// ----------------------------------------------------------------------

export function SidebarV3() {
  const location = useLocation();
  const navigate = useNavigate();
  const { user, logout } = useAuth();

  const [bookings, setBookings] = useState<Booking[]>([]);
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [activeSlots, setActiveSlots] = useState<ParkingSlot[]>([]);
  const [, setLoading] = useState(true);
  const [lotSwitcherOpen, setLotSwitcherOpen] = useState(false);
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const [nowTick, setNowTick] = useState(() => Date.now());

  // Ticker so the "time until checkout" progress redraws every 30s.
  useEffect(() => {
    const t = window.setInterval(() => setNowTick(Date.now()), 30_000);
    return () => window.clearInterval(t);
  }, []);

  // Fetch bookings + lots in parallel. Bookings feed both the Live Pass
  // and the "Up next" footer; lots feed the switcher.
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    Promise.all([api.getBookings(), api.getLots()])
      .then(([bookingsRes, lotsRes]) => {
        if (cancelled) return;
        if (bookingsRes.success && bookingsRes.data) setBookings(bookingsRes.data);
        if (lotsRes.success && lotsRes.data) setLots(lotsRes.data);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const activeBooking = useMemo<Booking | undefined>(() => {
    const now = Date.now();
    return bookings.find(b => {
      if (b.status !== 'active' && b.status !== 'confirmed') return false;
      const start = new Date(b.start_time).getTime();
      const end = new Date(b.end_time).getTime();
      return Number.isFinite(start) && Number.isFinite(end) && start <= now && now <= end;
    });
  }, [bookings]);

  const upNext = useMemo<Booking | undefined>(() => {
    const now = Date.now();
    const future = bookings.filter(b => {
      if (b.status === 'cancelled' || b.status === 'completed') return false;
      const start = new Date(b.start_time).getTime();
      return Number.isFinite(start) && start > now;
    });
    future.sort((a, b) => new Date(a.start_time).getTime() - new Date(b.start_time).getTime());
    return future[0];
  }, [bookings]);

  // Fetch slots only for the currently occupied lot. Kept in a separate
  // effect so it re-runs when the user's active booking changes.
  useEffect(() => {
    if (!activeBooking?.lot_id) {
      setActiveSlots([]);
      return;
    }
    let cancelled = false;
    api.getLotSlots(activeBooking.lot_id).then(res => {
      if (cancelled) return;
      if (res.success && res.data) setActiveSlots(res.data);
      else setActiveSlots([]);
    });
    return () => {
      cancelled = true;
    };
  }, [activeBooking?.lot_id]);

  const activeLot = useMemo<ParkingLot | undefined>(() => {
    if (!activeBooking) return undefined;
    return lots.find(l => l.id === activeBooking.lot_id);
  }, [lots, activeBooking]);

  const floors = useMemo(
    () => summariseFloors(activeSlots, activeBooking?.slot_number),
    [activeSlots, activeBooking?.slot_number],
  );

  const checkoutMs = activeBooking ? new Date(activeBooking.end_time).getTime() : 0;
  const startMs = activeBooking ? new Date(activeBooking.start_time).getTime() : 0;
  const totalWindow = Math.max(1, checkoutMs - startMs);
  const msUntil = Math.max(0, checkoutMs - nowTick);
  const { h: hUntil, m: mUntil } = formatHoursMinutes(msUntil);
  const checkoutLabel = activeBooking
    ? `Until ${new Date(activeBooking.end_time).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`
    : '';
  const progressPct = Math.max(5, ((totalWindow - msUntil) / totalWindow) * 100);

  const freePct = activeLot && activeLot.total_slots > 0
    ? activeLot.available_slots / activeLot.total_slots
    : 0;

  const initial = (user?.name || user?.username || 'U').charAt(0).toUpperCase();
  const isAdmin = !!(user?.role && ['admin', 'superadmin'].includes(user.role));

  const openCommandPalette = useCallback(() => {
    window.dispatchEvent(new CustomEvent('parkhub:open-command-palette'));
  }, []);

  const handleLogout = useCallback(() => {
    setUserMenuOpen(false);
    logout();
    navigate('/welcome');
  }, [logout, navigate]);

  const slotDisplay = useMemo(() => {
    if (!activeBooking?.slot_number) return null;
    const parsed = parseSlotFloor(activeBooking.slot_number);
    return parsed ?? { floor: '', number: activeBooking.slot_number };
  }, [activeBooking?.slot_number]);

  return (
    <aside
      className="sv3-root flex flex-col flex-shrink-0 h-full relative overflow-hidden"
      style={{
        width: 280,
        background: 'oklch(0.17 0.02 260)',
        color: 'oklch(0.96 0.005 260)',
        backgroundImage:
          'radial-gradient(ellipse 400px 300px at 0% 0%, color-mix(in oklch, var(--color-primary-500, #6366f1) 12%, transparent), transparent 70%)',
        borderRight: '1px solid oklch(0.24 0.015 260)',
      }}
      aria-label="Main navigation"
    >
      {/* Local tokens — scoped via .sv3-root so the sidebar stays dark on light pages. */}
      <style>{`
        .sv3-root .sv3-hover:hover { background: oklch(0.22 0.018 260); }
        .sv3-root .sv3-num {
          font-family: "SF Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
          font-variant-numeric: tabular-nums;
          font-feature-settings: "tnum";
          letter-spacing: -0.02em;
        }
        .sv3-root .sv3-display {
          font-feature-settings: "ss01", "cv11";
          letter-spacing: -0.04em;
          font-weight: 700;
        }
      `}</style>

      {/* TOP: Brand + bell + user chip */}
      <div className="flex items-center justify-between" style={{ padding: '16px 18px 12px' }}>
        <Link to="/" className="flex items-center gap-2.5" aria-label="Dashboard">
          <div
            className="flex items-center justify-center font-extrabold text-white"
            style={{
              width: 24,
              height: 24,
              borderRadius: 6,
              background: 'linear-gradient(135deg, var(--color-primary-400, #818cf8), var(--color-primary-600, #4f46e5))',
              fontSize: 11,
              letterSpacing: '-0.03em',
              boxShadow:
                '0 0 0 1px color-mix(in oklch, var(--color-primary-500, #6366f1) 60%, transparent), 0 4px 12px -4px var(--color-primary-600, #4f46e5)',
            }}
          >
            P
          </div>
          <div className="font-bold" style={{ fontSize: 13, letterSpacing: '-0.01em' }}>
            ParkHub
          </div>
        </Link>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => {
              // Dispatches the same event pages use to open the notification centre.
              window.dispatchEvent(new CustomEvent('parkhub:open-notifications'));
            }}
            title="Notifications"
            className="sv3-hover relative flex items-center justify-center"
            style={{ width: 28, height: 28, borderRadius: 7, color: 'oklch(0.75 0.01 260)' }}
          >
            <Bell size={14} />
            <span
              aria-hidden="true"
              style={{
                position: 'absolute',
                top: 6,
                right: 6,
                width: 6,
                height: 6,
                borderRadius: '50%',
                background: 'var(--color-danger, #ef4444)',
              }}
            />
          </button>
          <button
            type="button"
            onClick={() => setUserMenuOpen(o => !o)}
            title={user?.name || 'Account'}
            aria-haspopup="menu"
            aria-expanded={userMenuOpen}
            className="flex items-center justify-center font-bold text-white transition-shadow"
            style={{
              width: 28,
              height: 28,
              borderRadius: '50%',
              background: 'linear-gradient(135deg, var(--color-primary-400, #818cf8), var(--color-primary-600, #4f46e5))',
              fontSize: 11,
              letterSpacing: '-0.02em',
              border: '1.5px solid oklch(0.17 0.02 260)',
              boxShadow: userMenuOpen
                ? '0 0 0 1.5px var(--color-primary-500, #6366f1)'
                : '0 0 0 1.5px transparent',
            }}
          >
            {initial}
          </button>
        </div>
      </div>

      {userMenuOpen && (
        <UserMenu
          userName={user?.name || user?.username || 'You'}
          userEmail={user?.email}
          userRole={user?.role}
          onClose={() => setUserMenuOpen(false)}
          onLogout={handleLogout}
          onOpenAssistant={() => {
            setUserMenuOpen(false);
            openCommandPalette();
          }}
        />
      )}

      {/* LIVE PASS CARD (or empty-state CTA) */}
      <div style={{ padding: '4px 14px 14px' }}>
        {activeBooking && slotDisplay ? (
          <div
            className="relative overflow-hidden"
            style={{
              padding: '14px 14px 12px',
              borderRadius: 14,
              background: 'linear-gradient(165deg, oklch(0.24 0.03 260) 0%, oklch(0.19 0.02 260) 100%)',
              border: '1px solid oklch(0.26 0.018 260)',
              boxShadow: '0 1px 0 oklch(0.28 0.02 260) inset, 0 20px 40px -20px rgba(0,0,0,0.5)',
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
                  'radial-gradient(circle, color-mix(in oklch, var(--color-primary-500, #6366f1) 30%, transparent), transparent 65%)',
                pointerEvents: 'none',
              }}
            />

            <div className="relative flex items-center justify-between" style={{ marginBottom: 10 }}>
              <div className="flex items-center gap-1.5">
                <span
                  style={{
                    width: 7,
                    height: 7,
                    borderRadius: '50%',
                    background: 'var(--color-success, #10b981)',
                    boxShadow: '0 0 8px var(--color-success, #10b981)',
                  }}
                />
                <span
                  className="uppercase font-bold"
                  style={{ fontSize: 10, letterSpacing: '0.14em', color: 'oklch(0.82 0.04 150)' }}
                >
                  Parked
                </span>
              </div>
              <Link
                to="/guest-pass"
                className="sv3-hover uppercase font-bold"
                style={{
                  fontSize: 10,
                  letterSpacing: '0.08em',
                  color: 'oklch(0.70 0.01 260)',
                  padding: '3px 6px',
                  borderRadius: 4,
                }}
              >
                Pass →
              </Link>
            </div>

            <div className="sv3-display relative" style={{ fontSize: 28, lineHeight: 1, marginBottom: 2 }}>
              {slotDisplay.floor}
              {slotDisplay.floor && slotDisplay.number && (
                <span style={{ color: 'oklch(0.45 0.015 260)', fontWeight: 500, margin: '0 3px' }}>·</span>
              )}
              {slotDisplay.number}
            </div>
            <div
              className="relative"
              style={{ fontSize: 11.5, color: 'oklch(0.70 0.01 260)', marginBottom: 12 }}
            >
              {activeBooking.vehicle_plate || 'No vehicle assigned'}
              {activeLot?.name ? ` · ${activeLot.name}` : ''}
            </div>

            <div className="relative">
              <div className="flex items-baseline justify-between" style={{ marginBottom: 5 }}>
                <span
                  className="uppercase font-semibold"
                  style={{ fontSize: 10.5, color: 'oklch(0.62 0.015 260)', letterSpacing: '0.06em' }}
                >
                  {checkoutLabel}
                </span>
                <span
                  className="sv3-num font-bold"
                  style={{ fontSize: 12, color: 'oklch(0.92 0.005 260)' }}
                >
                  {hUntil}h {String(mUntil).padStart(2, '0')}m
                </span>
              </div>
              <div
                style={{
                  height: 3,
                  borderRadius: 2,
                  background: 'oklch(0.24 0.02 260)',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    width: `${Math.min(100, progressPct)}%`,
                    height: '100%',
                    background:
                      'linear-gradient(90deg, var(--color-primary-400, #818cf8), var(--color-primary-600, #4f46e5))',
                    borderRadius: 2,
                  }}
                />
              </div>
            </div>

            <div className="relative flex gap-1.5" style={{ marginTop: 12 }}>
              <Link
                to="/guest-pass"
                className="inline-flex items-center justify-center gap-1.5 font-bold"
                style={{
                  flex: 1,
                  padding: '7px 10px',
                  fontSize: 11.5,
                  borderRadius: 7,
                  background: 'oklch(0.98 0.005 260)',
                  color: 'oklch(0.18 0.02 260)',
                }}
              >
                <QrCode size={12} /> Show QR
              </Link>
              <button
                type="button"
                disabled
                className="inline-flex items-center justify-center gap-1.5 font-semibold"
                style={{
                  padding: '7px 10px',
                  fontSize: 11.5,
                  borderRadius: 7,
                  background: 'oklch(0.26 0.02 260)',
                  color: 'oklch(0.70 0.01 260)',
                  opacity: 0.6,
                  cursor: 'not-allowed',
                }}
                title="Extend parking — coming soon"
              >
                +1h
              </button>
            </div>
          </div>
        ) : (
          <EmptyPassCard />
        )}
      </div>

      {/* SEARCH */}
      <div style={{ padding: '0 14px 10px' }}>
        <button
          type="button"
          onClick={openCommandPalette}
          className="flex items-center gap-2.5 w-full text-left transition-colors"
          style={{
            padding: '8px 10px',
            borderRadius: 8,
            background: 'oklch(0.22 0.018 260)',
            border: '1px solid oklch(0.26 0.018 260)',
            color: 'oklch(0.62 0.015 260)',
            fontSize: 12.5,
          }}
        >
          <MagnifyingGlass size={13} />
          <span className="flex-1">Search</span>
          <kbd
            className="sv3-num font-semibold"
            style={{
              padding: '1px 5px',
              fontSize: 10,
              borderRadius: 3,
              background: 'oklch(0.16 0.018 260)',
              color: 'oklch(0.70 0.01 260)',
              border: '1px solid oklch(0.26 0.018 260)',
            }}
          >
            ⌘K
          </kbd>
        </button>
      </div>

      {/* SCROLLABLE NAV */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden" style={{ padding: '0 6px' }}>
        <div style={{ padding: '6px 8px 4px' }}>
          <SectionHeading label="Personal" />
          <div className="flex flex-col" style={{ gap: 1 }}>
            {PRIMARY_NAV.map(item => (
              <NumberedNavItem
                key={item.to}
                item={item}
                active={isActivePath(location.pathname, item.to)}
              />
            ))}
          </div>
        </div>

        {/* Current lot — signature move */}
        <div style={{ padding: '14px 8px 4px' }}>
          <SectionHeading
            label="Current lot"
            right={
              lots.length > 1 ? (
                <button
                  type="button"
                  onClick={() => setLotSwitcherOpen(o => !o)}
                  className="uppercase font-semibold"
                  style={{
                    fontSize: 10,
                    color: 'oklch(0.60 0.015 260)',
                    letterSpacing: '0.06em',
                  }}
                >
                  {lotSwitcherOpen ? 'Close' : 'Switch'}
                </button>
              ) : null
            }
          />

          {lotSwitcherOpen ? (
            <div
              style={{
                borderRadius: 10,
                padding: 6,
                border: '1px solid oklch(0.26 0.018 260)',
                background: 'oklch(0.20 0.018 260)',
              }}
            >
              {lots.map(lot => {
                const isActive = lot.id === activeLot?.id;
                return (
                  <button
                    key={lot.id}
                    type="button"
                    onClick={() => {
                      setLotSwitcherOpen(false);
                      navigate(`/map?lot=${lot.id}`);
                    }}
                    className={isActive ? '' : 'sv3-hover'}
                    style={{
                      width: '100%',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      padding: 8,
                      borderRadius: 7,
                      background: isActive ? 'oklch(0.25 0.02 260)' : 'transparent',
                      textAlign: 'left',
                    }}
                  >
                    <div
                      className="sv3-display"
                      style={{
                        fontSize: 11,
                        width: 24,
                        textAlign: 'center',
                        color: isActive ? '#fff' : 'oklch(0.60 0.015 260)',
                      }}
                    >
                      {lot.name.slice(0, 2).toUpperCase()}
                    </div>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div
                        className="font-semibold truncate"
                        style={{ fontSize: 12.5 }}
                      >
                        {lot.name}
                      </div>
                      <div
                        className="sv3-num"
                        style={{ fontSize: 10.5, color: 'oklch(0.60 0.015 260)' }}
                      >
                        {lot.available_slots} / {lot.total_slots} free
                      </div>
                    </div>
                    {isActive && (
                      <span
                        style={{
                          width: 6,
                          height: 6,
                          borderRadius: '50%',
                          background: 'var(--color-success, #10b981)',
                        }}
                      />
                    )}
                  </button>
                );
              })}
            </div>
          ) : activeLot ? (
            <LotFloorMap
              lotName={activeLot.name}
              freeSlots={activeLot.available_slots}
              totalSlots={activeLot.total_slots}
              freePct={freePct}
              floors={floors}
              onOpenMap={() => navigate('/map')}
            />
          ) : null}
        </div>

        {/* Workspace */}
        <div style={{ padding: '14px 8px 4px' }}>
          <SectionHeading label="Workspace" />
          <div className="flex flex-col" style={{ gap: 1 }}>
            {WORKSPACE_NAV.map(item => (
              <FlatNavItem
                key={item.to}
                item={item}
                active={isActivePath(location.pathname, item.to)}
              />
            ))}
          </div>
        </div>

        {/* Admin */}
        {isAdmin && (
          <div style={{ padding: '14px 8px 8px' }}>
            <SectionHeading label="Admin" tag="restricted" />
            <div className="flex flex-col" style={{ gap: 1 }}>
              {ADMIN_NAV.map(item => (
                <FlatNavItem
                  key={item.to}
                  item={item}
                  active={isActivePath(location.pathname, item.to)}
                />
              ))}
            </div>
          </div>
        )}
      </div>

      {/* FOOTER: Up next */}
      <WhatsNextCard upNext={upNext} />
    </aside>
  );
}

// ----------------------------------------------------------------------
// Sub-components
// ----------------------------------------------------------------------

function SectionHeading({
  label,
  right,
  tag,
}: {
  label: string;
  right?: React.ReactNode;
  tag?: string;
}) {
  return (
    <div className="flex items-center justify-between" style={{ padding: '4px 8px 8px' }}>
      <div className="flex items-center gap-2">
        <span
          className="uppercase font-bold"
          style={{
            fontSize: 9.5,
            color: 'oklch(0.55 0.015 260)',
            letterSpacing: '0.14em',
          }}
        >
          {label}
        </span>
        {tag && (
          <span
            className="sv3-num uppercase font-bold"
            style={{
              fontSize: 8.5,
              padding: '1px 4px',
              borderRadius: 2,
              background: 'color-mix(in oklch, var(--color-warning, #f59e0b) 18%, transparent)',
              color: 'oklch(0.82 0.12 75)',
              letterSpacing: '0.08em',
            }}
          >
            {tag}
          </span>
        )}
      </div>
      {right}
    </div>
  );
}

function NumberedNavItem({ item, active }: { item: NumberedItem; active: boolean }) {
  const [hover, setHover] = useState(false);
  return (
    <Link
      to={item.to}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      className="relative flex items-center"
      style={{
        gap: 12,
        padding: '9px 10px',
        borderRadius: 8,
        background: active
          ? 'oklch(0.25 0.02 260)'
          : hover
          ? 'oklch(0.21 0.018 260)'
          : 'transparent',
        color: active ? '#fff' : 'oklch(0.85 0.01 260)',
        transition: 'background 120ms, color 120ms',
      }}
    >
      {active && (
        <span
          aria-hidden="true"
          style={{
            position: 'absolute',
            left: 0,
            top: 10,
            bottom: 10,
            width: 2,
            borderRadius: 2,
            background: 'var(--color-primary-400, #818cf8)',
            boxShadow: '0 0 8px var(--color-primary-500, #6366f1)',
          }}
        />
      )}
      <span
        className="sv3-num font-bold"
        style={{
          fontSize: 10,
          color: active ? 'var(--color-primary-300, #a5b4fc)' : 'oklch(0.45 0.015 260)',
          width: 18,
          letterSpacing: '-0.02em',
        }}
      >
        {item.num}
      </span>
      <item.Icon size={15} />
      <span
        className="flex-1"
        style={{
          fontSize: 13.5,
          fontWeight: active ? 600 : 500,
          letterSpacing: '-0.01em',
        }}
      >
        {item.label}
      </span>
      {hover && item.shortcut && (
        <kbd
          className="sv3-num font-semibold"
          style={{
            fontSize: 9.5,
            padding: '1px 5px',
            borderRadius: 3,
            background: 'oklch(0.16 0.018 260)',
            color: 'oklch(0.70 0.01 260)',
            border: '1px solid oklch(0.26 0.018 260)',
          }}
        >
          {item.shortcut}
        </kbd>
      )}
    </Link>
  );
}

function FlatNavItem({ item, active }: { item: FlatItem; active: boolean }) {
  const [hover, setHover] = useState(false);
  return (
    <Link
      to={item.to}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      className="relative flex items-center"
      style={{
        gap: 10,
        padding: '7px 10px',
        borderRadius: 7,
        background: active
          ? 'oklch(0.25 0.02 260)'
          : hover
          ? 'oklch(0.21 0.018 260)'
          : 'transparent',
        color: active ? '#fff' : 'oklch(0.75 0.01 260)',
        transition: 'background 120ms',
      }}
    >
      <item.Icon size={14} />
      <span className="flex-1" style={{ fontSize: 12.5, fontWeight: active ? 600 : 500 }}>
        {item.label}
      </span>
      {item.badge && item.badge > 0 && (
        <span
          className="sv3-num inline-flex items-center justify-center font-bold"
          style={{
            minWidth: 16,
            height: 16,
            padding: '0 5px',
            fontSize: 10,
            borderRadius: 8,
            background: 'var(--color-danger, #ef4444)',
            color: '#fff',
          }}
        >
          {item.badge}
        </span>
      )}
    </Link>
  );
}

function LotFloorMap({
  freeSlots,
  totalSlots,
  freePct,
  floors,
  onOpenMap,
}: {
  lotName: string;
  freeSlots: number;
  totalSlots: number;
  freePct: number;
  floors: FloorSummary[];
  onOpenMap: () => void;
}) {
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);
  const badgeColor =
    freePct < 0.15 ? 'oklch(0.78 0.14 25)' : freePct < 0.3 ? 'oklch(0.82 0.12 75)' : 'oklch(0.80 0.14 150)';
  const badgeVar =
    freePct < 0.15 ? 'var(--color-danger, #ef4444)' : freePct < 0.3 ? 'var(--color-warning, #f59e0b)' : 'var(--color-success, #10b981)';

  return (
    <div
      className="relative overflow-hidden"
      style={{
        borderRadius: 12,
        padding: 12,
        border: '1px solid oklch(0.26 0.018 260)',
        background: 'oklch(0.20 0.018 260)',
      }}
    >
      <div className="flex items-baseline justify-between" style={{ marginBottom: 10 }}>
        <div>
          <div className="sv3-display" style={{ fontSize: 20, lineHeight: 1 }}>
            {freeSlots}
            <span
              style={{ fontSize: 12, color: 'oklch(0.55 0.015 260)', fontWeight: 500, marginLeft: 4 }}
            >
              / {totalSlots}
            </span>
          </div>
          <div
            className="uppercase font-semibold"
            style={{
              fontSize: 10.5,
              color: 'oklch(0.60 0.015 260)',
              letterSpacing: '0.08em',
              marginTop: 2,
            }}
          >
            slots open
          </div>
        </div>
        <div
          className="sv3-num font-bold"
          style={{
            fontSize: 11,
            color: badgeColor,
            padding: '2px 6px',
            borderRadius: 4,
            background: `color-mix(in oklch, ${badgeVar} 18%, transparent)`,
          }}
        >
          {Math.round(freePct * 100)}%
        </div>
      </div>

      {floors.length > 0 ? (
        <div className="flex flex-col" style={{ gap: 2 }}>
          {floors.map((floor, i) => {
            const color =
              floor.occupancy > 0.85
                ? 'oklch(0.58 0.16 25)'
                : floor.occupancy > 0.6
                ? 'oklch(0.70 0.14 75)'
                : 'oklch(0.58 0.14 150)';
            const isMe = !!floor.youLabel;
            return (
              <button
                key={floor.label}
                type="button"
                onMouseEnter={() => setHoverIndex(i)}
                onMouseLeave={() => setHoverIndex(null)}
                onClick={onOpenMap}
                style={{
                  display: 'grid',
                  gridTemplateColumns: '22px 1fr auto',
                  alignItems: 'center',
                  gap: 8,
                  padding: '4px 2px',
                  borderRadius: 4,
                  background: hoverIndex === i ? 'oklch(0.24 0.02 260)' : 'transparent',
                  textAlign: 'left',
                }}
              >
                <span
                  className="sv3-num font-bold"
                  style={{
                    fontSize: 10.5,
                    color: isMe ? 'var(--color-primary-300, #a5b4fc)' : 'oklch(0.55 0.015 260)',
                  }}
                >
                  {floor.label}
                </span>
                <div
                  className="relative overflow-hidden"
                  style={{
                    height: 6,
                    borderRadius: 2,
                    background: 'oklch(0.16 0.02 260)',
                  }}
                >
                  <div
                    style={{
                      width: `${Math.min(100, floor.occupancy * 100)}%`,
                      height: '100%',
                      background: color,
                      boxShadow: isMe ? `0 0 8px ${color}` : 'none',
                    }}
                  />
                  {isMe && (
                    <span
                      aria-hidden="true"
                      style={{
                        position: 'absolute',
                        left: `${Math.min(100, floor.occupancy * 100)}%`,
                        top: -1,
                        bottom: -1,
                        width: 2,
                        background: 'var(--color-primary-300, #a5b4fc)',
                        transform: 'translateX(-1px)',
                        boxShadow: '0 0 8px var(--color-primary-400, #818cf8)',
                      }}
                    />
                  )}
                </div>
                <span
                  className="sv3-num"
                  style={{
                    fontSize: 10,
                    color: isMe ? 'var(--color-primary-300, #a5b4fc)' : 'oklch(0.55 0.015 260)',
                    fontWeight: isMe ? 700 : 500,
                    minWidth: 28,
                    textAlign: 'right',
                  }}
                >
                  {isMe ? 'you' : `${Math.round(floor.occupancy * 100)}%`}
                </span>
              </button>
            );
          })}
        </div>
      ) : null}

      <button
        type="button"
        onClick={onOpenMap}
        className="sv3-hover inline-flex items-center justify-center font-semibold"
        style={{
          marginTop: 10,
          width: '100%',
          padding: '7px 10px',
          fontSize: 11.5,
          gap: 6,
          borderRadius: 7,
          background: 'oklch(0.26 0.02 260)',
          color: 'oklch(0.92 0.005 260)',
        }}
      >
        <MapPin size={11} /> Open floor map
      </button>
    </div>
  );
}

function EmptyPassCard() {
  return (
    <div
      className="flex items-center justify-between"
      style={{
        padding: '12px 14px',
        borderRadius: 14,
        background: 'oklch(0.22 0.018 260)',
        border: '1px dashed oklch(0.30 0.018 260)',
      }}
    >
      <div>
        <div
          className="uppercase font-semibold"
          style={{
            fontSize: 10,
            letterSpacing: '0.14em',
            color: 'oklch(0.60 0.015 260)',
          }}
        >
          No active booking
        </div>
        <div className="font-semibold" style={{ fontSize: 12.5, marginTop: 2, color: 'oklch(0.85 0.01 260)' }}>
          You&apos;re free to go
        </div>
      </div>
      <Link
        to="/book"
        className="inline-flex items-center justify-center font-bold"
        style={{
          padding: '6px 10px',
          fontSize: 11.5,
          borderRadius: 7,
          background:
            'linear-gradient(135deg, var(--color-primary-400, #818cf8), var(--color-primary-600, #4f46e5))',
          color: '#fff',
        }}
      >
        Book now
      </Link>
    </div>
  );
}

function WhatsNextCard({ upNext }: { upNext: Booking | undefined }) {
  const subtitle = upNext ? formatUpNext(upNext) : 'Nothing scheduled';
  return (
    <div
      style={{
        padding: '10px 14px 14px',
        borderTop: '1px solid oklch(0.24 0.015 260)',
      }}
    >
      <div
        className="flex items-center"
        style={{
          gap: 10,
          padding: '10px 12px',
          borderRadius: 10,
          background:
            'linear-gradient(135deg, color-mix(in oklch, var(--color-primary-500, #6366f1) 12%, oklch(0.22 0.02 260)) 0%, oklch(0.22 0.02 260) 100%)',
          border: '1px solid oklch(0.26 0.018 260)',
        }}
      >
        <div
          className="flex items-center justify-center"
          style={{
            width: 30,
            height: 30,
            borderRadius: 8,
            background:
              'color-mix(in oklch, var(--color-primary-500, #6366f1) 24%, oklch(0.22 0.02 260))',
            color: 'var(--color-primary-300, #a5b4fc)',
          }}
        >
          <CalendarPlus size={14} />
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            className="uppercase font-bold"
            style={{
              fontSize: 10,
              letterSpacing: '0.08em',
              color: 'oklch(0.62 0.015 260)',
            }}
          >
            Up next
          </div>
          <div
            className="font-semibold truncate"
            style={{ fontSize: 12.5, color: 'oklch(0.92 0.005 260)', marginTop: 1 }}
          >
            {subtitle}
          </div>
        </div>
        <Link
          to="/book"
          className="sv3-hover flex items-center justify-center"
          style={{
            width: 26,
            height: 26,
            borderRadius: 6,
            background: 'oklch(0.30 0.02 260)',
            color: 'oklch(0.92 0.005 260)',
          }}
          title="Book"
          aria-label="Book"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M5 12h14M13 6l6 6-6 6" />
          </svg>
        </Link>
      </div>
    </div>
  );
}

function UserMenu({
  userName,
  userEmail,
  userRole,
  onClose,
  onLogout,
  onOpenAssistant,
}: {
  userName: string;
  userEmail?: string;
  userRole?: string;
  onClose: () => void;
  onLogout: () => void;
  onOpenAssistant: () => void;
}) {
  const navigate = useNavigate();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handler(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    }
    // Defer one tick so the click that opened the menu doesn't immediately close it.
    const id = window.setTimeout(() => document.addEventListener('mousedown', handler), 0);
    return () => {
      window.clearTimeout(id);
      document.removeEventListener('mousedown', handler);
    };
  }, [onClose]);

  const roleLabel = userRole ? userRole.charAt(0).toUpperCase() + userRole.slice(1) : 'User';

  return (
    <div
      ref={ref}
      role="menu"
      className="absolute z-50"
      style={{
        top: 54,
        right: 14,
        width: 220,
        background: 'oklch(0.22 0.018 260)',
        border: '1px solid oklch(0.28 0.02 260)',
        borderRadius: 10,
        padding: 6,
        boxShadow: '0 20px 40px -12px rgba(0,0,0,0.6)',
        color: 'oklch(0.92 0.005 260)',
      }}
    >
      <div
        style={{
          padding: '8px 10px 10px',
          borderBottom: '1px solid oklch(0.28 0.02 260)',
          marginBottom: 6,
        }}
      >
        <div className="font-bold truncate" style={{ fontSize: 13 }}>
          {userName}
        </div>
        <div
          className="truncate"
          style={{ fontSize: 10.5, color: 'oklch(0.62 0.015 260)', marginTop: 2 }}
        >
          {userEmail ? `${userEmail} · ${roleLabel}` : roleLabel}
        </div>
      </div>
      <button
        type="button"
        role="menuitem"
        onClick={() => {
          onClose();
          navigate('/profile');
        }}
        className="sv3-hover w-full flex items-center text-left"
        style={{ gap: 10, padding: '7px 10px', borderRadius: 6, fontSize: 12.5, color: 'oklch(0.90 0.005 260)' }}
      >
        <User size={14} /> Profile
      </button>
      <button
        type="button"
        role="menuitem"
        onClick={() => {
          onClose();
          navigate('/profile');
        }}
        className="sv3-hover w-full flex items-center text-left"
        style={{ gap: 10, padding: '7px 10px', borderRadius: 6, fontSize: 12.5, color: 'oklch(0.90 0.005 260)' }}
      >
        <GearSix size={14} /> Preferences
      </button>
      <button
        type="button"
        role="menuitem"
        onClick={onOpenAssistant}
        className="sv3-hover w-full flex items-center text-left"
        style={{ gap: 10, padding: '7px 10px', borderRadius: 6, fontSize: 12.5, color: 'oklch(0.90 0.005 260)' }}
      >
        <Sparkle size={14} /> Assistant
      </button>
      <div style={{ height: 1, background: 'oklch(0.28 0.02 260)', margin: '6px 0' }} />
      <button
        type="button"
        role="menuitem"
        onClick={onLogout}
        className="sv3-hover w-full flex items-center text-left"
        style={{ gap: 10, padding: '7px 10px', borderRadius: 6, fontSize: 12.5, color: 'oklch(0.80 0.12 25)' }}
      >
        <SignOut size={14} /> Sign out
      </button>
    </div>
  );
}

export default SidebarV3;
