import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { WifiSlash, ArrowDown, House, CalendarBlank, Car, User } from '@phosphor-icons/react';
import { useNavigate, useLocation } from 'react-router-dom';

// ─────────────────────────────────────────────────────────────────────────────
// Offline Indicator
// ─────────────────────────────────────────────────────────────────────────────

/** Shows a banner when the user goes offline. */
export function OfflineIndicator() {
  const { t } = useTranslation();
  const [isOffline, setIsOffline] = useState(!navigator.onLine);

  useEffect(() => {
    const goOffline = () => setIsOffline(true);
    const goOnline = () => setIsOffline(false);
    window.addEventListener('offline', goOffline);
    window.addEventListener('online', goOnline);
    return () => {
      window.removeEventListener('offline', goOffline);
      window.removeEventListener('online', goOnline);
    };
  }, []);

  if (!isOffline) return null;

  return (
    <div className="fixed top-0 left-0 right-0 z-50 bg-amber-500 text-white text-center py-2 px-4 text-sm font-medium flex items-center justify-center gap-2" role="alert">
      <WifiSlash size={18} weight="bold" />
      {t('pwa.offlineMessage')}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Cached Booking Display
// ─────────────────────────────────────────────────────────────────────────────

interface CachedBooking {
  id: string;
  lot_name: string;
  slot_label: string;
  date: string;
  start_time: string;
  end_time: string;
}

/** Shows the cached next booking when offline. */
export function CachedBookingCard() {
  const { t } = useTranslation();
  const [booking, setBooking] = useState<CachedBooking | null>(null);
  const [isOffline, setIsOffline] = useState(!navigator.onLine);

  useEffect(() => {
    const goOffline = () => setIsOffline(true);
    const goOnline = () => setIsOffline(false);
    window.addEventListener('offline', goOffline);
    window.addEventListener('online', goOnline);
    return () => {
      window.removeEventListener('offline', goOffline);
      window.removeEventListener('online', goOnline);
    };
  }, []);

  useEffect(() => {
    // Try to load from cached API response
    const cached = localStorage.getItem('parkhub_offline_data');
    if (cached) {
      try {
        const data = JSON.parse(cached);
        if (data.next_booking) {
          setBooking(data.next_booking);
        }
      } catch { /* ignore parse errors */ }
    }

    // Refresh offline data when online
    if (navigator.onLine) {
      fetch('/api/v1/pwa/offline-data')
        .then(r => r.json())
        .then(res => {
          if (res.success && res.data) {
            localStorage.setItem('parkhub_offline_data', JSON.stringify(res.data));
            if (res.data.next_booking) {
              setBooking(res.data.next_booking);
            }
          }
        })
        .catch(() => {});
    }
  }, []);

  if (!isOffline || !booking) return null;

  return (
    <div className="bg-primary-50 dark:bg-primary-950/30 border border-primary-200 dark:border-primary-800 rounded-xl p-4 mb-4">
      <h3 className="text-sm font-semibold text-primary-700 dark:text-primary-300 mb-2">{t('pwa.nextBooking')}</h3>
      <div className="text-sm text-surface-700 dark:text-surface-300">
        <p className="font-medium">{booking.lot_name} &middot; {booking.slot_label}</p>
        <p className="text-xs text-surface-500 mt-1">{booking.date} {booking.start_time} - {booking.end_time}</p>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bottom Navigation Bar (Mobile)
// ─────────────────────────────────────────────────────────────────────────────

/** Mobile bottom navigation bar — visible only on small screens. */
export function BottomNavBar() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();

  const tabs = [
    { path: '/', icon: House, label: t('nav.dashboard') },
    { path: '/book', icon: CalendarBlank, label: t('nav.book') },
    { path: '/bookings', icon: CalendarBlank, label: t('nav.bookings') },
    { path: '/vehicles', icon: Car, label: t('nav.vehicles') },
    { path: '/profile', icon: User, label: t('nav.profile') },
  ];

  return (
    <nav className="fixed bottom-0 left-0 right-0 z-40 bg-white dark:bg-surface-900 border-t border-surface-200 dark:border-surface-800 md:hidden safe-area-bottom" aria-label={t('pwa.mobileNav')}>
      <div className="flex items-center justify-around h-14">
        {tabs.map(tab => {
          const isActive = location.pathname === tab.path || (tab.path !== '/' && location.pathname.startsWith(tab.path));
          return (
            <button
              key={tab.path}
              onClick={() => navigate(tab.path)}
              className={`flex flex-col items-center gap-0.5 px-3 py-1 rounded-lg transition-colors ${isActive ? 'text-primary-500' : 'text-surface-400 hover:text-surface-600'}`}
              aria-label={tab.label}
              aria-current={isActive ? 'page' : undefined}
            >
              <tab.icon size={22} weight={isActive ? 'fill' : 'regular'} />
              <span className="text-[10px] font-medium leading-tight">{tab.label}</span>
            </button>
          );
        })}
      </div>
    </nav>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Pull to Refresh
// ─────────────────────────────────────────────────────────────────────────────

/** Pull-to-refresh gesture for mobile. */
export function PullToRefresh({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation();
  const [pulling, setPulling] = useState(false);
  const [pullDistance, setPullDistance] = useState(0);
  const threshold = 80;

  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    if (window.scrollY === 0) {
      const startY = e.touches[0].clientY;
      const handleTouchMove = (moveEvent: TouchEvent) => {
        const diff = moveEvent.touches[0].clientY - startY;
        if (diff > 0 && window.scrollY === 0) {
          setPulling(true);
          setPullDistance(Math.min(diff, threshold * 1.5));
          moveEvent.preventDefault();
        }
      };
      const handleTouchEnd = () => {
        if (pullDistance >= threshold) {
          window.location.reload();
        }
        setPulling(false);
        setPullDistance(0);
        document.removeEventListener('touchmove', handleTouchMove);
        document.removeEventListener('touchend', handleTouchEnd);
      };
      document.addEventListener('touchmove', handleTouchMove, { passive: false });
      document.addEventListener('touchend', handleTouchEnd);
    }
  }, [pullDistance, threshold]);

  return (
    <div onTouchStart={handleTouchStart}>
      {pulling && (
        <div className="flex items-center justify-center py-3 text-primary-500 text-sm" style={{ height: pullDistance }}>
          <ArrowDown size={20} className={`transition-transform ${pullDistance >= threshold ? 'rotate-180' : ''}`} />
          <span className="ml-2">{pullDistance >= threshold ? t('pwa.releaseToRefresh') : t('pwa.pullToRefresh')}</span>
        </div>
      )}
      {children}
    </div>
  );
}
