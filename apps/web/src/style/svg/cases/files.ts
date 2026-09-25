import type { Gen } from '../../../wasm/calls/harness';
import { numberText, pathData, shapes } from './geometry';

/**
 * Test support for the SVG editor's files (docs/adr/0008 “SVG
 * düzenleyicisi”): values as files write them (colours and paints,
 * transforms, lengths, style sheets), whole SVG files (styles,
 * definitions, `<use>` and its loops, texts and `<tspan>`, units, nested
 * `<svg>`, images, the editor's own source) in the flat form the page
 * sends, import options, drawings to write and PNG bytes.
 */

/** Colours and paints as files write them: every syntax, keywords, references with fallbacks, and broken ones. */
export function colorText(g: Gen): string {
  const hex = () => `#${Array.from({ length: g.pick([3, 4, 6, 8, 5]) }, () => '0123456789abcdefABCDEF'[g.int(0, 21)]).join('')}`;
  const pct = () => `${g.int(-10, 110)}%`;
  const byte = () => String(g.pick([g.int(0, 255), g.int(-20, 300), g.num(0, 255).toFixed(1)]));
  const alpha = () => g.pick(['0.5', '.3', '50%', '1', '0', '2', 'x', '1e-1']);
  const sep = () => g.pick([', ', ',', ' ', ' / ']);
  const forms = [
    hex,
    () => g.pick(['red', 'RebeccaPurple', 'black', 'white', 'darkslategrey', 'transparent', 'none', 'None', 'currentColor', 'CURRENTCOLOR', 'inherit', '', 'nonsense', 'constructor', 'gray']),
    () => `rgb(${byte()}${sep()}${byte()}${sep()}${byte()})`,
    () => `rgba(${g.chance(0.5) ? pct() : byte()}${sep()}${byte()}${sep()}${byte()}${sep()}${alpha()})`,
    () => `rgb(${byte()} ${byte()}${g.chance(0.3) ? '' : ` ${byte()}`} / ${alpha()})`,
    () => `hsl(${g.int(-400, 400)}${sep()}${pct()}${sep()}${pct()})`,
    () => `hsla(${g.pick(['120', 'x', '30.5'])} ${pct()} ${pct()} / ${alpha()})`,
    () => g.pick(['param(fill)', 'param(stroke) #123456', 'param(fill-opacity)', 'PARAM(stroke)']),
    () => `url(${g.pick(['#grad', "'#grad2'", '"#pat"', '#missing', ' #clip ', '#grad)', 'foo', '#'])})${g.chance(0.5) ? ` ${g.pick(['red', 'none', 'url(#grad)', 'inherit', '#00ff0080', 'url(#pat) blue', 'x'])}` : ''}`,
    () => `URL(#grad)`,
    () => g.pick(['url(#pat)', 'url(#grad)', 'url(#grad2) red', 'url(#pat) none', 'url(#sym)']),
    () => `rgb(1,2)`,
    () => `rgb(1, 2, 3) x`,
    () => `#1D1D1B`,
    () => `#231f20`,
  ];
  const s = g.pick(forms)();
  return g.chance(0.15) ? g.pick([` ${s} `, `\n${s}\t`, s.toUpperCase()]) : s;
}

/** Transform attributes: every function, lists with and without commas, bad argument counts and junk between. */
export function transformText(g: Gen): string {
  const n = () => numberText(g);
  const deg = () => String(g.pick([0, 90, -45, 30, 180, g.int(-360, 360), Number(g.num(-360, 360).toFixed(2))]));
  const sep = () => g.pick([' ', ',', ', ', '']);
  const one = () =>
    g.pick([
      () => `translate(${n()}${g.chance(0.7) ? `${sep() || ' '}${n()}` : ''})`,
      () => `scale(${g.pick(['2', '0.5', '-1', '1.5'])}${g.chance(0.5) ? ` ${g.pick(['1', '3', '-2'])}` : ''})`,
      () => `rotate(${deg()}${g.chance(0.4) ? ` ${n()} ${n()}` : ''})`,
      () => `skewX(${deg()})`,
      () => `skewY(${deg()})`,
      () => `matrix(${Array.from({ length: g.pick([6, 6, 6, 4, 7]) }, () => g.pick(['1', '0', '-1', '0.5', '2', n()])).join(sep() || ' ')})`,
      () => g.pick(['translate (5)', 'rotate(90', 'skew(10)', 'translate()', 'scale(2)x', 'foo(1)']),
    ])();
  return Array.from({ length: g.int(0, 3) }, one).join(g.pick([' ', ',', '', ' , ']));
}

/** Lengths: units, percentages, font units, signs, exponents and what is not a length. */
export function lengthText(g: Gen): string {
  const v = g.pick([String(g.int(0, 120)), Number(g.num(0, 100).toFixed(2)).toString(), '.5', '-3', '1e1', '+4', '0']);
  const u = g.pick(['', '', 'px', 'mm', 'cm', 'in', 'pt', 'pc', 'q', '%', 'em', 'ex', 'rem', 'PX', 'Mm', ' mm', 'x', 'm']);
  return g.pick([`${v}${u}`, `${v}${u}`, `${v}${u}`, ` ${v}${u} `, 'auto', '', 'abc', `${v} ${v}`]);
}

const IDS = ['a', 'b', 'grad', 'grad2', 'pat', 'sym', 'clip', 'g1', 'r1', 'x', '1n', 'constructor'];
const CLASSES = ['a', 'b', 'k', 'l', 'red'];

/** Style sheets: tags, classes, ids, attributes, compounds, descendants and children, lists, !important, at-rules, comments, pseudo-classes. */
export function cssText(g: Gen): string {
  const compound = () => {
    const parts: string[] = [];
    if (g.chance(0.5)) parts.push(g.pick(['rect', 'g', 'path', 'circle', 'text', 'tspan', 'svg', 'RECT', '*', 'stop']));
    for (let i = g.int(0, 2); i > 0; i--)
      parts.push(
        g.pick([
          () => `.${g.pick(CLASSES)}`,
          () => `#${g.pick(IDS)}`,
          () => `[${g.pick(['id', 'class', 'fill', 'data-x', 'constructor', 'xlink:href'])}]`,
          () => `[${g.pick(['id', 'fill', 'class'])}=${g.pick(['"a"', "'b'", 'red', '""', '"a\'b"', "'x"])}]`,
        ])(),
      );
    return parts.join('') || g.pick(['rect', '.a', '*']);
  };
  const selector = () => Array.from({ length: g.int(1, 3) }, compound).join(g.pick([' ', ' > ', '>', '  ']));
  const decl = () => {
    const prop = g.pick(['fill', 'stroke', 'stroke-width', 'opacity', 'fill-opacity', 'stroke-opacity', 'display', 'visibility', 'font-size', 'stroke-dasharray', 'stop-color', 'stop-opacity', 'Fill', 'color', 'clip-path', 'marker-end', 'x']);
    const value = prop.includes('color') || prop.toLowerCase().includes('fill') || prop === 'stroke' ? colorText(g) : prop === 'display' ? g.pick(['none', 'inline']) : prop === 'visibility' ? g.pick(['hidden', 'visible', 'collapse']) : lengthText(g);
    return `${prop}:${g.pick([' ', ''])}${value}${g.chance(0.15) ? g.pick([' !important', '!IMPORTANT', ' ! important ']) : ''}`;
  };
  const rule = () => {
    const sels = Array.from({ length: g.int(1, 3) }, () => (g.chance(0.1) ? g.pick(['a:hover', 'a + b', 'g ~ rect', 'rect::before']) : selector()));
    return `${sels.join(g.pick([', ', ',']))} { ${Array.from({ length: g.int(0, 3) }, decl).join('; ')}${g.chance(0.5) ? ';' : ''} }`;
  };
  const parts = Array.from({ length: g.int(1, 4) }, () =>
    g.pick([rule, rule, rule, () => `@media print { ${rule()} }`, () => `/* ${rule()} */`, () => '<!-- -->', () => '@font-face { font-family: x; }'])(),
  );
  if (g.chance(0.1)) parts.push(g.pick(['rect { fill: red', 'x', '/* open']));
  return parts.join(g.pick(['\n', ' ', '']));
}

type Xml = { tag: string; attrs: Record<string, string>; children: Xml[]; text?: string };

const el = (tag: string, attrs: Record<string, string> = {}, children: Xml[] = [], text?: string): Xml => (text === undefined ? { tag, attrs, children } : { tag, attrs, children, text });

/** Presentation attributes, classes, ids, styles and transforms of an element, each present or not. */
function presentation(g: Gen, a: Record<string, string>): Record<string, string> {
  if (g.chance(0.25)) a.id = g.pick(IDS);
  if (g.chance(0.25)) a.class = Array.from({ length: g.int(1, 2) }, () => g.pick(CLASSES)).join(' ');
  if (g.chance(0.35)) a.fill = colorText(g);
  if (g.chance(0.3)) a.stroke = colorText(g);
  if (g.chance(0.2)) a['stroke-width'] = lengthText(g);
  if (g.chance(0.15)) a.opacity = g.pick(['0.5', '1', '50%', '2', 'x', '.25']);
  if (g.chance(0.1)) a['fill-opacity'] = g.pick(['0.5', '40%', 'x', '1']);
  if (g.chance(0.1)) a['stroke-opacity'] = g.pick(['0.3', '0', '100%']);
  if (g.chance(0.1)) a['stroke-dasharray'] = g.pick(['4 2', '1', '3,1,0.5', 'none', '0 0', '2 -1', '5mm 2', '1em']);
  if (g.chance(0.1)) a['stroke-linecap'] = g.pick(['butt', 'round', 'square', 'x']);
  if (g.chance(0.1)) a['stroke-linejoin'] = g.pick(['miter', 'round', 'bevel', 'miter-clip', 'arcs']);
  if (g.chance(0.1)) a['fill-rule'] = g.pick(['nonzero', 'evenodd']);
  if (g.chance(0.08)) a.display = g.pick(['none', 'inline', ' none']);
  if (g.chance(0.08)) a.visibility = g.pick(['hidden', 'visible', 'collapse', 'inherit']);
  if (g.chance(0.08)) a[g.pick(['clip-path', 'mask', 'filter', 'marker-end', 'marker'])] = g.pick(['url(#clip)', "url('#m1')", 'none', 'url(x) url(#a)']);
  if (g.chance(0.2)) a.transform = transformText(g);
  if (g.chance(0.15)) a.style = Array.from({ length: g.int(1, 3), }, () => `${g.pick(['fill', 'stroke', 'opacity', 'stroke-width', 'display', 'font-size'])}: ${g.chance(0.5) ? colorText(g) : lengthText(g)}${g.chance(0.1) ? ' !important' : ''}`).join(';');
  if (g.chance(0.05)) a['inkscape:label'] = g.pick(['Katman 1', ' ', 'Işık kulesi — çok uzun bir ad olan bu etiketin altmış karakterden sonrası kesilir ✓']);
  if (g.chance(0.05)) a['data-name'] = g.pick(['Ad', '']);
  return a;
}

function shapeEl(g: Gen): Xml {
  const n = () => numberText(g);
  const L = () => (g.chance(0.8) ? n() : lengthText(g));
  const kind = g.pick(['rect', 'circle', 'ellipse', 'line', 'polyline', 'polygon', 'path', 'path', 'text']);
  const a: Record<string, string> = {};
  if (kind === 'rect') {
    a.width = L();
    a.height = L();
    if (g.chance(0.7)) a.x = L();
    if (g.chance(0.7)) a.y = L();
    if (g.chance(0.4)) a.rx = g.pick(['auto', '3', '50%', L()]);
    if (g.chance(0.4)) a.ry = g.pick(['auto', '5', L()]);
  } else if (kind === 'circle') {
    a.r = L();
    a.cx = L();
    a.cy = L();
  } else if (kind === 'ellipse') {
    if (g.chance(0.9)) a.rx = g.pick(['auto', L()]);
    if (g.chance(0.9)) a.ry = g.pick(['auto', L()]);
    a.cx = L();
  } else if (kind === 'line') {
    for (const k of ['x1', 'y1', 'x2', 'y2']) if (g.chance(0.85)) a[k] = L();
  } else if (kind === 'polyline' || kind === 'polygon') {
    a.points = Array.from({ length: g.int(0, 6) }, () => `${n()}${g.pick([',', ' '])}${n()}`).join(' ') + (g.chance(0.2) ? ` ${n()}` : '');
  } else if (kind === 'path') {
    if (g.chance(0.95)) a.d = pathData(g);
  } else {
    return textEl(g);
  }
  presentation(g, a);
  const kids: Xml[] = [];
  if (g.chance(0.08)) kids.push(el('title', {}, g.chance(0.5) ? [el('#text', {}, [], g.pick(['Başlık', '  '])) ] : [], g.chance(0.5) ? 'Ad' : undefined));
  return el(kind, a, kids);
}

function textEl(g: Gen): Xml {
  const pos = (a: Record<string, string>) => {
    if (g.chance(0.6)) a.x = g.pick([numberText(g), `${numberText(g)} ${numberText(g)}`, '1em', '10%', 'x']);
    if (g.chance(0.6)) a.y = g.pick([numberText(g), '2em', '50%']);
    if (g.chance(0.2)) a.dx = g.pick(['5', '1em', '-2 3']);
    if (g.chance(0.2)) a.dy = g.pick(['1.2em', '4']);
    return a;
  };
  const words = () => g.pick(['Parsel', ' 12 ', '\n  ', 'Işık\tkulesi', '', 'ağaç ✓', 'A  B', '🌳🌲 orman']);
  const runs = (depth: number): Xml[] =>
    Array.from({ length: g.int(0, 3) }, () =>
      depth < 2 && g.chance(0.4)
        ? el(g.pick(['tspan', 'tspan', 'textPath', 'a', 'TSPAN', 'title']), presentation(g, pos({})), runs(depth + 1), g.chance(0.2) ? words() : undefined)
        : el('#text', {}, [], words()),
    );
  const a = pos({});
  if (g.chance(0.4)) a['font-size'] = g.pick(['12', 'large', 'xx-small', '150%', '2em', 'constructor', 'toString', '8pt', 'inherit']);
  if (g.chance(0.3)) a['font-family'] = g.pick(['serif', 'Times, serif', 'sans-serif', 'Arial', 'Georgia']);
  if (g.chance(0.3)) a['font-weight'] = g.pick(['bold', 'normal', 'bolder', 'lighter', '600', '900', '300', 'x', '0']);
  if (g.chance(0.3)) a['text-anchor'] = g.pick(['start', 'middle', 'end', 'x']);
  presentation(g, a);
  const kids = runs(0);
  return el('text', a, kids, kids.length === 0 && g.chance(0.7) ? words() : undefined);
}

function defsEl(g: Gen): Xml {
  const stop = () => {
    const a: Record<string, string> = { offset: g.pick(['0', '0.5', '1']) };
    if (g.chance(0.7)) a['stop-color'] = colorText(g);
    if (g.chance(0.4)) a['stop-opacity'] = g.pick(['0.5', '1', 'x', '2']);
    if (g.chance(0.2)) a.style = `stop-color: ${colorText(g)}`;
    return el(g.pick(['stop', 'stop', 'STOP']), a);
  };
  const items = Array.from({ length: g.int(1, 4) }, () =>
    g.pick([
      () => el(g.pick(['linearGradient', 'radialGradient']), { id: g.pick(['grad', 'grad2']), ...(g.chance(0.3) ? { href: `#${g.pick(['grad', 'grad2', 'x'])}` } : {}) }, Array.from({ length: g.int(0, 3) }, stop)),
      () => el('pattern', { id: 'pat' }, g.chance(0.8) ? [g.chance(0.3) ? el('g', { fill: colorText(g) }, [shapeEl(g)]) : shapeEl(g)] : []),
      () => el('symbol', { id: 'sym', ...(g.chance(0.6) ? { viewBox: g.pick(['0 0 10 10', '0 0 20 10', '5 5 10 10']) } : {}), ...(g.chance(0.3) ? { preserveAspectRatio: g.pick(['none', 'xMinYMax slice']) } : {}) }, Array.from({ length: g.int(1, 3) }, () => shapeEl(g))),
      () => el(g.pick(['clipPath', 'mask', 'filter', 'marker']), { id: g.pick(['clip', 'm1']) }, [shapeEl(g)]),
      () => el('style', g.chance(0.2) ? { type: g.pick(['text/css', 'text/x', '']) } : {}, [], cssText(g)),
    ])(),
  );
  return el('defs', {}, items);
}

function anyEl(g: Gen, depth: number): Xml {
  const pickKind = depth >= 3 ? 0 : g.int(0, 11);
  if (pickKind <= 4) return shapeEl(g);
  if (pickKind === 5 || pickKind === 6) {
    const a = presentation(g, {});
    if (g.chance(0.1)) a['inkscape:groupmode'] = 'layer';
    if (g.chance(0.15)) a['data-group'] = g.pick(['grp', '']);
    return el(g.pick(['g', 'g', 'a', 'switch']), a, Array.from({ length: g.int(0, 3) }, () => anyEl(g, depth + 1)));
  }
  if (pickKind === 7) {
    const a = presentation(g, { href: `#${g.pick([...IDS, 'missing', ''])}` });
    if (g.chance(0.3)) a.x = numberText(g);
    if (g.chance(0.3)) a.y = numberText(g);
    if (g.chance(0.2)) a.width = lengthText(g);
    return el('use', g.chance(0.2) ? { 'xlink:href': a.href, ...a, href: undefined as unknown as string } : a);
  }
  if (pickKind === 8) {
    const a: Record<string, string> = {};
    if (g.chance(0.6)) a.viewBox = g.pick(['0 0 10 10', '0 0 50 20', '-5 -5 10 10', '0 0 0 10']);
    if (g.chance(0.5)) a.width = lengthText(g);
    if (g.chance(0.5)) a.height = lengthText(g);
    if (g.chance(0.4)) a.x = numberText(g);
    if (g.chance(0.3)) a.preserveAspectRatio = g.pick(['none', 'xMaxYMin', 'xMidYMid slice', 'defer xMinYMin meet']);
    return el('svg', presentation(g, a), Array.from({ length: g.int(0, 3) }, () => anyEl(g, depth + 1)));
  }
  if (pickKind === 9) return defsEl(g);
  if (pickKind === 10)
    return el('image', g.chance(0.5) ? { 'data-kentos': 'reference', href: g.pick(['data:image/png;base64,AAA', 'x.png', 'data:image/jpeg;base64,BB']), x: numberText(g), y: numberText(g), width: g.pick(['10', 'x']), ...(g.chance(0.5) ? { 'data-locked': g.pick(['0', '1']) } : {}), ...(g.chance(0.5) ? { 'data-name': 'Altlık 2' } : {}), ...(g.chance(0.3) ? { opacity: '0.3' } : {}) } : { href: 'x.png' });
  return el(g.pick(['foo', 'foreignObject', '#text', 'desc', 'metadata', 'style']), {}, [], g.pick(['', ' rect { fill: blue } ', undefined as unknown as string]));
}

/** An SVG file as the page hands it over: a root with size and viewBox, styles, definitions and every kind of element. */
export function svgTree(g: Gen): Xml {
  const a: Record<string, string> = {};
  if (g.chance(0.7)) a.viewBox = g.pick(['0 0 100 100', '0 0 64 48', '-10 -10 120 80', '0,0,24,24', '0 0 0 0', '1 2 3']);
  if (g.chance(0.6)) a.width = g.pick(['100', '64mm', '2in', '100%', '50', '12pt', '10em', '5cm']);
  if (g.chance(0.6)) a.height = g.pick(['100', '48mm', '1in', '80', '100%', '30pt']);
  if (g.chance(0.2)) a.preserveAspectRatio = g.pick(['none', 'xMinYMin', ' none ']);
  if (g.chance(0.1)) a['data-size-mm'] = g.pick(['6', '0', 'x', '12.5']);
  if (g.chance(0.1)) a['data-background'] = g.pick(['#FFFFCC', 'red', 'x']);
  if (g.chance(0.3)) a.fill = colorText(g);
  const kids: Xml[] = [];
  if (g.chance(0.4)) kids.push(el('style', {}, g.chance(0.5) ? [el('#text', {}, [], cssText(g))] : [], g.chance(0.5) ? cssText(g) : undefined));
  if (g.chance(0.4)) kids.push(defsEl(g));
  for (let i = g.int(1, 6); i > 0; i--) kids.push(anyEl(g, 1));
  return el('svg', a, kids);
}

/** Options of an import: symbol colour (auto, none, black, dominant, a colour), second colour, the editor's own source. */
export function importOptions(g: Gen): Record<string, unknown> {
  const o: Record<string, unknown> = {};
  if (g.chance(0.7)) o.symbolColor = g.pick(['auto', null, 'black', 'dominant', '#FF0000', '#aa3300', '']);
  if (g.chance(0.4)) o.secondColor = g.pick([null, '#00FF00', '#aa3300', '', '#11223344']);
  if (g.chance(0.3)) o.editor = g.chance(0.7);
  return o;
}

/** A drawing to write or map: shapes with near-black and fixed colours, a size in mm and a background now and then. */
export function drawing(g: Gen): Record<string, unknown> {
  const list = shapes(g, g.int(0, 6)).map((s) => (g.chance(0.3) ? { ...s, fill: g.pick(['#000000', '#1D1D1B', '#303031', '#AA330080', '#aa3300', '#1d1d1b40']), stroke: g.pick(['none', '#000000', '#AA3300']) } : s));
  const d: Record<string, unknown> = { width: g.pick([100, 64, g.num(1, 200)]), height: g.pick([100, 48, g.num(1, 200)]), shapes: list };
  if (g.chance(0.4)) d.sizeMm = g.pick([6, 12.5, g.num(1, 30), 0]);
  if (g.chance(0.3)) d.background = g.pick(['#FFFFCC', '', '#000']);
  if (g.chance(0.2)) d.guides = [{ id: 'k0', x: 10, y: 20, angle: 90 }];
  return d;
}

/** Bytes of a PNG (or not): signature, IHDR, then chunks (old pHYs among them), sometimes cut short or with the wrong first chunk. */
export function pngBytes(g: Gen): Uint8Array {
  const out: number[] = [137, 80, 78, 71, 13, 10, 26, 10];
  const chunk = (type: string, len: number, lie = 0) => {
    const n = len + lie;
    out.push((n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255, ...[...type].map((c) => c.charCodeAt(0)));
    for (let i = 0; i < len; i++) out.push(g.int(0, 255));
    for (let i = 0; i < 4; i++) out.push(g.int(0, 255));
  };
  chunk(g.chance(0.9) ? 'IHDR' : 'IDAT', 13);
  for (let i = g.int(0, 4); i > 0; i--) chunk(g.pick(['IDAT', 'pHYs', 'tEXt', 'IEND', 'pHYs']), g.int(0, 12), g.chance(0.1) ? g.int(1, 50) : 0);
  const cut = g.chance(0.15) ? g.int(0, out.length) : out.length;
  return Uint8Array.from(out.slice(0, cut));
}

/** An SVG file's elements as the page sends them: flat, in document order, each with its parent's place. */
export function flatTree(n: Xml, parent = -1, out: [string, Record<string, string>, string | null, number][] = []): [string, Record<string, string>, string | null, number][] {
  const at = out.length;
  out.push([n.tag, n.attrs, n.text ?? null, parent]);
  for (const k of n.children) flatTree(k, at, out);
  return out;
}
