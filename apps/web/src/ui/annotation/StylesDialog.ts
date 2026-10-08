import type { AppContext } from '../../app/context';
import { saveDimensionStyles, saveTextStyles, styleUsage } from '../../app/styleTables';
import { DRAWING_FONTS, drawingFontById } from '../../app/appearance';
import { uuidv7 } from '../../core/uuid';
import {
  DIMENSION_ARROWS,
  dimensionStylesProblem,
  faceOf,
  lookOf,
  STANDARD_STYLE,
  textStylesProblem,
  type DimensionArrow,
  type DimensionStyleDef,
  type TextStyleDef,
} from '../../model/annotationStyles';
import { layoutDimension } from '../../model/geom/dimension';
import { LINE_TYPE_LABEL, LINE_TYPES, type LineType } from '../../model/layers';
import { LINE_COLORS, LINE_WEIGHTS, weightText } from '../ribbon/fields';
import { fixed } from '../../core/displayNumber';
import type { DrawingFont, DrawingUnit } from '../../model/projectSettings';
import { readCanvasPalette } from '../../render/color';
import { faceFont, leanOf, valueFont } from '../../render/drawingFaces';
import { DisposableStore } from '../../core/disposable';
import { h, replaceChildren } from '../dom';
import { checkField, field, select, summaryLine } from '../io/common';
import { askRemove, askUnsaved } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';

/** The windows' titles, which a trace names them by. */
export const TEXT_STYLES_TITLE = 'Yazı stilleri';
export const DIMENSION_STYLES_TITLE = 'Ölçü stilleri';

type Kind = 'text' | 'dimension';
type Style = TextStyleDef | DimensionStyleDef;

const ARROW_LABEL: Record<DimensionArrow | 'tick', string> = { tick: 'Çentik', closed: 'Dolu ok', open: 'Açık ok', dot: 'Nokta', none: 'Yok' };
const UNIT_LABEL: Record<DrawingUnit | 'project', string> = { project: 'Projenin', m: 'm', cm: 'cm', mm: 'mm' };

/** A number field's value: a finite number, or undefined when it is empty (the field's absence). */
const numberOf = (s: string): number | undefined | null => {
  const t = s.trim().replace(',', '.');
  if (!t) return undefined;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
};

/** A name not taken yet: `base`, else `base 2`, `base 3` … */
function freeName(base: string, styles: readonly Style[]): string {
  const taken = new Set(styles.map((s) => s.name.toLowerCase()));
  if (!taken.has(base.toLowerCase())) return base;
  for (let i = 2; ; i++) if (!taken.has(`${base} ${i}`.toLowerCase())) return `${base} ${i}`;
}

/**
 * Yazı stilleri and Ölçü stilleri (docs/adr/0183 §5; the desktop's `annotation_styles.rs`): the project's styles of a
 * kind, Standart first and not edited; each with how many objects follow it. A style chosen shows its form and a
 * preview drawn as the drawing draws it; Yeni copies the chosen one, Sil takes the chosen one away (asking when objects
 * follow it: they keep their look, without the link). The form is a draft: Kaydet writes the table to the project and
 * the objects that follow the styles in one undo step (`saveTextStyles`, `saveDimensionStyles`); Vazgeç, Esc and ×
 * leave it (asking when it changed). A CAD project's window: the command is hidden elsewhere.
 */
export function openAnnotationStyles(ctx: AppContext, kind: Kind): void {
  const settings = ctx.doc.settings;
  const saved = (): Style[] => (kind === 'text' ? [...settings.textStyles.value] : [...settings.dimensionStyles.value]);
  let draft: Style[] = structuredClone(saved());
  let chosen: string | null = draft[0]?.id ?? null;
  const usage = styleUsage(ctx, kind);
  const d = new DisposableStore();

  const list = h('div', { class: 'states-list astyle-list', role: 'listbox', 'aria-label': kind === 'text' ? 'Yazı stilleri' : 'Ölçü stilleri' });
  const form = h('div', { class: 'astyle-form' });
  const problem = h('div', { class: 'io-summary' });
  const preview = h('canvas', { class: 'astyle-preview', width: 360, height: 120, 'aria-label': 'Önizleme' }) as HTMLCanvasElement;
  const button = (words: string, primary = false) => h('button', { class: primary ? 'btn btn--primary' : 'btn', type: 'button' }, words) as HTMLButtonElement;
  const add = button('Yeni');
  const remove = button('Sil');
  const save = button('Kaydet', true);
  const cancel = button('Vazgeç');

  const current = (): Style | null => draft.find((s) => s.id === chosen) ?? null;
  const tableProblem = (): string | null => (kind === 'text' ? textStylesProblem(draft as TextStyleDef[]) : dimensionStylesProblem(draft as DimensionStyleDef[]));
  const changed = () => JSON.stringify(draft) !== JSON.stringify(saved());

  function rowOf(id: string | null, name: string): HTMLElement {
    const n = id === null ? null : (usage.get(id) ?? 0);
    const row = h(
      'button',
      { class: 'states-row', type: 'button', role: 'option', 'aria-label': name, 'aria-selected': String(id === chosen) },
      h('span', { class: 'states-row__mark' }),
      h('span', { class: 'states-row__name' }, name),
      h('span', { class: 'states-row__parts', title: n === null ? 'Stilsiz nesneler' : `${n} nesne bu stili kullanıyor` }, n === null ? '' : String(n)),
    );
    row.addEventListener('click', () => {
      chosen = id;
      refresh();
    });
    return row;
  }

  /** The chosen style's values changed through `patch` (a field set to undefined is taken away). */
  function edit(patch: Record<string, unknown>): void {
    const s = current();
    if (!s) return;
    const next: Record<string, unknown> = { ...s, ...patch };
    for (const k of Object.keys(next)) if (next[k] === undefined) delete next[k];
    draft = draft.map((x) => (x.id === s.id ? (next as unknown as Style) : x));
    refreshState();
  }

  /** A number field for `key`: empty is the field's absence; what is not a number keeps the old value and is said. */
  function numberField(label: string, key: string, value: number | undefined, hint: string, required = false): HTMLElement {
    const input = h('input', { class: 'field', type: 'text', inputmode: 'decimal', value: value === undefined ? '' : String(value), 'aria-label': label, spellcheck: 'false' }) as HTMLInputElement;
    input.addEventListener('change', () => {
      const n = numberOf(input.value);
      if (n === null || (required && n === undefined)) {
        input.value = value === undefined ? '' : String(value);
        ctx.log.warn(`${label}: “${input.value}” bir sayı değil. Noktalı ya da virgüllü bir sayı yazın${required ? '' : ' ya da alanı boş bırakın'}.`);
        return;
      }
      edit({ [key]: n });
    });
    return field(label, input, hint);
  }

  function textForm(s: TextStyleDef): HTMLElement[] {
    const fonts = DRAWING_FONTS.map((f) => ({ value: f.id, label: f.label }));
    const name = h('input', { class: 'field', type: 'text', value: s.name, 'aria-label': 'Ad', spellcheck: 'false' }) as HTMLInputElement;
    name.addEventListener('change', () => edit({ name: name.value.trim() }));
    return [
      field('Ad', name, undefined, 'grow'),
      h(
        'div',
        { class: 'io-row' },
        field('Yazı tipi', select('Yazı tipi', fonts, s.font, (v) => edit({ font: v as DrawingFont }))),
        checkField('Kalınlık', 'Kalın', !!s.bold, (v) => edit({ bold: v || undefined }), 'bold'),
        checkField('Biçim', 'İtalik', !!s.italic, (v) => edit({ italic: v || undefined }), 'italic'),
      ),
      h(
        'div',
        { class: 'io-row' },
        numberField('Eğiklik (°)', 'oblique', s.oblique, '−85 ile 85; boş: dik'),
        numberField('Yükseklik (mm)', 'height', s.height, 'Kâğıtta; boş: aracın'),
        numberField('Genişlik çarpanı', 'widthFactor', s.widthFactor, 'Boş: 1'),
      ),
      ...(s.fontFile ? [field('Yazı tipi dosyası', h('span', { class: 'astyle-file' }, s.fontFile), 'DXF dosyasından; DXF’e aynen yazılır.')] : []),
    ];
  }

  function dimensionForm(s: DimensionStyleDef): HTMLElement[] {
    const name = h('input', { class: 'field', type: 'text', value: s.name, 'aria-label': 'Ad', spellcheck: 'false' }) as HTMLInputElement;
    name.addEventListener('change', () => edit({ name: name.value.trim() }));
    const arrows = (['tick', ...DIMENSION_ARROWS] as const).map((a) => ({ value: a, label: ARROW_LABEL[a] }));
    const units = (['project', 'm', 'cm', 'mm'] as const).map((u) => ({ value: u, label: UNIT_LABEL[u] }));
    const fonts = [{ value: 'project', label: `Projenin (${drawingFontById(settings.drawingFont.value).label})` }, ...DRAWING_FONTS.map((f) => ({ value: f.id, label: f.label }))];
    const affix = (label: string, key: 'prefix' | 'suffix', value: string | undefined) => {
      const input = h('input', { class: 'field', type: 'text', value: value ?? '', 'aria-label': label, spellcheck: 'false' }) as HTMLInputElement;
      input.addEventListener('change', () => edit({ [key]: input.value === '' ? undefined : input.value }));
      return field(label, input, 'Boş: yok');
    };
    return [
      field('Ad', name, undefined, 'grow'),
      h(
        'div',
        { class: 'io-row' },
        numberField('Değer yüksekliği (mm)', 'height', s.height, 'Kâğıtta', true),
        field('Uçlar', select('Uçlar', arrows, s.arrow ?? 'tick', (v) => edit({ arrow: v === 'tick' ? undefined : v }))),
        numberField('Uç boyu (mm)', 'arrowSize', s.arrowSize, 'Boş: yüksekliğe göre'),
      ),
      h(
        'div',
        { class: 'io-row' },
        numberField('Uzatma boşluğu (mm)', 'extOffset', s.extOffset, 'Noktadan'),
        numberField('Uzatma aşması (mm)', 'extBeyond', s.extBeyond, 'Çizgiyi'),
        numberField('Değerin yüksekliği (mm)', 'textGap', s.textGap, 'Çizgiden'),
      ),
      h(
        'div',
        { class: 'io-row' },
        field('Değerin yeri', select('Değerin yeri', [{ value: 'above', label: 'Çizginin üstünde' }, { value: 'centre', label: 'Çizginin ortasında' }], s.textPlace ?? 'above', (v) => edit({ textPlace: v === 'centre' ? 'centre' : undefined }))),
        numberField('Basamak', 'decimals', s.decimals, 'Boş: projenin'),
        field('Birim', select('Birim', units, s.unit ?? 'project', (v) => edit({ unit: v === 'project' ? undefined : v }))),
      ),
      h('div', { class: 'io-row' }, affix('Önek', 'prefix', s.prefix), affix('Sonek', 'suffix', s.suffix), field('Yazı tipi', select('Yazı tipi', fonts, s.font ?? 'project', (v) => edit({ font: v === 'project' ? undefined : v })))),
      // Its lines (docs/adr/0205 §6): the dimension line's and arrowheads', the extension lines', the value's colour.
      h('div', { class: 'astyle-section' }, 'Çizgiler'),
      h(
        'div',
        { class: 'io-row' },
        field('Ölçü çizgisi', colorSelect('Ölçü çizgisinin rengi', s.dimLineColor, (v) => edit({ dimLineColor: v }))),
        field('Kalınlık', weightSelect('Ölçü çizgisinin kalınlığı', s.dimLineWeight, (v) => edit({ dimLineWeight: v }))),
        field('Tip', typeSelect('Ölçü çizgisinin tipi', s.dimLineType, (v) => edit({ dimLineType: v }))),
      ),
      h(
        'div',
        { class: 'io-row' },
        field('Uzatma çizgileri', colorSelect('Uzatma çizgilerinin rengi', s.extColor, (v) => edit({ extColor: v }))),
        field('Kalınlık', weightSelect('Uzatma çizgilerinin kalınlığı', s.extWeight, (v) => edit({ extWeight: v }))),
        field('Tip', typeSelect('Uzatma çizgilerinin tipi', s.extLineType, (v) => edit({ extLineType: v }))),
      ),
      // A third of the row, as each of the lines' fields (the desktop's too).
      h('div', { class: 'io-row' }, field('Değer', colorSelect('Değerin rengi', s.textColor, (v) => edit({ textColor: v }))), h('div', { class: 'io-field' }), h('div', { class: 'io-field' })),
    ];
  }

  /** A colour of a dimension's lines: none (the object's), a drawing colour, or the one it has. */
  function colorSelect(label: string, value: string | undefined, set: (v: string | undefined) => void): HTMLElement {
    const options = [{ value: '', label: 'Nesnenin rengi' }, ...LINE_COLORS.map((c) => ({ value: c.value, label: c.name }))];
    const known = value && options.find((o) => o.value.toLowerCase() === value.toLowerCase());
    if (value && !known) options.push({ value, label: value.toUpperCase() });
    return select(label, options, known ? known.value : (value ?? ''), (v) => set(v || undefined));
  }

  /** A weight of a dimension's lines on paper: none (a hairline) or one of the drawing's weights. */
  function weightSelect(label: string, value: number | undefined, set: (v: number | undefined) => void): HTMLElement {
    const options = [{ value: '', label: 'Kılcal' }, ...LINE_WEIGHTS.map((w) => ({ value: String(w), label: weightText(w) }))];
    if (value !== undefined && !LINE_WEIGHTS.includes(value)) options.push({ value: String(value), label: weightText(value) });
    return select(label, options, value === undefined ? '' : String(value), (v) => set(v === '' ? undefined : Number(v)));
  }

  /** A type of a dimension's lines: none (continuous) or another. */
  function typeSelect(label: string, value: LineType | undefined, set: (v: LineType | undefined) => void): HTMLElement {
    const options = LINE_TYPES.map((t) => ({ value: t, label: LINE_TYPE_LABEL[t] }));
    return select(label, options, value ?? 'continuous', (v) => set(v === 'continuous' ? undefined : (v as LineType)));
  }

  /** Standart's form: what a styleless object looks like, nothing to edit. */
  function standardForm(): HTMLElement[] {
    const font = drawingFontById(settings.drawingFont.value).label;
    return [
      h(
        'p',
        { class: 'astyle-standard' },
        kind === 'text'
          ? `Stilsiz yazılar projenin yazı tipiyle (${font}) yazılır; tek satırlı yazı eğiktir. Yazı tipi Proje ayarları'ndadır. Standart düzenlenmez: yeni bir stil için Yeni'ye basın.`
          : `Stilsiz ölçüler: uçlarda çentik, projenin ölçü yüksekliğinde (${fixed(settings.annotationMm('dimension'), 1)} mm) değer, nesnenin renginde kılcal çizgiler, projenin yazı tipi (${font}), birimi ve basamakları. Standart düzenlenmez: yeni bir stil için Yeni'ye basın.`,
      ),
    ];
  }

  function drawPreview(): void {
    const g = preview.getContext('2d');
    if (!g) return;
    const pal = readCanvasPalette();
    const dpr = window.devicePixelRatio || 1;
    const w = preview.clientWidth || 360;
    const hgt = preview.clientHeight || 120;
    preview.width = Math.round(w * dpr);
    preview.height = Math.round(hgt * dpr);
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, hgt);
    g.fillStyle = pal.paper;
    g.fillRect(0, 0, w, hgt);
    const s = current();
    if (kind === 'text') {
      const st = s as TextStyleDef | null;
      const face = st ? faceOf(st) : {};
      const px = 30;
      g.save();
      g.translate(16, hgt / 2 + px * 0.35);
      const lean = leanOf(face);
      if (lean) g.transform(1, 0, -lean, 1, 0, 0);
      g.scale(st?.widthFactor ?? 1, 1);
      g.font = faceFont(face, px, pal.drawingFont, 'italic 400');
      g.fillStyle = pal.label;
      g.textBaseline = 'alphabetic';
      g.fillText('Ada 104 · Parsel 12', 0, 0);
      g.restore();
      return;
    }
    // An aligned dimension 60 mm long on the paper, its line 12 mm over the points, at the project's scale.
    const st = s as DimensionStyleDef | null;
    const scale = settings.plotScale.value;
    const look = st ? lookOf(st) : {};
    const height = ((st?.height ?? settings.annotationMm('dimension')) / 1000) * scale;
    const len = (60 / 1000) * scale;
    const l = layoutDimension({ a: { x: 0, y: 0 }, b: { x: len, y: 0 }, offset: (12 / 1000) * scale, height, ...look });
    if (!l) return;
    const pts = [...l.lines.flat(), ...(l.fills ?? []).flat(), l.textAt];
    const [minX, maxX] = [Math.min(...pts.map((p) => p.x)), Math.max(...pts.map((p) => p.x))];
    const [minY, maxY] = [Math.min(...pts.map((p) => p.y)), Math.max(...pts.map((p) => p.y)) + height * 1.2];
    const k = Math.min((w - 32) / (maxX - minX || 1), (hgt - 24) / (maxY - minY || 1));
    const at = (p: { x: number; y: number }) => ({ x: 16 + (p.x - minX) * k + (w - 32 - (maxX - minX) * k) / 2, y: hgt - 12 - (p.y - minY) * k });
    // Its lines as its look names them (docs/adr/0205 §6): colour, weight (at the preview's paper size) and type.
    const lineLook = (color: string | undefined, weight: number | undefined, type: LineType | undefined) => {
      g.strokeStyle = color ?? pal.label;
      g.lineWidth = weight ? Math.max(1, (weight / 1000) * scale * k) : 1.2;
      const dash = { dashed: [3, 1.5], dashdot: [5, 1.2, 0.6, 1.2], dotted: [0.6, 1.2], continuous: [] }[type ?? 'continuous'];
      g.setLineDash(dash.map((mm) => Math.max(1, (mm / 1000) * scale * k)));
    };
    l.lines.forEach(([p, q], i) => {
      if (l.ext.includes(i)) lineLook(look.extColor, look.extWeight, look.extLineType);
      else lineLook(look.dimLineColor, look.dimLineWeight, look.dimLineType);
      const [a, b] = [at(p), at(q)];
      g.beginPath();
      g.moveTo(a.x, a.y);
      g.lineTo(b.x, b.y);
      g.stroke();
    });
    g.setLineDash([]);
    g.fillStyle = look.dimLineColor ?? pal.label;
    for (const ring of l.fills ?? []) {
      g.beginPath();
      ring.forEach((p, i) => (i ? g.lineTo(at(p).x, at(p).y) : g.moveTo(at(p).x, at(p).y)));
      g.closePath();
      g.fill();
    }
    const text = ctx.format.dimension(l, look);
    const t = at(l.textAt);
    const px = height * k;
    g.save();
    g.translate(t.x, t.y);
    g.rotate((-l.rotation * Math.PI) / 180);
    g.font = valueFont(look.font, px, pal.drawingFont);
    g.textAlign = 'center';
    g.textBaseline = 'alphabetic';
    if (look.textPlace === 'centre') {
      const tw = g.measureText(text).width;
      g.fillStyle = pal.paper;
      g.fillRect(-tw / 2 - px * 0.1, -px * 1.15, tw + px * 0.2, px * 1.38);
      g.fillStyle = pal.label;
    }
    g.fillStyle = look.textColor ?? pal.label;
    g.fillText(text, 0, 0);
    g.restore();
  }

  /** The list, the buttons and the problem line, after the draft changed (the form keeps its fields). */
  function refreshState(): void {
    replaceChildren(list, rowOf(null, STANDARD_STYLE), ...draft.map((s) => rowOf(s.id, s.name)));
    const p = tableProblem();
    replaceChildren(problem, ...(p ? [summaryLine('warn', `${p.charAt(0).toLocaleUpperCase('tr-TR')}${p.slice(1)}.`)] : []));
    problem.hidden = !p;
    remove.disabled = chosen === null;
    save.disabled = !!p || !changed();
    drawPreview();
  }

  function refresh(): void {
    if (chosen !== null && !draft.some((s) => s.id === chosen)) chosen = null;
    const s = current();
    replaceChildren(form, ...(s === null ? standardForm() : kind === 'text' ? textForm(s as TextStyleDef) : dimensionForm(s as DimensionStyleDef)));
    refreshState();
  }

  add.addEventListener('click', () => {
    const from = current();
    const base = from ? `${from.name} kopyası` : kind === 'text' ? 'Yazı stili' : 'Ölçü stili';
    const fresh: Style =
      kind === 'text'
        ? { ...(from as TextStyleDef | null ?? { font: settings.drawingFont.value }), id: uuidv7(), name: freeName(base, draft) } as TextStyleDef
        : { ...(from as DimensionStyleDef | null ?? { height: 2.5 }), id: uuidv7(), name: freeName(base, draft) } as DimensionStyleDef;
    delete (fresh as { fontFile?: string }).fontFile;
    draft = [...draft, fresh];
    chosen = fresh.id;
    refresh();
  });
  remove.addEventListener('click', async () => {
    const s = current();
    if (!s) return;
    const n = usage.get(s.id) ?? 0;
    if (n > 0) {
      const what = kind === 'text' ? 'yazı' : 'ölçü';
      const ok = await askRemove({
        title: 'Stili sil',
        message: `${n} ${what} “${s.name}” stilini kullanıyor. Kaydedince görünüşleri kalır, stilsiz olurlar.`,
        action: 'Sil',
      });
      if (!ok) return;
    }
    draft = draft.filter((x) => x.id !== s.id);
    chosen = null;
    refresh();
  });

  const dialog = new Dialog({
    title: kind === 'text' ? TEXT_STYLES_TITLE : DIMENSION_STYLES_TITLE,
    width: 760,
    className: 'dialog--io dialog--annotation',
    content: [
      problem,
      h(
        'div',
        { class: 'astyle-body' },
        h('div', { class: 'astyle-side' }, field(kind === 'text' ? 'Yazı stilleri' : 'Ölçü stilleri', list), h('div', { class: 'io-row states-actions' }, add, remove)),
        h('div', { class: 'astyle-main' }, form, field('Önizleme', preview)),
      ),
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
    beforeClose: () => {
      if (!changed()) return true;
      void askUnsaved({ name: dialog ? (kind === 'text' ? TEXT_STYLES_TITLE : DIMENSION_STYLES_TITLE) : '', after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat', canSave: !tableProblem() }).then((a) => {
        if (a === 'save') commit();
        else if (a === 'discard') {
          draft = structuredClone(saved());
          dialog.close();
        }
      });
      return false;
    },
    onClose: () => d.dispose(),
  });

  function commit(): void {
    if (tableProblem()) return;
    const out = kind === 'text' ? saveTextStyles(ctx, draft as TextStyleDef[]) : saveDimensionStyles(ctx, draft as DimensionStyleDef[]);
    const what = kind === 'text' ? 'yazı' : 'ölçü';
    ctx.log.success(`${kind === 'text' ? 'Yazı' : 'Ölçü'} stilleri kaydedildi${out.changed ? `; ${out.changed} ${what} stiline uydu` : ''}.`);
    if (out.locked) ctx.log.warn(`${out.locked} ${what} kilitli katmanda; eski görünüşüyle kaldı. Kilidi açıp stili yeniden kaydedin.`);
    draft = structuredClone(saved());
    dialog.close();
  }

  save.addEventListener('click', commit);
  cancel.addEventListener('click', () => dialog.request());
  d.add(settings.drawingFont.subscribe(() => refresh()));
  refresh();
}
