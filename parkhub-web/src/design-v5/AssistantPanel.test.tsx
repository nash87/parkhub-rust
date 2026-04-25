import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

vi.mock('./Toast', () => ({
  useV5Toast: () => vi.fn(),
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { V5AssistantPanel } from './AssistantPanel';

describe('V5AssistantPanel — branding', () => {
  it('renders Assistent header without AI/KI branding', () => {
    render(<V5AssistantPanel open />);
    expect(screen.getByRole('complementary', { name: 'Assistent' })).toBeInTheDocument();
    expect(screen.getByText('Assistent')).toBeInTheDocument();
    expect(screen.queryByText(/KI-Assistent/)).toBeNull();
    expect(screen.queryByText(/^KI$/)).toBeNull();
  });

  it('returns null when closed', () => {
    const { container } = render(<V5AssistantPanel open={false} />);
    expect(container.firstChild).toBeNull();
  });
});
