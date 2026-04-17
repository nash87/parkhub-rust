import { describe, it, expect, vi } from 'vitest';
import { createRegistry, type Command, type CommandContext } from './commandRegistry';

const ctx: CommandContext = {
  user: { id: '1', role: 'user' },
  isAdmin: false,
  navigate: () => undefined,
};

function cmd(overrides: Partial<Command> & Pick<Command, 'id' | 'title'>): Command {
  return {
    group: 'navigation',
    perform: () => undefined,
    ...overrides,
  };
}

describe('commandRegistry', () => {
  it('register returns an unregister function that removes the command', () => {
    const reg = createRegistry();
    const unregister = reg.register(cmd({ id: 'go-home', title: 'Home' }));
    expect(reg.all()).toHaveLength(1);
    unregister();
    expect(reg.all()).toHaveLength(0);
  });

  it('re-registering the same id replaces the previous command (dedupe)', () => {
    const reg = createRegistry();
    reg.register(cmd({ id: 'x', title: 'First' }));
    reg.register(cmd({ id: 'x', title: 'Second' }));
    const all = reg.all();
    expect(all).toHaveLength(1);
    expect(all[0]?.title).toBe('Second');
  });

  it('registerMany registers a batch and unregisters all at once', () => {
    const reg = createRegistry();
    const unregister = reg.registerMany([
      cmd({ id: 'a', title: 'Alpha' }),
      cmd({ id: 'b', title: 'Bravo' }),
    ]);
    expect(reg.all()).toHaveLength(2);
    unregister();
    expect(reg.all()).toHaveLength(0);
  });

  it('search filters by query (prefix > substring > keyword > char-run)', () => {
    const reg = createRegistry();
    reg.register(cmd({ id: 'book', title: 'Book a Spot' }));
    reg.register(cmd({ id: 'bookings', title: 'Bookings' }));
    reg.register(cmd({ id: 'profile', title: 'Profile', keywords: ['me', 'account'] }));

    const bookHits = reg.search('book', ctx).map((c) => c.id);
    expect(bookHits).toEqual(expect.arrayContaining(['book', 'bookings']));
    expect(bookHits).not.toContain('profile');

    // Keyword boost — "account" is a profile keyword.
    const accountHits = reg.search('account', ctx).map((c) => c.id);
    expect(accountHits).toContain('profile');
  });

  it('search honours the when() predicate (visibility gating)', () => {
    const reg = createRegistry();
    reg.register(cmd({ id: 'adm', title: 'Admin Panel', when: (c) => c.isAdmin }));
    reg.register(cmd({ id: 'usr', title: 'User Dashboard' }));

    expect(reg.search('', ctx).map((c) => c.id)).toEqual(['usr']);
    expect(reg.search('', { ...ctx, isAdmin: true }).map((c) => c.id).sort()).toEqual(['adm', 'usr']);
  });

  it('empty query returns every visible command', () => {
    const reg = createRegistry();
    reg.registerMany([
      cmd({ id: 'a', title: 'Alpha' }),
      cmd({ id: 'b', title: 'Bravo' }),
      cmd({ id: 'c', title: 'Charlie' }),
    ]);
    expect(reg.search('', ctx)).toHaveLength(3);
  });

  it('subscribe notifies listeners on mutation', () => {
    const reg = createRegistry();
    const listener = vi.fn();
    const unsubscribe = reg.subscribe(listener);
    reg.register(cmd({ id: 'x', title: 'X' }));
    expect(listener).toHaveBeenCalledTimes(1);
    reg.clear();
    expect(listener).toHaveBeenCalledTimes(2);
    unsubscribe();
    reg.register(cmd({ id: 'y', title: 'Y' }));
    expect(listener).toHaveBeenCalledTimes(2);
  });

  it('unregister after a different command took the id does not remove the new one', () => {
    const reg = createRegistry();
    const undoFirst = reg.register(cmd({ id: 'dup', title: 'First' }));
    reg.register(cmd({ id: 'dup', title: 'Second' }));
    undoFirst();
    expect(reg.all()).toHaveLength(1);
    expect(reg.all()[0]?.title).toBe('Second');
  });
});
