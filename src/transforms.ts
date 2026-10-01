export const transforms = {
  upper: 'UPPERCASE',
  lower: 'lowercase',
  title: 'Title Case',
  trim: 'Usuń białe znaki na końcach',
  spaces: 'Usuń wielokrotne spacje',
  plain: 'Usuń formatowanie',
  pretty: 'JSON · formatuj',
  minify: 'JSON · minimalizuj',
  urlEncode: 'URL · koduj',
  urlDecode: 'URL · dekoduj',
  base64Encode: 'Base64 · koduj',
  base64Decode: 'Base64 · dekoduj',
  sort: 'Sortuj linie',
  unique: 'Usuń powtórzone linie',
} as const;
export type Transform = keyof typeof transforms;
// Validate syntax, then format tokens without converting numeric values to IEEE-754.
// This preserves large IDs, exponents and significant number representations.
function jsonFormat(text: string, pretty: boolean): string {
  JSON.parse(text);
  let compact = '',
    quoted = false,
    escaped = false;
  for (const ch of text) {
    if (quoted) {
      compact += ch;
      if (escaped) escaped = false;
      else if (ch === '\\') escaped = true;
      else if (ch === '"') quoted = false;
    } else if (ch === '"') {
      quoted = true;
      compact += ch;
    } else if (!/\s/.test(ch)) compact += ch;
  }
  if (!pretty) return compact;
  let out = '',
    depth = 0;
  quoted = false;
  escaped = false;
  const newline = () => {
    out += '\n' + '  '.repeat(depth);
  };
  for (let i = 0; i < compact.length; i++) {
    const ch = compact[i];
    if (quoted) {
      out += ch;
      if (escaped) escaped = false;
      else if (ch === '\\') escaped = true;
      else if (ch === '"') quoted = false;
      continue;
    }
    if (ch === '"') {
      quoted = true;
      out += ch;
    } else if (ch === '{' || ch === '[') {
      out += ch;
      depth++;
      if (compact[i + 1] !== '}' && compact[i + 1] !== ']') newline();
    } else if (ch === '}' || ch === ']') {
      depth--;
      if (compact[i - 1] !== '{' && compact[i - 1] !== '[') newline();
      out += ch;
    } else if (ch === ',') {
      out += ch;
      newline();
    } else if (ch === ':') out += ': ';
    else out += ch;
  }
  return out;
}
export function transform(text: string, action: Transform): string {
  switch (action) {
    case 'upper':
      return text.toLocaleUpperCase('pl');
    case 'lower':
      return text.toLocaleLowerCase('pl');
    case 'title':
      return text
        .toLocaleLowerCase('pl')
        .replace(
          /(^|[^\p{L}\p{N}])(\p{L})/gu,
          (_, a: string, b: string) => a + b.toLocaleUpperCase('pl'),
        );
    case 'trim':
      return text.trim();
    case 'spaces':
      return text.replace(/[^\S\r\n]+/g, ' ');
    case 'plain':
      return text;
    case 'pretty':
      return jsonFormat(text, true);
    case 'minify':
      return jsonFormat(text, false);
    case 'urlEncode':
      return encodeURIComponent(text);
    case 'urlDecode':
      return decodeURIComponent(text);
    case 'base64Encode': {
      const bytes = new TextEncoder().encode(text);
      let s = '';
      for (let i = 0; i < bytes.length; i += 8192)
        s += String.fromCharCode(...bytes.subarray(i, i + 8192));
      return btoa(s);
    }
    case 'base64Decode': {
      const decoded = atob(text.replace(/\s/g, ''));
      return new TextDecoder('utf-8', { fatal: true }).decode(
        Uint8Array.from(decoded, (c) => c.charCodeAt(0)),
      );
    }
    case 'sort':
      return text
        .split(/\r?\n/)
        .sort((a, b) => a.localeCompare(b, 'pl'))
        .join('\n');
    case 'unique':
      return [...new Set(text.split(/\r?\n/))].join('\n');
  }
}
