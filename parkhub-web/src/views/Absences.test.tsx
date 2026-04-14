import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockListAbsences = vi.fn();
const mockGetAbsencePattern = vi.fn();
const mockCreateAbsence = vi.fn();
const mockDeleteAbsence = vi.fn();
const mockSetAbsencePattern = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    listAbsences: (...a: any[]) => mockListAbsences(...a),
    getAbsencePattern: (...a: any[]) => mockGetAbsencePattern(...a),
    createAbsence: (...a: any[]) => mockCreateAbsence(...a),
    deleteAbsence: (...a: any[]) => mockDeleteAbsence(...a),
    setAbsencePattern: (...a: any[]) => mockSetAbsencePattern(...a),
  },
}));

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string, f?: string) => f || k }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { House: C, Calendar: C, CalendarCheck: C, Trash: C, Plus: C, CaretLeft: C, CaretRight: C, X: C, Airplane: C, FirstAidKit: C, Briefcase: C, NoteBlank: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));
vi.mock('../constants/absenceConfig', () => ({
  ABSENCE_CONFIG: {
    homeoffice: { icon: (p: any) => <span {...p} />, color: 'text-blue-600', bg: 'bg-blue-100', dot: 'bg-blue-500' },
    vacation: { icon: (p: any) => <span {...p} />, color: 'text-orange-600', bg: 'bg-orange-100', dot: 'bg-orange-500' },
    sick: { icon: (p: any) => <span {...p} />, color: 'text-red-600', bg: 'bg-red-100', dot: 'bg-red-500' },
    business_trip: { icon: (p: any) => <span {...p} />, color: 'text-purple-600', bg: 'bg-purple-100', dot: 'bg-purple-500' },
    other: { icon: (p: any) => <span {...p} />, color: 'text-gray-600', bg: 'bg-gray-100', dot: 'bg-gray-500' },
  },
}));

import { AbsencesPage } from './Absences';
import toast from 'react-hot-toast';

const todayStr = new Date().toISOString().slice(0, 10);
const futureStr = new Date(Date.now() + 7 * 86400000).toISOString().slice(0, 10);
const pastStr = new Date(Date.now() - 7 * 86400000).toISOString().slice(0, 10);

const entries = [
  { id: 'e1', absence_type: 'homeoffice', start_date: todayStr, end_date: todayStr, note: null, created_at: todayStr },
  { id: 'e2', absence_type: 'vacation', start_date: futureStr, end_date: futureStr, note: 'Holiday', created_at: todayStr },
  { id: 'e3', absence_type: 'sick', start_date: pastStr, end_date: pastStr, note: null, created_at: pastStr },
];

const patterns = [
  { id: 'p1', absence_type: 'homeoffice', weekdays: [0, 2], user_id: 'u1' },
];

describe('AbsencesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListAbsences.mockResolvedValue({ success: true, data: entries });
    mockGetAbsencePattern.mockResolvedValue({ success: true, data: patterns });
    mockCreateAbsence.mockResolvedValue({ success: true, data: { id: 'new', absence_type: 'homeoffice', start_date: todayStr, end_date: todayStr, note: null, created_at: todayStr } });
    mockDeleteAbsence.mockResolvedValue({ success: true });
    mockSetAbsencePattern.mockResolvedValue({ success: true, data: { id: 'p1', absence_type: 'homeoffice', weekdays: [0, 2, 4], user_id: 'u1' } });
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders absences page', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByText('Abwesenheiten')).toBeInTheDocument());
  });

  it('shows upcoming entries', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByText('Anstehend')).toBeInTheDocument());
  });

  it('opens add absence modal', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getAllByText('Eintragen').length).toBeGreaterThan(0));
    fireEvent.click(screen.getAllByText('Eintragen')[0]);
    await waitFor(() => expect(screen.getByText('Abwesenheit eintragen')).toBeInTheDocument());
  });

  it('closes add modal', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getAllByText('Eintragen').length).toBeGreaterThan(0));
    fireEvent.click(screen.getAllByText('Eintragen')[0]);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByLabelText('Close'));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('creates absence', async () => {
    const user = userEvent.setup();
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getAllByText('Eintragen').length).toBeGreaterThan(0));
    await user.click(screen.getAllByText('Eintragen')[0]);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Heute'));
    // The submit button inside the modal dialog
    const btns = screen.getAllByText('Eintragen');
    const submitBtn = btns.find(el => el.closest('[role="dialog"]') && el.closest('button'));
    if (submitBtn) await user.click(submitBtn);
    await waitFor(() => expect(mockCreateAbsence).toHaveBeenCalled());
    expect(toast.success).toHaveBeenCalledWith('Abwesenheit eingetragen');
  });

  it('create absence failure', async () => {
    mockCreateAbsence.mockResolvedValue({ success: false, error: { message: 'Overlap' } });
    const user = userEvent.setup();
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getAllByText('Eintragen').length).toBeGreaterThan(0));
    await user.click(screen.getAllByText('Eintragen')[0]);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Heute'));
    const btns = screen.getAllByText('Eintragen');
    const submitBtn = btns.find(el => el.closest('[role="dialog"]') && el.closest('button'));
    if (submitBtn) await user.click(submitBtn);
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Overlap'));
  });

  it('deletes absence entry', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByText('Anstehend')).toBeInTheDocument());
    const delBtns = screen.getAllByLabelText('Delete absence entry');
    fireEvent.click(delBtns[0]);
    await waitFor(() => expect(mockDeleteAbsence).toHaveBeenCalled());
    expect(toast.success).toHaveBeenCalledWith('Abwesenheit gelöscht');
  });

  it('toggles homeoffice pattern', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByText('Homeoffice-Muster')).toBeInTheDocument());
    // Expand pattern section
    fireEvent.click(screen.getByText('Homeoffice-Muster'));
    await waitFor(() => expect(screen.getByText('Wähle deine festen Homeoffice-Tage')).toBeInTheDocument());
    // Click a weekday button
    const dayBtns = screen.getAllByRole('button').filter(b => b.getAttribute('aria-pressed') !== null);
    if (dayBtns.length > 0) fireEvent.click(dayBtns[0]);
    await waitFor(() => expect(mockSetAbsencePattern).toHaveBeenCalled());
  });

  it('navigates calendar months', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByLabelText('Previous month')).toBeInTheDocument());
    fireEvent.click(screen.getByLabelText('Previous month'));
    fireEvent.click(screen.getByLabelText('Next month'));
    fireEvent.click(screen.getByText('absences.today'));
  });

  it('shows empty entries message', async () => {
    mockListAbsences.mockResolvedValue({ success: true, data: [] });
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByText('Keine Einträge')).toBeInTheDocument());
  });

  it('handles data load error', async () => {
    mockListAbsences.mockRejectedValue(new Error('net'));
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByText('Abwesenheiten')).toBeInTheDocument());
  });

  it('navigates to January from December and vice versa', async () => {
    render(<AbsencesPage />);
    await waitFor(() => expect(screen.getByLabelText('Previous month')).toBeInTheDocument());
    // Click previous month many times to cross year boundary
    for (let i = 0; i < 13; i++) {
      fireEvent.click(screen.getByLabelText('Previous month'));
    }
    // Click next month many times to cross year boundary forward
    for (let i = 0; i < 13; i++) {
      fireEvent.click(screen.getByLabelText('Next month'));
    }
  });
});
