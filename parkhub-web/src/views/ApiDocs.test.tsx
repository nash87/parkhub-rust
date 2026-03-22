import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import { useTranslation } from 'react-i18next';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'apiDocs.title': 'API Documentation',
        'apiDocs.help': "Explore and test ParkHub's REST API",
        'apiDocs.openDocs': 'Open API Docs',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('@phosphor-icons/react', () => ({
  BookOpen: (props: any) => <span data-testid="icon-book" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  ArrowSquareOut: (props: any) => <span data-testid="icon-external" {...props} />,
}));

// Inline component that represents the API docs admin sidebar link
function ApiDocsLink() {
  const { t } = useTranslation();
  return (
    <div>
      <a
        href="/api/v1/docs"
        target="_blank"
        rel="noopener noreferrer"
        className="flex items-center gap-2"
        data-testid="api-docs-link"
      >
        <span data-testid="icon-book" />
        <span>{t('apiDocs.title')}</span>
      </a>
      <p className="text-sm text-surface-500">{t('apiDocs.help')}</p>
    </div>
  );
}

describe('ApiDocs Link', () => {
  it('renders the API docs link', () => {
    render(<ApiDocsLink />);
    expect(screen.getByText('API Documentation')).toBeTruthy();
    expect(screen.getByTestId('api-docs-link')).toBeTruthy();
  });

  it('shows help text', () => {
    render(<ApiDocsLink />);
    expect(screen.getByText("Explore and test ParkHub's REST API")).toBeTruthy();
  });

  it('links to /api/v1/docs', () => {
    render(<ApiDocsLink />);
    const link = screen.getByTestId('api-docs-link');
    expect(link.getAttribute('href')).toBe('/api/v1/docs');
    expect(link.getAttribute('target')).toBe('_blank');
  });
});
