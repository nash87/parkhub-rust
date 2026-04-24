/**
 * Tier-2 items 9 + 10 — shared export helpers.
 *
 *  * buildCsv(): RFC-4180 CSV serializer used by DataTable + admin screens.
 *  * buildIcsEvent(): RFC-5545 VEVENT / VCALENDAR serializer used by the
 *    "Zum Kalender hinzufügen" action on bookings. Server-side bulk feeds
 *    still come from the backend endpoints — this helper covers the
 *    client-side single-event download path.
 *  * downloadPdfTable(): lazy-loaded pdf-lib PDF export for admin tables.
 *
 * PDF backend swapped from jspdf@3.0.4 → pdf-lib@^1.17 on 2026-04-25.
 * jspdf 3.x has 8 outstanding GHSA advisories (path traversal, HTML
 * injection in new-window paths, BMP/GIF DoS, AcroForm + addJS PDF object
 * injection, FreeText color injection) with no patched release. pdf-lib
 * is MIT-licensed, pure TypeScript, zero outstanding advisories, and
 * ships smaller when lazy-imported. Contract (arguments + async void
 * return) is unchanged so DataTable + tests need no changes.
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
 * Split a line into chunks that fit the page width using the supplied font.
 * pdf-lib does not ship a line-breaker, so we measure greedily — the API is
 * small enough to keep inline rather than introduce another dep.
 */
function wrapLine(text: string, font: import('pdf-lib').PDFFont, size: number, maxWidth: number): string[] {
  if (font.widthOfTextAtSize(text, size) <= maxWidth) return [text];
  const out: string[] = [];
  const words = text.split(/(\s+)/);
  let current = '';
  for (const token of words) {
    const candidate = current + token;
    if (font.widthOfTextAtSize(candidate, size) > maxWidth && current.length > 0) {
      out.push(current.trimEnd());
      current = token.trimStart();
    } else {
      current = candidate;
    }
  }
  if (current.length > 0) out.push(current.trimEnd());
  return out;
}

/**
 * Lazy PDF export — pdf-lib is dynamic-imported on first click to keep
 * it out of the main bundle. A4 portrait, Helvetica, pipe-separated rows
 * matching the previous jspdf layout so admins see no visual regression.
 */
export async function downloadPdfTable(
  filename: string,
  title: string,
  headers: readonly string[],
  rows: readonly (readonly CsvCell[])[],
): Promise<void> {
  const { PDFDocument, StandardFonts } = await import('pdf-lib');
  const pdf = await PDFDocument.create();
  const font = await pdf.embedFont(StandardFonts.Helvetica);

  // A4 portrait at 72 dpi: 595 × 842 pt. Match the jspdf coordinate
  // system (top-origin) by tracking `y` in top-down pt values.
  const pageWidth = 595;
  const pageHeight = 842;
  const marginX = 40;
  const maxWidth = pageWidth - marginX * 2;
  const bodySize = 9;
  const bodyLineHeight = 12;

  let page = pdf.addPage([pageWidth, pageHeight]);
  page.drawText(title, { x: marginX, y: pageHeight - 46, size: 14, font });
  page.drawText(headers.join(' | '), { x: marginX, y: pageHeight - 74, size: bodySize, font });
  let yFromTop = 88;

  const drawLine = (text: string): void => {
    if (yFromTop > pageHeight - 30) {
      page = pdf.addPage([pageWidth, pageHeight]);
      yFromTop = 46;
    }
    page.drawText(text, { x: marginX, y: pageHeight - yFromTop, size: bodySize, font });
    yFromTop += bodyLineHeight;
  };

  for (const row of rows) {
    const cells = row.map(c => (c === null || c === undefined ? '' : String(c)));
    const wrapped = wrapLine(cells.join(' | '), font, bodySize, maxWidth);
    for (const line of wrapped) drawLine(line);
  }

  const bytes = await pdf.save();
  const blob = new Blob([bytes as BlobPart], { type: 'application/pdf' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename.endsWith('.pdf') ? filename : `${filename}.pdf`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
