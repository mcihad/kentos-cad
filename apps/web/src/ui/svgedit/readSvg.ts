import { sanitizeSvg } from '../../style/file';
import { docFromSvgTree, type ImportOptions, type ImportResult, type XmlNode } from '../../style/svg/importSvg';

/**
 * SVG text into the editor's model: the browser parses the XML (after the
 * same cleaning the library applies), the pure importer does the rest.
 * Files from other programs often break strict XML (undeclared prefixes,
 * HTML entities); those are mended, and as a last resort the HTML parser
 * (which reads inline SVG leniently) is used. Texts keep their runs
 * (`<tspan>`) and style sheets their text.
 */

/** Elements whose text matters (and their descendants'). */
const TEXTUAL = new Set(['text', 'style', 'title']);

function toNode(el: Element, keepText: boolean): XmlNode {
  const attrs: Record<string, string> = {};
  for (const a of Array.from(el.attributes)) attrs[a.name] = a.value;
  const tag = el.localName;
  const inText = keepText || TEXTUAL.has(tag);
  const children: XmlNode[] = [];
  for (const c of Array.from(el.childNodes)) {
    if (c.nodeType === Node.ELEMENT_NODE) children.push(toNode(c as Element, inText));
    else if (inText && (c.nodeType === Node.TEXT_NODE || c.nodeType === Node.CDATA_SECTION_NODE)) children.push({ tag: '#text', attrs: {}, children: [], text: c.nodeValue ?? '' });
  }
  return { tag, attrs, children };
}

export interface XmlError {
  message: string;
  line: number;
  column: number;
}

/** The first XML error of a text (line and column from the browser's report), or null when it is well formed. */
export function xmlError(text: string): XmlError | null {
  const xml = new DOMParser().parseFromString(text, 'image/svg+xml');
  const err = xml.getElementsByTagName('parsererror')[0];
  if (!err) return null;
  const report = err.textContent ?? '';
  // Chrome: "error on line 3 at column 7: …"; Firefox: "… Line Number 3, Column 7:".
  const chrome = /line (\d+) at column (\d+):\s*([^\n]*)/i.exec(report);
  const firefox = /Line Number (\d+), Column (\d+)/i.exec(report);
  const m = chrome ?? firefox;
  const message = (chrome ? chrome[3].replace(/Below is a rendering[\s\S]*$/i, '') : report.split('\n')[0].replace(/^XML Parsing Error:\s*/i, '')).trim() || 'XML okunamadı';
  return { message, line: m ? Number(m[1]) : 1, column: m ? Number(m[2]) : 1 };
}

const ENTITIES: Record<string, string> = { nbsp: '#160', copy: '#169', reg: '#174', deg: '#176', middot: '#183', ndash: '#8211', mdash: '#8212', hellip: '#8230', laquo: '#171', raquo: '#187' };

/** Common breakages of hand-made and exported files, mended for strict XML. */
function mend(text: string): string {
  let t = text.replace(/&([a-z]+);/gi, (m, name: string) => (/^(amp|lt|gt|quot|apos)$/i.test(name) ? m : ENTITIES[name.toLowerCase()] ? `&${ENTITIES[name.toLowerCase()]};` : `&amp;${name};`));
  // Prefixes (xlink:, inkscape:, sodipodi: …) used without a declaration.
  const root = /<svg\b[^>]*>/i.exec(t);
  if (root) {
    const declared = new Set([...root[0].matchAll(/xmlns:([\w-]+)=/g)].map((m) => m[1]));
    const used = new Set([...t.matchAll(/\s([a-zA-Z][\w-]*):[\w-]+\s*=/g)].map((m) => m[1]).filter((p) => p !== 'xmlns' && p !== 'xml'));
    const missing = [...used].filter((p) => !declared.has(p));
    if (missing.length) {
      const decl = missing.map((p) => ` xmlns:${p}="${p === 'xlink' ? 'http://www.w3.org/1999/xlink' : `urn:kentos:${p}`}"`).join('');
      t = t.replace(/<svg\b/i, `<svg${/\sxmlns\s*=/.test(root[0]) ? '' : ' xmlns="http://www.w3.org/2000/svg"'}${decl}`);
    }
  }
  return t;
}

/** The `<svg>` root of a text: strict XML, mended XML, then the lenient HTML parser; null when none reads. */
export function parseSvgRoot(text: string): Element | null {
  const clean = sanitizeSvg(text);
  const strict = (t: string) => {
    const xml = new DOMParser().parseFromString(t, 'image/svg+xml');
    const root = xml.documentElement;
    return root && root.localName === 'svg' && !xml.getElementsByTagName('parsererror').length ? root : null;
  };
  const root = strict(clean) ?? strict(mend(clean));
  if (root) return root;
  const html = new DOMParser().parseFromString(clean, 'text/html');
  return html.querySelector('svg');
}

/** Does this text look like SVG markup (clipboard, dropped text)? */
export const looksLikeSvg = (text: string) => /<svg[\s>]/i.test(text.slice(0, 4000));

export function readSvg(text: string, opts: ImportOptions = {}): ImportResult | { error: string } {
  const root = parseSvgRoot(text);
  if (!root) return { error: 'Dosya okunabilir bir SVG çizimi değil.' };
  return docFromSvgTree(toNode(root, false), opts);
}
