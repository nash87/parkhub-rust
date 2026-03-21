import { useState, useMemo } from 'react';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  flexRender,
  type ColumnDef,
  type SortingState,
} from '@tanstack/react-table';
import { CaretUp, CaretDown, DownloadSimple } from '@phosphor-icons/react';
import { motion, AnimatePresence } from 'framer-motion';

interface DataTableProps<T> {
  data: T[];
  columns: ColumnDef<T, unknown>[];
  searchValue?: string;
  searchColumn?: string;
  emptyMessage?: string;
  /** Optional row click handler */
  onRowClick?: (row: T) => void;
  /** When provided, show a CSV download button with this filename (without extension) */
  exportFilename?: string;
}

function escapeCsvCell(value: unknown): string {
  const str = String(value ?? '');
  if (str.includes(',') || str.includes('"') || str.includes('\n')) {
    return `"${str.replace(/"/g, '""')}"`;
  }
  return str;
}

/** Reusable data table with sorting, search filtering, and staggered row animation. */
export function DataTable<T>({
  data,
  columns,
  searchValue = '',
  searchColumn,
  emptyMessage = 'No data',
  onRowClick,
  exportFilename,
}: DataTableProps<T>) {
  const [sorting, setSorting] = useState<SortingState>([]);

  const globalFilter = searchValue;

  const table = useReactTable({
    data,
    columns: columns as ColumnDef<T, unknown>[],
    state: { sorting, globalFilter },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    globalFilterFn: searchColumn
      ? (row, _columnId, filterValue) => {
          const val = String(row.getValue(searchColumn) ?? '').toLowerCase();
          return val.includes(String(filterValue).toLowerCase());
        }
      : 'includesString',
  });

  const rows = table.getRowModel().rows;

  function handleExportCsv() {
    if (!exportFilename) return;
    const headerGroups = table.getHeaderGroups();
    const visibleColumns = headerGroups[0]?.headers.filter(h => h.column.columnDef.header) ?? [];
    const headerRow = visibleColumns
      .map(h => {
        const rendered = h.column.columnDef.header;
        if (typeof rendered === 'function') {
          // Use the column id as header label for CSV
          return escapeCsvCell(h.column.id);
        }
        return escapeCsvCell(rendered ?? h.column.id);
      })
      .join(',');

    const dataRows = rows.map(row =>
      visibleColumns
        .map(h => {
          const cell = row.getAllCells().find(c => c.column.id === h.column.id);
          return escapeCsvCell(cell?.getValue() ?? '');
        })
        .join(',')
    );

    const csv = [headerRow, ...dataRows].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${exportFilename}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <div className="card overflow-hidden">
      {exportFilename && (
        <div className="flex justify-end px-5 pt-4">
          <button
            onClick={handleExportCsv}
            className="btn btn-sm btn-secondary"
            aria-label={`Export ${exportFilename} as CSV`}
          >
            <DownloadSimple weight="bold" className="w-4 h-4" />
            CSV
          </button>
        </div>
      )}
      <div className="overflow-x-auto">
        <table className="w-full" role="grid">
          <thead>
            {table.getHeaderGroups().map(hg => (
              <tr key={hg.id} className="border-b border-surface-200 dark:border-surface-700">
                {hg.headers.map(header => {
                  const canSort = header.column.getCanSort();
                  const sorted = header.column.getIsSorted();
                  return (
                    <th
                      key={header.id}
                      className={`text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider ${
                        canSort ? 'cursor-pointer select-none hover:text-surface-700 dark:hover:text-surface-200 transition-colors' : ''
                      }`}
                      onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                      aria-sort={sorted === 'asc' ? 'ascending' : sorted === 'desc' ? 'descending' : 'none'}
                    >
                      <span className="inline-flex items-center gap-1">
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        {canSort && (
                          <span className="inline-flex flex-col -space-y-1 ml-0.5" aria-hidden="true">
                            <CaretUp weight="bold" className={`w-3 h-3 ${sorted === 'asc' ? 'text-primary-600' : 'text-surface-300 dark:text-surface-600'}`} />
                            <CaretDown weight="bold" className={`w-3 h-3 ${sorted === 'desc' ? 'text-primary-600' : 'text-surface-300 dark:text-surface-600'}`} />
                          </span>
                        )}
                      </span>
                    </th>
                  );
                })}
              </tr>
            ))}
          </thead>
          <tbody className="divide-y divide-surface-100 dark:divide-surface-800">
            <AnimatePresence mode="popLayout">
              {rows.map((row, i) => (
                <motion.tr
                  key={row.id}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ delay: Math.min(i * 0.02, 0.3) }}
                  className={`hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors ${
                    onRowClick ? 'cursor-pointer' : ''
                  }`}
                  onClick={onRowClick ? () => onRowClick(row.original) : undefined}
                >
                  {row.getVisibleCells().map(cell => (
                    <td key={cell.id} className="px-5 py-4">
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  ))}
                </motion.tr>
              ))}
            </AnimatePresence>
          </tbody>
        </table>
      </div>

      {rows.length === 0 && (
        <div className="p-8 text-center">
          <p className="text-sm text-surface-500 dark:text-surface-400">{emptyMessage}</p>
        </div>
      )}
    </div>
  );
}
