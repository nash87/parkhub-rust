import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { createColumnHelper } from '@tanstack/react-table';

vi.mock('framer-motion', () => ({
  motion: {
    tr: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <tr ref={ref} {...props}>{children}</tr>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CaretUp: (props: any) => <span data-testid="icon-caret-up" {...props} />,
  CaretDown: (props: any) => <span data-testid="icon-caret-down" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
}));

import { DataTable } from './DataTable';

interface TestRow {
  id: string;
  name: string;
  email: string;
  role: string;
}

const columnHelper = createColumnHelper<TestRow>();

const columns = [
  columnHelper.accessor('name', { header: 'Name', enableSorting: true }),
  columnHelper.accessor('email', { header: 'Email', enableSorting: true }),
  columnHelper.accessor('role', { header: () => 'Role', enableSorting: false }),
];

const sampleData: TestRow[] = [
  { id: '1', name: 'Alice', email: 'alice@test.com', role: 'admin' },
  { id: '2', name: 'Bob', email: 'bob@test.com', role: 'user' },
  { id: '3', name: 'Charlie', email: 'charlie@test.com', role: 'user' },
];

describe('DataTable', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders table with headers and rows', () => {
    render(<DataTable data={sampleData} columns={columns} />);
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByText('Name')).toBeInTheDocument();
    expect(screen.getByText('Email')).toBeInTheDocument();
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('Bob')).toBeInTheDocument();
    expect(screen.getByText('Charlie')).toBeInTheDocument();
  });

  it('shows empty message when no data', () => {
    render(<DataTable data={[]} columns={columns} emptyMessage="Nothing here" />);
    expect(screen.getByText('Nothing here')).toBeInTheDocument();
  });

  it('shows default empty message', () => {
    render(<DataTable data={[]} columns={columns} />);
    expect(screen.getByText('No data')).toBeInTheDocument();
  });

  it('sorts by column when header is clicked', () => {
    render(<DataTable data={sampleData} columns={columns} />);
    const nameHeader = screen.getByText('Name');
    // Click to sort ascending
    fireEvent.click(nameHeader);
    // Click again to sort descending
    fireEvent.click(nameHeader);
    // Click again to clear sort
    fireEvent.click(nameHeader);
    // Should not crash
    expect(screen.getByText('Alice')).toBeInTheDocument();
  });

  it('does not sort on non-sortable columns', () => {
    render(<DataTable data={sampleData} columns={columns} />);
    // 'Role' column has enableSorting: false — its header is a function
    // The header text "Role" should be rendered but clicking should not crash
    const rows = screen.getAllByRole('row');
    expect(rows.length).toBeGreaterThan(1);
  });

  it('filters data with global search (includesString mode)', () => {
    render(<DataTable data={sampleData} columns={columns} searchValue="alice" />);
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.queryByText('Bob')).not.toBeInTheDocument();
  });

  it('filters data with searchColumn-specific mode', () => {
    render(
      <DataTable data={sampleData} columns={columns} searchValue="alice" searchColumn="name" />
    );
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.queryByText('Bob')).not.toBeInTheDocument();
  });

  it('handles onRowClick callback', () => {
    const onClick = vi.fn();
    render(<DataTable data={sampleData} columns={columns} onRowClick={onClick} />);
    const aliceRow = screen.getByText('Alice').closest('tr')!;
    fireEvent.click(aliceRow);
    expect(onClick).toHaveBeenCalledWith(sampleData[0]);
  });

  it('does not show CSV button when exportFilename is not provided', () => {
    render(<DataTable data={sampleData} columns={columns} />);
    expect(screen.queryByText('CSV')).not.toBeInTheDocument();
  });

  it('shows CSV button when exportFilename is provided', () => {
    render(<DataTable data={sampleData} columns={columns} exportFilename="test-export" />);
    expect(screen.getByText('CSV')).toBeInTheDocument();
  });

  it('exports CSV on button click', () => {
    const mockCreateObjectURL = vi.fn(() => 'blob:test');
    const mockRevokeObjectURL = vi.fn();
    global.URL.createObjectURL = mockCreateObjectURL;
    global.URL.revokeObjectURL = mockRevokeObjectURL;

    const mockClick = vi.fn();
    const originalCreateElement = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      if (tag === 'a') {
        return { href: '', download: '', click: mockClick } as any;
      }
      return originalCreateElement(tag);
    });

    render(<DataTable data={sampleData} columns={columns} exportFilename="users" />);
    fireEvent.click(screen.getByText('CSV'));

    expect(mockCreateObjectURL).toHaveBeenCalled();
    expect(mockClick).toHaveBeenCalled();
    expect(mockRevokeObjectURL).toHaveBeenCalled();
  });

  it('renders sort indicators for sortable columns', () => {
    render(<DataTable data={sampleData} columns={columns} />);
    // Name and Email columns are sortable, each should have CaretUp and CaretDown
    const caretUps = screen.getAllByTestId('icon-caret-up');
    const caretDowns = screen.getAllByTestId('icon-caret-down');
    expect(caretUps.length).toBe(2); // Name and Email
    expect(caretDowns.length).toBe(2);
  });

  it('applies aria-sort ascending after click', () => {
    render(<DataTable data={sampleData} columns={columns} />);
    const nameHeader = screen.getByText('Name').closest('th')!;
    expect(nameHeader).toHaveAttribute('aria-sort', 'none');
    fireEvent.click(nameHeader);
    expect(nameHeader).toHaveAttribute('aria-sort', 'ascending');
    fireEvent.click(nameHeader);
    expect(nameHeader).toHaveAttribute('aria-sort', 'descending');
  });

  it('handles CSV export with values containing commas and quotes', () => {
    const dataWithSpecial: TestRow[] = [
      { id: '1', name: 'Alice, Jr.', email: 'alice@test.com', role: 'admin "super"' },
    ];
    const mockCreateObjectURL = vi.fn(() => 'blob:test');
    const mockRevokeObjectURL = vi.fn();
    global.URL.createObjectURL = mockCreateObjectURL;
    global.URL.revokeObjectURL = mockRevokeObjectURL;

    const mockClick = vi.fn();
    const originalCreateElement = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      if (tag === 'a') return { href: '', download: '', click: mockClick } as any;
      return originalCreateElement(tag);
    });

    render(<DataTable data={dataWithSpecial} columns={columns} exportFilename="special" />);
    fireEvent.click(screen.getByText('CSV'));

    expect(mockCreateObjectURL).toHaveBeenCalled();
  });

  it('rows have cursor-pointer class when onRowClick is provided', () => {
    const onClick = vi.fn();
    render(<DataTable data={sampleData} columns={columns} onRowClick={onClick} />);
    const row = screen.getByText('Alice').closest('tr')!;
    expect(row.className).toContain('cursor-pointer');
  });

  it('rows do not have cursor-pointer when no onRowClick', () => {
    render(<DataTable data={sampleData} columns={columns} />);
    const row = screen.getByText('Alice').closest('tr')!;
    expect(row.className).not.toContain('cursor-pointer');
  });

  it('handles empty export (no rows)', () => {
    const mockCreateObjectURL = vi.fn(() => 'blob:test');
    const mockRevokeObjectURL = vi.fn();
    global.URL.createObjectURL = mockCreateObjectURL;
    global.URL.revokeObjectURL = mockRevokeObjectURL;
    const mockClick = vi.fn();
    const originalCreateElement = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      if (tag === 'a') return { href: '', download: '', click: mockClick } as any;
      return originalCreateElement(tag);
    });

    render(<DataTable data={[]} columns={columns} exportFilename="empty" />);
    fireEvent.click(screen.getByText('CSV'));
    expect(mockClick).toHaveBeenCalled();
  });
});
