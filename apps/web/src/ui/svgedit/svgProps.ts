import type { Matrix } from '../../style/svg/pathData';
import { SNAP_KINDS } from '../../style/svg/snapping';
import { rotation, shapeBox, shapesBox, transformShape, translate, type Paint, type SvgDoc, type SvgShape } from '../../style/svg/svgModel';
import { h, type Child } from '../dom';
import { icon } from '../icons';
import { checkbox, numberInput, pair, row, select, textInput } from '../style/designerFields';
import type { EditActions, PathOpId, Tab } from './svgActions';
import { alignTab } from './svgAlign';
import { arrayTab } from './svgArray';
import type { SvgCanvas } from './svgCanvas';
import { svgIcon } from './svgIcons';
import { nodeBox } from './svgNodeProps';
import { boxFields, strokeStyle } from './svgStyleProps';
import { transformTab } from './svgTransform';
import type { CanvasOptions, ToolId } from './svgView';

/**
 * The SVG editor's right column, in tabs: Özellikler (with nothing
 * selected the canvas: size, grid, snapping, rulers, tile preview, preview
 * colours; with a selection its paint, stroke and its look, box, geometry,
 * path operations, order and selection helpers; the node tool's box and
 * the polygon tool's settings on top while they are in use), Hizala,
 * Dönüştür and Dizi.
 */

export interface PropsHost {
  readonly doc: SvgDoc;
  readonly selection: ReadonlySet<string>;
  readonly options: CanvasOptions;
  readonly tool: ToolId;
  readonly nodeEdit: string | null;
  readonly edit: EditActions;
  readonly canvas: SvgCanvas;
  /** One undo step (typing into one field within a second coalesces). */
  change(key: string, fn: () => void): void;
  setOption(patch: Partial<CanvasOptions>): void;
  select(ids: string[]): void;
  editNodes(id: string | null): void;
  action(name: ActionName): void;
  refresh(): void;
}

export type ActionName = 'front' | 'back' | 'raise' | 'lower' | 'group' | 'ungroup' | 'duplicate' | 'delete' | 'flipH' | 'flipV' | 'rot90' | 'alignL' | 'alignC' | 'alignR' | 'alignT' | 'alignM' | 'alignB';

const TABS: { id: Tab; label: string; key: string }[] = [
  { id: 'props', label: 'Özellikler', key: '' },
  { id: 'align', label: 'Hizala', key: 'Ctrl+Shift+A' },
  { id: 'transform', label: 'Dönüştür', key: 'Ctrl+Shift+M' },
  { id: 'array', label: 'Dizi', key: '' },
];

export function renderProps(host: PropsHost): HTMLElement {
  const ui = host.edit.ui;
  const tabs = h(
    'div',
    { class: 'svgp__tabs', role: 'tablist', 'aria-label': 'Sağ panel' },
    TABS.map((t) => {
      const b = h('button', { class: 'tab svgp__tab', type: 'button', role: 'tab', 'aria-selected': String(ui.tab === t.id), title: t.key ? `${t.label} (${t.key})` : t.label }, t.label);
      b.addEventListener('click', () => {
        ui.tab = t.id;
        host.edit.previewArray();
        host.refresh();
      });
      return b;
    }),
  );
  const count = host.doc.shapes.filter((s) => host.selection.has(s.id)).length;
  const refresh = () => host.refresh();
  const body = ui.tab === 'align' ? alignTab(host.edit, count) : ui.tab === 'transform' ? transformTab(host.edit, host.canvas, count, refresh) : ui.tab === 'array' ? arrayTab(host.edit, host.canvas, count, refresh) : propsTab(host);
  return h('div', { class: 'svgp' }, tabs, body);
}

function propsTab(host: PropsHost): HTMLElement {
  const sel = host.doc.shapes.filter((s) => host.selection.has(s.id));
  const top: Child[] = [];
  if (host.tool === 'polygon') top.push(polygonOptions(host));
  if (host.nodeEdit && host.canvas.nodes.shape) top.push(nodeBox(host.edit, host.canvas, () => host.refresh()));
  return h('div', { class: 'sdf__form' }, top, sel.length ? selectionProps(host, sel) : canvasProps(host));
}

const PAINTS: { value: string; label: string }[] = [
  { value: 'none', label: 'Yok' },
  { value: 'fill', label: 'Sembol rengi' },
  { value: 'stroke', label: 'İkinci renk' },
  { value: 'fixed', label: 'Sabit renk' },
];

function paintField(label: string, value: Paint | null, onChange: (p: Paint) => void): HTMLElement {
  const kind = value === null ? 'none' : value === 'none' || value === 'fill' || value === 'stroke' ? value : 'fixed';
  const host = h('div', { class: 'svgp__paint' });
  const render = (k: string, color: string) => {
    const sel = select(k, PAINTS, (v) => {
      if (v === 'fixed') {
        onChange(color);
        render('fixed', color);
      } else {
        onChange(v);
        render(v, color);
      }
    }, label);
    const picker = h('input', { type: 'color', class: 'svgp__color', value: color.slice(0, 7), 'aria-label': `${label} rengi` });
    picker.addEventListener('input', () => onChange(picker.value.toUpperCase()));
    host.replaceChildren(sel, ...(k === 'fixed' ? [picker] : []));
  };
  render(kind, kind === 'fixed' && value ? value : '#E0457B');
  return row(label, host, value === null ? 'Seçilenlerde farklı' : undefined);
}

const same = <T>(list: readonly T[]): T | null => (list.length && list.every((v) => v === list[0]) ? list[0] : null);

function polygonOptions(host: PropsHost): HTMLElement {
  const o = host.options;
  return h(
    'div',
    { class: 'svgp__group svgp__group--tool' },
    h('div', { class: 'sdf__grouptitle' }, 'Çokgen aracı'),
    pair(row('Kenar sayısı', numberInput(o.sides, (v) => host.setOption({ sides: Math.max(3, Math.round(v)) }), { label: 'Kenar sayısı', min: 3, max: 24, step: 1 })), row('Biçim', checkbox(o.star, (v) => host.setOption({ star: v }), 'Yıldız'))),
  );
}

/** An icon button for the panel's action rows. */
function act(iconName: string, label: string, run: () => void, disabled = false): HTMLButtonElement {
  const b = h('button', { class: 'ibtn svgp__act', type: 'button', title: label, 'aria-label': label, disabled }, svgIcon(iconName, 16));
  b.addEventListener('click', run);
  return b;
}

function selectionProps(host: PropsHost, sel: SvgShape[]): HTMLElement {
  const set = (key: string, fn: (s: SvgShape) => SvgShape) =>
    host.change(key, () => {
      host.doc.shapes = host.doc.shapes.map((s) => (host.selection.has(s.id) ? fn(s) : s));
    });
  const locked = sel.some((s) => s.locked);
  const parts: Child[] = [
    h('div', { class: 'svgp__title' }, sel.length === 1 ? (sel[0].name ?? KIND[sel[0].kind]) : `${sel.length} şekil`, locked ? h('span', { class: 'svgp__locked' }, svgIcon('lock', 13), 'kilitli') : null),
    paintField('Dolgu', same(sel.map((s) => s.fill)), (p) => set('fill', (s) => ({ ...s, fill: p }))),
    paintField('Çizgi', same(sel.map((s) => s.stroke)), (p) => set('stroke', (s) => ({ ...s, stroke: p }))),
    pair(
      row('Çizgi kalınlığı', numberInput(same(sel.map((s) => s.strokeWidth)) ?? sel[0].strokeWidth, (v) => set('sw', (s) => ({ ...s, strokeWidth: Math.max(0, v) })), { label: 'Çizgi kalınlığı', min: 0, step: 0.5 })),
      row('Saydamlık', numberInput(Math.round((1 - (same(sel.map((s) => s.opacity ?? 1)) ?? 1)) * 100), (v) => set('op', (s) => ({ ...s, opacity: v > 0 ? 1 - Math.min(100, v) / 100 : undefined })), { label: 'Saydamlık', unit: '%', min: 0, max: 100, step: 5 })),
    ),
    strokeStyle(host, sel),
    boxFields(host, sel),
  ];
  if (sel.length === 1) parts.push(geometry(sel[0], (key, s) => host.change(key, () => (host.doc.shapes = host.doc.shapes.map((x) => (x.id === s.id ? s : x))))));
  parts.push(pathGroup(host, sel), orderGroup(host), pickGroup(host));
  if (sel.length === 1 && sel[0].kind === 'path' && !host.nodeEdit) {
    const edit = h('button', { class: 'btn btn--small', type: 'button' }, icon('vertex', 14), 'Düğümleri düzenle');
    edit.addEventListener('click', () => host.editNodes(sel[0].id));
    parts.push(edit);
  }
  if (sel.length === 1 && (sel[0].kind === 'rect' || sel[0].kind === 'ellipse')) {
    const conv = h('button', { class: 'btn btn--small', type: 'button', title: 'Düğümleri düzenlemek ya da köşe yuvarlamak için (Ctrl+Shift+C)' }, svgIcon('toPath', 14), 'Yola çevir');
    conv.addEventListener('click', () => host.edit.path('toPath'));
    parts.push(conv);
  }
  return h('div', { class: 'sdf__form' }, parts);
}

/** Path operations as buttons, with the distance and tolerance they use. */
function pathGroup(host: PropsHost, sel: SvgShape[]): HTMLElement {
  const e = host.edit;
  const p = (op: PathOpId, iconName: string, label: string, min = 1) => act(iconName, label, () => e.path(op), sel.length < min);
  return h(
    'div',
    { class: 'svgp__group' },
    h('div', { class: 'sdf__grouptitle' }, 'Yol'),
    h(
      'div',
      { class: 'svgp__acts' },
      p('union', 'pathUnion', 'Birleşim (Ctrl++)', 2),
      p('difference', 'pathDifference', 'Fark: alttakinden üsttekiler çıkar (Ctrl+-)', 2),
      p('intersection', 'pathIntersection', 'Kesişim (Ctrl+*)', 2),
      p('exclusion', 'pathExclusion', 'Dışlama: ortak yerler boşalır (Ctrl+^)', 2),
      p('division', 'pathDivision', 'Bölme: alttaki üsttekilerin çizgileriyle bölünür (Ctrl+/)', 2),
      p('cut', 'pathCut', 'Yolu kes: alttakinin çizgisi kesişimlerde kesilir (Ctrl+Alt+/)', 2),
    ),
    h(
      'div',
      { class: 'svgp__acts' },
      p('combine', 'pathCombine', 'Tek yolda topla (Ctrl+K)', 2),
      p('breakApart', 'pathBreak', 'Parçalara ayır (Ctrl+Shift+K)'),
      p('split', 'pathSplit', 'Parçalara ayır, delikler yerinde kalsın'),
      p('toPath', 'toPath', 'Nesneyi yola çevir (Ctrl+Shift+C)'),
      p('strokeToPath', 'strokeToPath', 'Çizgiyi yola çevir (Ctrl+Alt+C)'),
      p('reverse', 'reverse', 'Yönü çevir'),
      p('close', 'closePath', 'Yolu kapat'),
      p('open', 'openPath', 'Yolu aç (kapanış parçası kalır)'),
    ),
    h(
      'div',
      { class: 'svgp__acts' },
      p('inset', 'inset', 'İçe küçült (Ctrl+()'),
      p('outset', 'outset', 'Dışa büyüt (Ctrl+))'),
      numberInput(e.ui.offset, (v) => (e.ui.offset = Math.max(0, v)), { label: 'Küçültme / büyütme mesafesi', step: 0.5, min: 0 }),
      select(e.ui.offsetJoin, [
        { value: 'round', label: 'Yuvarlak' },
        { value: 'miter', label: 'Sivri' },
        { value: 'bevel', label: 'Pah' },
      ], (v) => (e.ui.offsetJoin = v), 'Açılan köşeler'),
    ),
    h('div', { class: 'svgp__acts' }, p('simplify', 'simplify', 'Sadeleştir (Ctrl+L)'), numberInput(e.ui.simplify, (v) => (e.ui.simplify = Math.max(0.001, v)), { label: 'Sadeleştirme toleransı', unit: '%', step: 0.1, min: 0.001 })),
    h('div', { class: 'sdf__hint' }, 'Sonuç alttaki şeklin boyasını alır; eğriler eğri kalır. Mesafe çizim biriminde, tolerans seçimin boyuna göre %.'),
  );
}

function orderGroup(host: PropsHost): HTMLElement {
  const e = host.edit;
  const b = (name: ActionName, iconName: string, label: string) => act(iconName, label, () => host.action(name));
  const t = (name: ActionName, label: string, text: Child) => {
    const el = h('button', { class: 'ibtn svgp__act', type: 'button', title: label, 'aria-label': label }, text);
    el.addEventListener('click', () => host.action(name));
    return el;
  };
  return h(
    'div',
    { class: 'svgp__group' },
    h('div', { class: 'sdf__grouptitle' }, 'Düzen'),
    h(
      'div',
      { class: 'svgp__acts' },
      act('toTop', 'En öne (Home)', () => e.restack('top')),
      act('raise', 'Bir öne (Page Up)', () => e.restack('raise')),
      act('lower', 'Bir arkaya (Page Down)', () => e.restack('lower')),
      act('toBottom', 'En arkaya (End)', () => e.restack('bottom')),
      t('flipH', 'Yatay çevir (H)', '⇋'),
      t('flipV', 'Dikey çevir (Shift+H)', '⥮'),
      b('rot90', 'rotate', '90° döndür'),
    ),
    h(
      'div',
      { class: 'svgp__acts' },
      t('group', 'Grupla (Ctrl+G)', 'Grupla'),
      t('ungroup', 'Grubu çöz (Ctrl+Shift+G)', 'Çöz'),
      t('duplicate', 'Çoğalt (Ctrl+D)', icon('copy', 15)),
      t('delete', 'Sil (Delete)', icon('trash', 15)),
    ),
  );
}

function pickGroup(host: PropsHost): HTMLElement {
  const e = host.edit;
  return h(
    'div',
    { class: 'svgp__group' },
    h('div', { class: 'sdf__grouptitle' }, 'Seç'),
    h(
      'div',
      { class: 'svgp__acts' },
      act('selectSame', 'Aynı dolguyu seç', () => e.selectSame('fill')),
      act('selectSame', 'Aynı çizgiyi seç', () => e.selectSame('stroke')),
      act('selectSame', 'Aynı dolgu ve çizgiyi seç', () => e.selectSame('both')),
      act('selectInvert', 'Seçimi ters çevir (!)', () => e.invertSelection()),
    ),
  );
}

const KIND: Record<SvgShape['kind'], string> = { rect: 'Dikdörtgen', ellipse: 'Elips', path: 'Yol', text: 'Yazı' };

function geometry(s: SvgShape, put: (key: string, s: SvgShape) => void): HTMLElement {
  const num = (label: string, value: number, key: string, apply: (v: number) => SvgShape, opts: { min?: number; unit?: string } = {}) =>
    row(label, numberInput(value, (v) => put(key, apply(v)), { label, min: opts.min, unit: opts.unit, step: 1 }));
  const parts: Child[] = [];
  switch (s.kind) {
    case 'rect':
      parts.push(pair(num('Köşe yarıçapı', s.r ?? 0, 'r', (v) => ({ ...s, r: v > 0 ? v : undefined }), { min: 0 }), num('Döndürme', s.rotate ?? 0, 'rot', (v) => ({ ...s, rotate: v || undefined }), { unit: '°' })));
      break;
    case 'ellipse':
      parts.push(
        pair(num('Merkez X', s.cx, 'cx', (v) => ({ ...s, cx: v })), num('Merkez Y', s.cy, 'cy', (v) => ({ ...s, cy: v }))),
        pair(num('Yarıçap X', s.rx, 'rx', (v) => ({ ...s, rx: Math.max(0.01, v) }), { min: 0.01 }), num('Yarıçap Y', s.ry, 'ry', (v) => ({ ...s, ry: Math.max(0.01, v) }), { min: 0.01 })),
        num('Döndürme', s.rotate ?? 0, 'rot', (v) => ({ ...s, rotate: v || undefined }), { unit: '°' }),
      );
      break;
    case 'text':
      parts.push(
        row('Metin', textInput(s.text, (v) => put('text', { ...s, text: v }), { label: 'Metin' })),
        pair(num('X', s.x, 'x', (v) => ({ ...s, x: v })), num('Y (taban çizgisi)', s.y, 'y', (v) => ({ ...s, y: v }))),
        pair(num('Boyut', s.size, 'size', (v) => ({ ...s, size: Math.max(0.1, v) }), { min: 0.1 }), num('Döndürme', s.rotate ?? 0, 'rot', (v) => ({ ...s, rotate: v || undefined }), { unit: '°' })),
        pair(
          row('Yazı tipi', select(s.font, [{ value: 'sans', label: 'Arial' }, { value: 'serif', label: 'Times' }], (v) => put('font', { ...s, font: v }), 'Yazı tipi')),
          row('Kalınlık', select(String(s.weight), [{ value: '400', label: 'Normal' }, { value: '700', label: 'Kalın' }, { value: '900', label: 'Siyah' }], (v) => put('weight', { ...s, weight: Number(v) as 400 | 700 | 900 }), 'Kalınlık')),
        ),
        row('Hizalama', select(s.anchor, [{ value: 'start', label: 'Soldan' }, { value: 'middle', label: 'Ortadan' }, { value: 'end', label: 'Sağdan' }], (v) => put('anchor', { ...s, anchor: v }), 'Hizalama')),
      );
      break;
    case 'path': {
      const nodes = s.subs.reduce((n, sp) => n + sp.nodes.length, 0);
      parts.push(h('div', { class: 'sdf__hint' }, `${s.subs.length} parça, ${nodes} düğüm. Çift tık ya da “Düğümleri düzenle” ile düğümler sürüklenir.`));
      if (s.subs.length === 1) parts.push(row('Uçlar', checkbox(s.subs[0].closed, (v) => put('closed', { ...s, subs: [{ ...s.subs[0], closed: v }] }), 'Kapalı şekil')));
      break;
    }
  }
  return h('div', { class: 'svgp__group' }, h('div', { class: 'sdf__grouptitle' }, 'Geometri'), parts);
}

function canvasProps(host: PropsHost): HTMLElement {
  const o = host.options;
  const doc = host.doc;
  const swatch = (label: string, value: string, key: 'ink' | 'second') => {
    const picker = h('input', { type: 'color', class: 'svgp__color', value, 'aria-label': label });
    // Picking a symbol colour stops it following the theme.
    picker.addEventListener('input', () => host.setOption(key === 'ink' ? { ink: picker.value, inkAuto: false } : { second: picker.value }));
    return row(label, picker);
  };
  const kinds = new Set(o.snapKinds);
  const toggleKind = (k: (typeof SNAP_KINDS)[number]['kind'], on: boolean) => {
    const next = new Set(kinds);
    if (on) next.add(k);
    else next.delete(k);
    host.setOption({ snapKinds: SNAP_KINDS.map((x) => x.kind).filter((x) => next.has(x)) });
  };
  return h(
    'div',
    { class: 'sdf__form' },
    h('div', { class: 'svgp__title' }, 'Tuval'),
    pair(
      row('Genişlik', numberInput(doc.width, (v) => host.change('dw', () => (doc.width = Math.max(1, v))), { label: 'Genişlik', min: 1, step: 5 })),
      row('Yükseklik', numberInput(doc.height, (v) => host.change('dh', () => (doc.height = Math.max(1, v))), { label: 'Yükseklik', min: 1, step: 5 })),
    ),
    h('div', { class: 'sdf__hint' }, 'Birim çizimin kendi birimidir; semboldeki boyutu sembol belirler (genişlik = işaret boyu).'),
    row('Izgara aralığı', numberInput(o.grid, (v) => host.setOption({ grid: Math.max(0, v) }), { label: 'Izgara aralığı', min: 0, step: 1 })),
    checkbox(o.snapGrid, (v) => host.setOption({ snapGrid: v }), 'Izgaraya kenetle'),
    checkbox(o.rulers, (v) => host.setOption({ rulers: v }), 'Cetveller (kılavuz için cetvelden sürükleyin)'),
    checkbox(o.tile, (v) => host.setOption({ tile: v }), 'Döşeme önizlemesi (desen olarak yan yana)'),
    h(
      'div',
      { class: 'svgp__group' },
      h('div', { class: 'sdf__grouptitle' }, 'Kenetleme'),
      checkbox(o.snapObjects, (v) => host.setOption({ snapObjects: v }), 'Şekillere, kılavuzlara ve tuvale kenetle'),
      h('div', { class: 'svgp__snaps' }, SNAP_KINDS.map((k) => checkbox(kinds.has(k.kind), (v) => toggleKind(k.kind, v), k.label))),
      (doc.guides ?? []).length ? clearGuides(host) : null,
    ),
    h(
      'div',
      { class: 'svgp__group' },
      h('div', { class: 'sdf__grouptitle' }, 'Önizleme renkleri'),
      pair(swatch('Sembol rengi', o.ink.startsWith('#') ? o.ink.slice(0, 7) : '#000000', 'ink'), swatch('İkinci renk', o.second.slice(0, 7), 'second')),
      checkbox(o.inkAuto, (v) => host.setOption({ inkAuto: v }), 'Sembol rengi temayı izlesin (açıkta siyah, koyuda beyaz)'),
      h('div', { class: 'sdf__hint' }, 'Yalnızca burada denemek içindir: haritada sembol hangi rengi verirse o boyanır.'),
    ),
  );
}

function clearGuides(host: PropsHost): HTMLElement {
  const b = h('button', { class: 'btn btn--small', type: 'button' }, svgIcon('guide', 14), `Kılavuzları sil (${host.doc.guides!.length})`);
  b.addEventListener('click', () => host.canvas.rulers.clearAll());
  return b;
}

/** The matrix of a flip or 90° turn (and the older align actions) on the selection. */
export function actionMatrix(name: ActionName, shapes: readonly SvgShape[], doc: SvgDoc): (s: SvgShape) => Matrix | null {
  const all = shapesBox(shapes)!;
  const ref = shapes.length > 1 ? all : { minX: 0, minY: 0, maxX: doc.width, maxY: doc.height };
  const cx = (all.minX + all.maxX) / 2;
  const cy = (all.minY + all.maxY) / 2;
  switch (name) {
    case 'flipH':
      return () => [-1, 0, 0, 1, 2 * cx, 0];
    case 'flipV':
      return () => [1, 0, 0, -1, 0, 2 * cy];
    case 'rot90':
      return () => rotation(90, cx, cy);
    default: {
      const move = (b: { minX: number; minY: number; maxX: number; maxY: number }): Matrix | null => {
        switch (name) {
          case 'alignL':
            return translate(ref.minX - b.minX, 0);
          case 'alignR':
            return translate(ref.maxX - b.maxX, 0);
          case 'alignC':
            return translate((ref.minX + ref.maxX) / 2 - (b.minX + b.maxX) / 2, 0);
          case 'alignT':
            return translate(0, ref.minY - b.minY);
          case 'alignB':
            return translate(0, ref.maxY - b.maxY);
          case 'alignM':
            return translate(0, (ref.minY + ref.maxY) / 2 - (b.minY + b.maxY) / 2);
          default:
            return null;
        }
      };
      return shapes.length > 1 ? (s) => move(shapeBox(s)) : () => move(all);
    }
  }
}

export { transformShape };
