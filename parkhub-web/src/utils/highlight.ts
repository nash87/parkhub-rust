export interface HighlightPart {
  text: string;
  match: boolean;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Case-insensitive tokenizer used by the admin in-screen search
 * (Tier-2 item 13). Splits `text` into alternating match /
 * non-match segments so the view can wrap matches in `<mark>`.
 */
export function splitHighlight(text: string, query: string): HighlightPart[] {
  if (!query) return [{ text, match: false }];
  const re = new RegExp(`(${escapeRegex(query)})`, 'ig');
  const out: HighlightPart[] = [];
  let lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m.index > lastIndex) out.push({ text: text.slice(lastIndex, m.index), match: false });
    out.push({ text: m[0], match: true });
    lastIndex = m.index + m[0].length;
    if (m[0].length === 0) re.lastIndex++;
  }
  if (lastIndex < text.length) out.push({ text: text.slice(lastIndex), match: false });
  return out;
}
