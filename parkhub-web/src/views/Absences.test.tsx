import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockListAbsences = vi.fn();
const mockGetAbsencePattern = vi.fn();
const mockCreateAbsence = vi.fn();
const mockDeleteAbsence = vi.fn();
const mockSetAbsencePattern = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    listAbsences: (...args: any[]) => mockListAbsences(...args),
    getAbsencePattern: (...args: any[]) => mockGetAbsencePattern(...args),
    createAbsence: (...args: any[]) => mockCreateAbsence(...args),
    deleteAbsence: (...args: any[]) => mockDeleteAbsence(...args),
    setAbsencePattern: (...args: any[]) => mockSetAbsencePattern(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'absences.title': 'Abwesenheiten',
        'absences.subtitle': 'Homeoffice, Urlaub & mehr verwalten',
        'absences.addAbsence': 'Eintragen',
        'absences.weeklyPattern': 'Homeoffice-Muster',
        'absences.patternDesc': 'Wähle deine festen Homeoffice-Tage',
        'absences.upcoming': 'Anstehend',
        'absences.noEntries': 'Keine Einträge',
        'absences.deleted': 'Abwesenheit gelöscht',
        'absences.added': 'Abwesenheit eingetragen',
        'absences.patternUpdated': 'Muster aktualisiert',
        'absences.types.homeoffice': 'Homeoffice',
        'absences.types.vacation': 'Urlaub',
        'absences.types.sick': 'Krank',
        'absences.types.business_trip': 'Dienstreise',
        'absences.types.other': 'Sonstiges',
        'absences.quickToday': 'Heute',
        'absences.startDate': 'Von',
        'absences.endDate': 'Bis',
        'absences.notePlaceholder': 'Notiz (optional)',
        'absences.addBtn': 'Eintragen',
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
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  House: (props: any) => <span data-testid="icon-house" {...props} />,
  Calendar: (props: any) => <span data-testid="icon-calendar" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-calendar-check" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  CaretLeft: (props: any) => <span data-testid="icon-caret-left" {...props} />,
  CaretRight: (props: any) => <span data-testid="icon-caret-right" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
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

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

import { AbsencesPage } from './Absences';

describe('AbsencesPage', () => {
  beforeEach(() => {
    mockListAbsences.mockClear();
    mockGetAbsencePattern.mockClear();
    mockCreateAbsence.mockClear();
    mockDeleteAbsence.mockClear();
    mockSetAbsencePattern.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    mockListAbsences.mockReturnValue(new Promise(() => {}));
    mockGetAbsencePattern.mockReturnValue(new Promise(() => {}));
    render(<AbsencesPage />);
    const skeletons = document.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('renders heading and subtitle after loading', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);

    await waitFor(() => {
      expect(screen.getByText('Abwesenheiten')).toBeInTheDocument();
    });
    expect(screen.getByText('Homeoffice, Urlaub & mehr verwalten')).toBeInTheDocument();
  });

  it('renders add button', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);

    await waitFor(() => {
      expect(screen.getByText('Eintragen')).toBeInTheDocument();
    });
  });

  it('renders calendar with weekday headers', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);

    await waitFor(() => {
      expect(screen.getByText('Mo')).toBeInTheDocument();
    });
    expect(screen.getByText('Di')).toBeInTheDocument();
    expect(screen.getByText('Mi')).toBeInTheDocument();
    expect(screen.getByText('Do')).toBeInTheDocument();
    expect(screen.getByText('Fr')).toBeInTheDocument();
    expect(screen.getByText('Sa')).toBeInTheDocument();
    expect(screen.getByText('So')).toBeInTheDocument();
  });

  it('renders Homeoffice pattern section', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);

    await waitFor(() => {
      expect(screen.getByText('Homeoffice-Muster')).toBeInTheDocument();
    });
  });

  it('renders upcoming entries section', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);

    await waitFor(() => {
      expect(screen.getByText('Anstehend')).toBeInTheDocument();
    });
  });

  it('shows empty state for upcoming entries when none exist', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);

    await waitFor(() => {
      expect(screen.getByText('Keine Einträge')).toBeInTheDocument();
    });
  });

  it('renders legend with absence types', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);

    await waitFor(() => {
      expect(screen.getByText('Homeoffice')).toBeInTheDocument();
    });
    expect(screen.getByText('Urlaub')).toBeInTheDocument();
    expect(screen.getByText('Krank')).toBeInTheDocument();
    expect(screen.getByText('Dienstreise')).toBeInTheDocument();
    expect(screen.getByText('Sonstiges')).toBeInTheDocument();
  });

  it('renders upcoming absence entries', async () => {
    const futureDate = new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
    mockListAbsences.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'abs-1',
          user_id: 'u-1',
          absence_type: 'vacation',
          start_date: futureDate,
          end_date: futureDate,
          source: 'manual',
          created_at: '2026-03-01T00:00:00Z',
        },
      ],
    });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });

    render(<AbsencesPage />);

    await waitFor(() => {
      // "Urlaub" appears in both the legend and the upcoming entry
      expect(screen.getAllByText('Urlaub').length).toBeGreaterThanOrEqual(2);
    });
  });

  it('opens add absence modal', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AbsencesPage />);

    await waitFor(() => {
      // There may be multiple "Eintragen" texts; click the button one
      const addButtons = screen.getAllByText('Eintragen');
      expect(addButtons.length).toBeGreaterThan(0);
    });

    const addButton = screen.getAllByText('Eintragen')[0].closest('button');
    await user.click(addButton!);

    // Modal should show date fields
    await waitFor(() => {
      expect(screen.getByText('Von')).toBeInTheDocument();
    });
    expect(screen.getByText('Bis')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Notiz (optional)')).toBeInTheDocument();
  });
});
