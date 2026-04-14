import { describe, it, expect, vi } from 'vitest';

// Mock dependencies before import
vi.mock('./PageTransition', () => ({ PageTransition: 'PageTransition' }));
vi.mock('./Breadcrumb', () => ({ Breadcrumb: 'Breadcrumb' }));
vi.mock('./DataTable', () => ({ DataTable: 'DataTable' }));
vi.mock('./FormField', () => ({ FormField: 'FormField', FormInput: 'FormInput' }));
vi.mock('./ConfirmDialog', () => ({ ConfirmDialog: 'ConfirmDialog' }));
vi.mock('./NotificationBadge', () => ({ NotificationBadge: 'NotificationBadge' }));

import * as uiExports from './index';

describe('components/ui barrel export', () => {
  it('exports PageTransition', () => {
    expect(uiExports.PageTransition).toBeDefined();
  });

  it('exports Breadcrumb', () => {
    expect(uiExports.Breadcrumb).toBeDefined();
  });

  it('exports DataTable', () => {
    expect(uiExports.DataTable).toBeDefined();
  });

  it('exports FormField', () => {
    expect(uiExports.FormField).toBeDefined();
  });

  it('exports FormInput', () => {
    expect(uiExports.FormInput).toBeDefined();
  });

  it('exports ConfirmDialog', () => {
    expect(uiExports.ConfirmDialog).toBeDefined();
  });

  it('exports NotificationBadge', () => {
    expect(uiExports.NotificationBadge).toBeDefined();
  });

  it('exports exactly the expected number of items', () => {
    const exportNames = Object.keys(uiExports);
    expect(exportNames).toEqual([
      'PageTransition',
      'Breadcrumb',
      'DataTable',
      'FormField',
      'FormInput',
      'ConfirmDialog',
      'NotificationBadge',
    ]);
  });
});
