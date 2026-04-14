import { useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

/** Maps route paths to i18n nav keys for document.title */
const ROUTE_TITLE_MAP: Record<string, string> = {
  '/': 'nav.dashboard',
  '/book': 'book.title',
  '/bookings': 'bookings.title',
  '/credits': 'credits.title',
  '/vehicles': 'vehicles.title',
  '/favorites': 'favorites.title',
  '/absences': 'absences.title',
  '/profile': 'profile.title',
  '/team': 'team.title',
  '/notifications': 'notifications.title',
  '/calendar': 'calendar.title',
  '/visitors': 'visitors.title',
  '/ev-charging': 'evCharging.title',
  '/history': 'history.title',
  '/absence-approval': 'absenceApproval.title',
  '/map': 'map.title',
  '/swap-requests': 'swap.title',
  '/checkin': 'nav.checkin',
  '/guest-pass': 'guestBooking.title',
  '/leaderboard': 'nav.leaderboard',
  '/predict': 'nav.predictions',
  '/translations': 'translations.title',
  '/admin': 'admin.title',
};

/**
 * Sets document.title based on the current route.
 * Falls back to "ParkHub" for unmatched routes.
 */
export function usePageTitle() {
  const { pathname } = useLocation();
  const { t } = useTranslation();

  useEffect(() => {
    // Match exact path first, then try parent path for nested routes like /admin/*
    const key = ROUTE_TITLE_MAP[pathname]
      || ROUTE_TITLE_MAP['/' + pathname.split('/')[1]]
      || null;

    if (key) {
      const pageName = t(key);
      document.title = `${pageName} — ParkHub`;
    } else {
      document.title = 'ParkHub';
    }
  }, [pathname, t]);
}
