import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

const mockTeamAbsences = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    teamAbsences: (...args: any[]) => mockTeamAbsences(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'team.title': 'Team',
        'team.subtitle': 'Abwesenheiten im Team',
        'team.today': 'Heute abwesend',
        'team.upcoming': 'Kommende Abwesenheiten',
        'team.noAbsencesToday': 'Heute keine Abwesenheiten',
        'team.noUpcoming': 'Keine anstehenden Abwesenheiten',
        'absences.types.homeoffice': 'Homeoffice',
        'absences.types.vacation': 'Urlaub',
      };
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
}));

vi.mock('../constants/absenceConfig', () => ({
  ABSENCE_CONFIG: {
    homeoffice: { icon: (props: any) => <span {...props} />, color: 'text-primary-600', bg: 'bg-primary-100', dot: 'bg-primary-500' },
    vacation: { icon: (props: any) => <span {...props} />, color: 'text-orange-600', bg: 'bg-orange-100', dot: 'bg-orange-500' },
    sick: { icon: (props: any) => <span {...props} />, color: 'text-red-600', bg: 'bg-red-100', dot: 'bg-red-500' },
    business_trip: { icon: (props: any) => <span {...props} />, color: 'text-purple-600', bg: 'bg-purple-100', dot: 'bg-purple-500' },
    other: { icon: (props: any) => <span {...props} />, color: 'text-surface-600', bg: 'bg-surface-100', dot: 'bg-surface-500' },
  },
}));

import { TeamPage } from './Team';

describe('TeamPage', () => {
  beforeEach(() => {
    mockTeamAbsences.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    mockTeamAbsences.mockReturnValue(new Promise(() => {}));
    render(<TeamPage />);
    const skeletons = document.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('renders team heading', async () => {
    mockTeamAbsences.mockResolvedValue({ success: true, data: [] });
    render(<TeamPage />);

    await waitFor(() => {
      expect(screen.getByText('Team')).toBeInTheDocument();
    });
    expect(screen.getByText('Abwesenheiten im Team')).toBeInTheDocument();
  });

  it('shows empty state when no absences', async () => {
    mockTeamAbsences.mockResolvedValue({ success: true, data: [] });
    render(<TeamPage />);

    await waitFor(() => {
      expect(screen.getByText('Heute keine Abwesenheiten')).toBeInTheDocument();
    });
    expect(screen.getByText('Keine anstehenden Abwesenheiten')).toBeInTheDocument();
  });

  it('renders today and upcoming sections', async () => {
    mockTeamAbsences.mockResolvedValue({ success: true, data: [] });
    render(<TeamPage />);

    await waitFor(() => {
      expect(screen.getByText(/Heute abwesend/)).toBeInTheDocument();
    });
    expect(screen.getByText(/Kommende Abwesenheiten/)).toBeInTheDocument();
  });

  it('shows team members with absences today', async () => {
    const todayStr = new Date().toISOString().slice(0, 10);
    mockTeamAbsences.mockResolvedValue({
      success: true,
      data: [
        { user_name: 'Alice', absence_type: 'homeoffice', start_date: todayStr, end_date: todayStr },
        { user_name: 'Bob', absence_type: 'vacation', start_date: todayStr, end_date: todayStr },
      ],
    });

    render(<TeamPage />);

    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });
    expect(screen.getByText('Bob')).toBeInTheDocument();
    expect(screen.getByText('Homeoffice')).toBeInTheDocument();
    expect(screen.getByText('Urlaub')).toBeInTheDocument();
  });

  it('shows upcoming absences in the future', async () => {
    const futureDate = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
    mockTeamAbsences.mockResolvedValue({
      success: true,
      data: [
        { user_name: 'Charlie', absence_type: 'vacation', start_date: futureDate, end_date: futureDate },
      ],
    });

    render(<TeamPage />);

    await waitFor(() => {
      expect(screen.getByText('Charlie')).toBeInTheDocument();
    });
  });
});
