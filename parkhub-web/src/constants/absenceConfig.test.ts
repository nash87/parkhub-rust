import { describe, it, expect, vi } from 'vitest';

vi.mock('@phosphor-icons/react', () => ({
  House: 'House',
  Airplane: 'Airplane',
  FirstAidKit: 'FirstAidKit',
  Briefcase: 'Briefcase',
  NoteBlank: 'NoteBlank',
}));

import { ABSENCE_CONFIG, type AbsenceType } from './absenceConfig';

describe('ABSENCE_CONFIG', () => {
  const ALL_TYPES: AbsenceType[] = ['homeoffice', 'vacation', 'sick', 'business_trip', 'other'];

  it('defines all 5 absence types', () => {
    expect(Object.keys(ABSENCE_CONFIG)).toEqual(ALL_TYPES);
  });

  it('each type has icon, color, bg, and dot', () => {
    for (const type of ALL_TYPES) {
      const config = ABSENCE_CONFIG[type];
      expect(config.icon).toBeDefined();
      expect(config.color).toBeTruthy();
      expect(config.bg).toBeTruthy();
      expect(config.dot).toBeTruthy();
    }
  });

  it('homeoffice uses primary colors', () => {
    expect(ABSENCE_CONFIG.homeoffice.color).toContain('primary');
    expect(ABSENCE_CONFIG.homeoffice.bg).toContain('primary');
    expect(ABSENCE_CONFIG.homeoffice.dot).toContain('primary');
  });

  it('vacation uses orange colors', () => {
    expect(ABSENCE_CONFIG.vacation.color).toContain('orange');
    expect(ABSENCE_CONFIG.vacation.bg).toContain('orange');
    expect(ABSENCE_CONFIG.vacation.dot).toContain('orange');
  });

  it('sick uses red colors', () => {
    expect(ABSENCE_CONFIG.sick.color).toContain('red');
    expect(ABSENCE_CONFIG.sick.bg).toContain('red');
    expect(ABSENCE_CONFIG.sick.dot).toContain('red');
  });

  it('business_trip uses purple colors', () => {
    expect(ABSENCE_CONFIG.business_trip.color).toContain('purple');
    expect(ABSENCE_CONFIG.business_trip.bg).toContain('purple');
    expect(ABSENCE_CONFIG.business_trip.dot).toContain('purple');
  });

  it('other uses surface colors', () => {
    expect(ABSENCE_CONFIG.other.color).toContain('surface');
    expect(ABSENCE_CONFIG.other.bg).toContain('surface');
    expect(ABSENCE_CONFIG.other.dot).toContain('surface');
  });

  it('icons map to phosphor icon components', () => {
    expect(ABSENCE_CONFIG.homeoffice.icon).toBe('House');
    expect(ABSENCE_CONFIG.vacation.icon).toBe('Airplane');
    expect(ABSENCE_CONFIG.sick.icon).toBe('FirstAidKit');
    expect(ABSENCE_CONFIG.business_trip.icon).toBe('Briefcase');
    expect(ABSENCE_CONFIG.other.icon).toBe('NoteBlank');
  });

  it('all colors include dark mode variants', () => {
    for (const type of ALL_TYPES) {
      const config = ABSENCE_CONFIG[type];
      expect(config.color).toContain('dark:');
      expect(config.bg).toContain('dark:');
    }
  });
});
