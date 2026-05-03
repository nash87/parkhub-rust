import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('@phosphor-icons/react', () => ({
  CaretRightIcon: (props: any) => <span data-testid="caret-right" {...props} />,
}));

import { Breadcrumb, SEGMENT_LABELS } from './Breadcrumb';

describe('Breadcrumb', () => {
  it('renders nothing on root path', () => {
    const { container } = render(
      <MemoryRouter initialEntries={['/']}>
        <Breadcrumb />
      </MemoryRouter>,
    );
    expect(container.innerHTML).toBe('');
  });

  it('renders crumbs for /admin/users', () => {
    render(
      <MemoryRouter initialEntries={['/admin/users']}>
        <Breadcrumb />
      </MemoryRouter>,
    );
    expect(screen.getByText('nav.dashboard')).toBeInTheDocument();
    expect(screen.getByText('nav.admin')).toBeInTheDocument();
    expect(screen.getByText('admin.users')).toBeInTheDocument();
  });

  it.each([
    ['tenants', 'admin.tenants'],
    ['fleet', 'admin.fleet'],
    ['accessible', 'admin.accessible'],
    ['maintenance', 'admin.maintenance'],
    ['billing', 'admin.billing'],
    ['visitors', 'admin.visitors'],
    ['plugins', 'admin.plugins'],
    ['compliance', 'compliance.title'],
    ['sso', 'admin.sso'],
    ['heatmap', 'heatmap.title'],
  ])('maps /admin/%s to a scalar breadcrumb key', (segment, labelKey) => {
    expect(SEGMENT_LABELS[segment]).toBe(labelKey);
    expect(SEGMENT_LABELS[segment]).not.toBe(segment);

    render(
      <MemoryRouter initialEntries={[`/admin/${segment}`]}>
        <Breadcrumb />
      </MemoryRouter>,
    );

    expect(screen.getByText(labelKey)).toBeInTheDocument();
    expect(screen.queryByText(segment)).not.toBeInTheDocument();
  });

  it('marks the last crumb with aria-current=page', () => {
    render(
      <MemoryRouter initialEntries={['/admin/lots']}>
        <Breadcrumb />
      </MemoryRouter>,
    );
    const lastCrumb = screen.getByText('admin.lots');
    expect(lastCrumb).toHaveAttribute('aria-current', 'page');
  });

  it('uses raw segment when no SEGMENT_LABELS mapping exists', () => {
    render(
      <MemoryRouter initialEntries={['/foobar']}>
        <Breadcrumb />
      </MemoryRouter>,
    );
    expect(screen.getByText('foobar')).toBeInTheDocument();
  });

  it('renders carets between crumbs', () => {
    render(
      <MemoryRouter initialEntries={['/admin/lots']}>
        <Breadcrumb />
      </MemoryRouter>,
    );
    expect(screen.getAllByTestId('caret-right').length).toBeGreaterThanOrEqual(2);
  });
});
