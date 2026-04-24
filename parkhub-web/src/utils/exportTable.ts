/**
 * Tier-2 items 9 + 10 — shared export helpers.
 *
 *  * buildCsv(): RFC-4180 CSV serializer used by DataTable + admin screens.
 *  * buildIcsEvent(): RFC-5545 VEVENT / VCALENDAR serializer used by the
 *    "Zum Kalender hinzufügen" action on bookings. Server-side bulk feeds
 *    still come from the backend endpoints — this helper covers the
 *    client-side single-event download path.
 *  * downloadPdfTable(): lazy-loaded jspdf PDF export for admin tables.
 */

export type CsvCell = string | number | boolean | null | undefined;

function escapeCsvCell(value: CsvCell): string {
  if (value === null || value === undefined) return '';
  const str = String(value);
  if (/[",\n\r]/.test(str)) return `"${str.replace(/"/g, '""')}"`;
  return str;
}

export function buildCsv(headers: readonly string[], rows: readonly (readonly CsvCell[])[]): string {
  const head = headers.map(escapeCsvCell).join(',');
  const body = rows.map(row => row.map(escapeCsvCell).join(','));
  return [head, ...body].join('\n');
}

export function downloadCsv(filename: string, csv: string): void {
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename.endsWith('.csv') ? filename : `${filename}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function icsDate(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return (
    `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}` +
    `T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}Z`
  );
}

export interface IcsEventInput {
  uid: string;
  summary: string;
  location: string;
  start: Date;
  end: Date;
  description?: string;
}

export function buildIcsEvent(event: IcsEventInput, opts: { standalone?: boolean } = {}): string {
  const lines: string[] = [];
  if (opts.standalone) {
    lines.push('BEGIN:VCALENDAR', 'VERSION:2.0', 'PRODID:-//ParkHub//Bookings//EN', 'CALSCALE:GREGORIAN', 'METHOD:PUBLISH');
  }
  lines.push('BEGIN:VEVENT');
  lines.push(`UID:${event.uid}`);
  lines.push(`DTSTAMP:${icsDate(new Date())}`);
  lines.push(`DTSTART:${icsDate(event.start)}`);
  lines.push(`DTEND:${icsDate(event.end)}`);
  lines.push(`SUMMARY:${event.summary}`);
  lines.push(`LOCATION:${event.location}`);
  if (event.description) lines.push(`DESCRIPTION:${event.description}`);
  lines.push('STATUS:CONFIRMED');
  lines.push('END:VEVENT');
  if (opts.standalone) lines.push('END:VCALENDAR');
  return lines.join('\r\n') + '\r\n';
}

export function downloadIcs(filename: string, ics: string): void {
  const blob = new Blob([ics], { type: 'text/calendar;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename.endsWith('.ics') ? filename : `${filename}.ics`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/**
 * Lazy PDF export — jspdf is ~200 KB, so we dynamic-import on the first
 * click to keep it out of the main bundle.
 */
export async function downloadPdfTable(
  filename: string,
  title: string,
  headers: readonly string[],
  rows: readonly (readonly CsvCell[])[],
): Promise<void> {
  const { jsPDF } = await import('jspdf');
  const doc = new jsPDF();
  doc.setFontSize(14);
  doc.text(title, 14, 16);
  doc.setFontSize(9);
  let y = 26;
  doc.text(headers.join(' | '), 14, y);
  y += 6;
  for (const row of rows) {
    const line = row.map(c => (c === null || c === undefined ? '' : String(c))).join(' | ');
    const split = doc.splitTextToSize(line, 180);
    doc.text(split, 14, y);
    y += 5 * split.length;
    if (y > 280) { doc.addPage(); y = 16; }
  }
  doc.save(filename.endsWith('.pdf') ? filename : `${filename}.pdf`);
}
