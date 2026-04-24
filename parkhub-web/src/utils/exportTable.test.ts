import { describe, it, expect, vi, afterEach } from 'vitest';
import { buildCsv, buildIcsEvent, downloadPdfTable } from './exportTable';

describe('buildCsv', () => {
  it('emits header row + data rows joined by newline', () => {
    const csv = buildCsv(['name', 'count'], [['Alpha', 3], ['Beta', 7]]);
    expect(csv).toBe('name,count\nAlpha,3\nBeta,7');
  });

  it('escapes commas, quotes, and newlines with RFC-4180 double-quotes', () => {
    const csv = buildCsv(['name'], [['Alpha, Inc.'], ['He said "hi"'], ['line1\nline2']]);
    expect(csv).toBe('name\n"Alpha, Inc."\n"He said ""hi"""\n"line1\nline2"');
  });

  it('renders null/undefined as empty cells', () => {
    const csv = buildCsv(['a', 'b'], [[null, undefined]]);
    expect(csv).toBe('a,b\n,');
  });
});

describe('buildIcsEvent', () => {
  it('produces an RFC5545 VEVENT block with SUMMARY / DTSTART / DTEND / LOCATION / UID', () => {
    const ics = buildIcsEvent({
      uid: 'book-42@parkhub',
      summary: 'Parking: Slot 7',
      location: 'Alpha Lot',
      start: new Date('2026-04-24T10:00:00Z'),
      end: new Date('2026-04-24T12:00:00Z'),
    });
    expect(ics).toContain('BEGIN:VEVENT');
    expect(ics).toContain('UID:book-42@parkhub');
    expect(ics).toContain('DTSTART:20260424T100000Z');
    expect(ics).toContain('DTEND:20260424T120000Z');
    expect(ics).toContain('SUMMARY:Parking: Slot 7');
    expect(ics).toContain('LOCATION:Alpha Lot');
    expect(ics).toContain('END:VEVENT');
  });

  it('wraps the event in a VCALENDAR when standalone=true', () => {
    const ics = buildIcsEvent({
      uid: 'book-42@parkhub',
      summary: 'X',
      location: 'Y',
      start: new Date('2026-04-24T10:00:00Z'),
      end: new Date('2026-04-24T11:00:00Z'),
    }, { standalone: true });
    expect(ics.startsWith('BEGIN:VCALENDAR')).toBe(true);
    expect(ics.trimEnd().endsWith('END:VCALENDAR')).toBe(true);
    expect(ics).toContain('VERSION:2.0');
  });
});

describe('downloadPdfTable — pdf-lib backend', () => {
  const originalCreate = URL.createObjectURL;
  const originalRevoke = URL.revokeObjectURL;

  afterEach(() => {
    URL.createObjectURL = originalCreate;
    URL.revokeObjectURL = originalRevoke;
    vi.restoreAllMocks();
  });

  it('produces an application/pdf blob with a %PDF header and downloads it', async () => {
    const captured: Blob[] = [];
    URL.createObjectURL = vi.fn((blob: Blob) => {
      captured.push(blob);
      return 'blob:mock';
    }) as typeof URL.createObjectURL;
    URL.revokeObjectURL = vi.fn() as typeof URL.revokeObjectURL;
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    await downloadPdfTable('rows', 'Parking report', ['name', 'count'], [['Alpha', 3], ['Beta', 7]]);

    expect(clickSpy).toHaveBeenCalledTimes(1);
    expect(captured).toHaveLength(1);
    expect(captured[0].type).toBe('application/pdf');
    // Every valid PDF starts with the %PDF- magic; ensures pdf-lib actually
    // produced a document and we didn't regress to a zero-byte/text blob.
    const head = new Uint8Array(await captured[0].slice(0, 5).arrayBuffer());
    const headText = String.fromCharCode(...head);
    expect(headText).toBe('%PDF-');
  });
});
