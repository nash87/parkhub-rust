import { describe, it, expect } from 'vitest';
import React from 'react';
import { render } from '@testing-library/react';
import {
  SkeletonText,
  SkeletonAvatar,
  SkeletonCard,
  SkeletonTable,
  DashboardSkeleton,
  BookingsSkeleton,
  VehiclesSkeleton,
} from './Skeleton';

describe('SkeletonText', () => {
  it('renders with default w-full class', () => {
    const { container } = render(<SkeletonText />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('w-full');
    expect(el.className).toContain('skeleton');
    expect(el.className).toContain('h-4');
  });

  it('renders with custom width class', () => {
    const { container } = render(<SkeletonText width="w-48" />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('w-48');
    expect(el.className).not.toContain('w-full');
  });

  it('accepts extra className', () => {
    const { container } = render(<SkeletonText className="h-8" />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('h-8');
  });
});

describe('SkeletonAvatar', () => {
  it('renders with default size', () => {
    const { container } = render(<SkeletonAvatar />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('w-10');
    expect(el.className).toContain('h-10');
    expect(el.className).toContain('rounded-full');
    expect(el.className).toContain('skeleton');
  });

  it('renders with custom size', () => {
    const { container } = render(<SkeletonAvatar size="w-16 h-16" />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('w-16');
    expect(el.className).toContain('h-16');
  });
});

describe('SkeletonCard', () => {
  it('renders with default height', () => {
    const { container } = render(<SkeletonCard />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('h-28');
    expect(el.className).toContain('skeleton');
    expect(el.className).toContain('rounded-2xl');
  });

  it('renders with custom height', () => {
    const { container } = render(<SkeletonCard height="h-40" />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('h-40');
  });

  it('accepts extra className', () => {
    const { container } = render(<SkeletonCard className="w-full" />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain('w-full');
  });
});

describe('SkeletonTable', () => {
  it('renders default 3 data rows plus header', () => {
    const { container } = render(<SkeletonTable />);
    // Container has space-y-3: 1 header flex + 3 data row flex divs
    const wrapper = container.firstChild as HTMLElement;
    const rows = wrapper.children;
    // 1 header + 3 data rows = 4
    expect(rows.length).toBe(4);
  });

  it('renders custom number of rows', () => {
    const { container } = render(<SkeletonTable rows={5} />);
    const wrapper = container.firstChild as HTMLElement;
    // 1 header + 5 data rows
    expect(wrapper.children.length).toBe(6);
  });

  it('renders 0 rows when rows=0', () => {
    const { container } = render(<SkeletonTable rows={0} />);
    const wrapper = container.firstChild as HTMLElement;
    // 1 header + 0 data rows
    expect(wrapper.children.length).toBe(1);
  });

  it('header row has 4 skeleton columns', () => {
    const { container } = render(<SkeletonTable />);
    const wrapper = container.firstChild as HTMLElement;
    const headerRow = wrapper.children[0] as HTMLElement;
    expect(headerRow.children.length).toBe(4);
  });
});

describe('DashboardSkeleton', () => {
  it('renders without crashing', () => {
    const { container } = render(<DashboardSkeleton />);
    expect(container.firstChild).toBeTruthy();
  });

  it('renders 4 stat cards in the grid', () => {
    const { container } = render(<DashboardSkeleton />);
    const grid = container.querySelector('.grid.grid-cols-2') as HTMLElement;
    expect(grid).toBeTruthy();
    expect(grid.children.length).toBe(4);
  });

  it('renders a greeting skeleton', () => {
    const { container } = render(<DashboardSkeleton />);
    // First child of the wrapper is the greeting SkeletonText with w-72
    const greeting = container.querySelector('.w-72.h-8') as HTMLElement;
    expect(greeting).toBeTruthy();
    expect(greeting.className).toContain('skeleton');
  });

  it('renders quick actions section with 4 action items', () => {
    const { container } = render(<DashboardSkeleton />);
    // Quick actions has 4 items each with gap-3 p-3
    const actionItems = container.querySelectorAll('.flex.items-center.gap-3.p-3');
    expect(actionItems.length).toBe(4);
  });
});

describe('BookingsSkeleton', () => {
  it('renders without crashing', () => {
    const { container } = render(<BookingsSkeleton />);
    expect(container.firstChild).toBeTruthy();
  });

  it('renders 3 booking sections', () => {
    const { container } = render(<BookingsSkeleton />);
    // Each section has a grid with 2 booking cards (h-40)
    const bookingCards = container.querySelectorAll('.h-40');
    // 3 sections x 2 cards each = 6
    expect(bookingCards.length).toBe(6);
  });

  it('renders a header with title and button skeleton', () => {
    const { container } = render(<BookingsSkeleton />);
    const header = container.querySelector('.flex.items-center.justify-between') as HTMLElement;
    expect(header).toBeTruthy();
  });

  it('renders a filter bar', () => {
    const { container } = render(<BookingsSkeleton />);
    // Filter bar has 2 skeleton inputs in a grid
    const filterGrid = container.querySelector('.grid.grid-cols-1.sm\\:grid-cols-2') as HTMLElement;
    expect(filterGrid).toBeTruthy();
    expect(filterGrid.children.length).toBe(2);
  });
});

describe('VehiclesSkeleton', () => {
  it('renders without crashing', () => {
    const { container } = render(<VehiclesSkeleton />);
    expect(container.firstChild).toBeTruthy();
  });

  it('renders 2 vehicle cards', () => {
    const { container } = render(<VehiclesSkeleton />);
    const vehicleGrid = container.querySelector('.grid.grid-cols-1.md\\:grid-cols-2') as HTMLElement;
    expect(vehicleGrid).toBeTruthy();
    expect(vehicleGrid.children.length).toBe(2);
  });

  it('each vehicle card has a 14x14 avatar skeleton', () => {
    const { container } = render(<VehiclesSkeleton />);
    const avatars = container.querySelectorAll('.w-14.h-14.skeleton');
    expect(avatars.length).toBe(2);
  });
});
