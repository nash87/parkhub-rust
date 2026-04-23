import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import { OnboardingTour, hasSeenOnboardingTour, markOnboardingTourSeen } from './OnboardingTour';

// Minimal i18n mock — avoids booting the real i18next stack for these tests.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
    i18n: { changeLanguage: vi.fn() },
  }),
}));

function renderTour() {
  return render(
    <MemoryRouter initialEntries={['/tour']}>
      <Routes>
        <Route path="/tour" element={<OnboardingTour />} />
        <Route path="/" element={<div data-testid="home">Home</div>} />
      </Routes>
    </MemoryRouter>
  );
}

describe('OnboardingTour', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });
  afterEach(() => cleanup());

  it('starts on the Privacy step', () => {
    renderTour();
    expect(screen.getByTestId('onboarding-tour')).toBeInTheDocument();
    expect(screen.getByText(/Volle Transparenz zu Ihren Daten/i)).toBeInTheDocument();
  });

  it('advances through all three steps', () => {
    renderTour();
    // step 1 → 2
    fireEvent.click(screen.getByRole('button', { name: /Weiter/i }));
    expect(screen.getByText(/Features die Sie brauchen/i)).toBeInTheDocument();
    // step 2 → 3
    fireEvent.click(screen.getByRole('button', { name: /Weiter/i }));
    expect(screen.getByText(/Warum Sie ParkHub vertrauen können/i)).toBeInTheDocument();
  });

  it('can go back to a previous step', () => {
    renderTour();
    fireEvent.click(screen.getByRole('button', { name: /Weiter/i }));
    expect(screen.getByText(/Features die Sie brauchen/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Zurück/i }));
    expect(screen.getByText(/Volle Transparenz zu Ihren Daten/i)).toBeInTheDocument();
  });

  it('persists the tour-seen flag after Finish', () => {
    renderTour();
    fireEvent.click(screen.getByRole('button', { name: /Weiter/i }));
    fireEvent.click(screen.getByRole('button', { name: /Weiter/i }));
    fireEvent.click(screen.getByRole('button', { name: /Los geht/i }));
    expect(hasSeenOnboardingTour()).toBe(true);
  });

  it('persists the tour-seen flag on Skip', () => {
    renderTour();
    fireEvent.click(screen.getByRole('button', { name: /Überspringen/i }));
    expect(hasSeenOnboardingTour()).toBe(true);
  });

  it('feature toggles persist to localStorage', () => {
    renderTour();
    // advance to the features step
    fireEvent.click(screen.getByRole('button', { name: /Weiter/i }));
    // Pflicht-Features don't render a clickable toggle for "deactivate"
    const switches = screen.getAllByRole('switch');
    // there should be one switch per non-required feature
    expect(switches.length).toBeGreaterThan(0);
    fireEvent.click(switches[0]);
    const stored = JSON.parse(window.localStorage.getItem('parkhub_onboarding_v5_prefs') || '{}');
    expect(Object.keys(stored).length).toBeGreaterThan(0);
  });
});

describe('hasSeenOnboardingTour / markOnboardingTourSeen', () => {
  beforeEach(() => window.localStorage.clear());

  it('defaults to false', () => {
    expect(hasSeenOnboardingTour()).toBe(false);
  });

  it('returns true after mark', () => {
    markOnboardingTourSeen();
    expect(hasSeenOnboardingTour()).toBe(true);
  });
});
