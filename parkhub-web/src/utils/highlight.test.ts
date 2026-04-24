import { describe, it, expect } from 'vitest';
import { splitHighlight } from './highlight';

describe('splitHighlight', () => {
  it('returns a single non-matching segment when query is empty', () => {
    expect(splitHighlight('Admin Panel', '')).toEqual([{ text: 'Admin Panel', match: false }]);
  });

  it('is case-insensitive', () => {
    const parts = splitHighlight('Admin Panel', 'ADMIN');
    expect(parts).toEqual([
      { text: 'Admin', match: true },
      { text: ' Panel', match: false },
    ]);
  });

  it('splits every occurrence into alternating match / non-match segments', () => {
    const parts = splitHighlight('admin admin', 'admin');
    expect(parts).toEqual([
      { text: 'admin', match: true },
      { text: ' ', match: false },
      { text: 'admin', match: true },
    ]);
  });

  it('treats special regex characters in the query as literals', () => {
    const parts = splitHighlight('foo.bar', '.');
    expect(parts).toEqual([
      { text: 'foo', match: false },
      { text: '.', match: true },
      { text: 'bar', match: false },
    ]);
  });
});
