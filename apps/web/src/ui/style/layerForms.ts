import type { Anchor, FillLayer, LineLayer, LineWave, MarkerLayer, MarkerPlacement, ShapeName, SizeUnit, TextMarker } from '../../model/style';
import { h, type Child } from '../dom';
import { checkbox, colorInput, dashInput, dataDefined, numberInput, pair, row, select, textInput, type FieldEnv } from './designerFields';

/**
 * What each symbol layer type is called, how a new one starts, a one-line
 * summary for the layer list, and its property form. Forms report patches;
 * the designer applies them to its draft.
 */

export type AnyLayer = FillLayer | LineLayer | MarkerLayer;
export type LayerType = AnyLayer['type'];

export const LAYER_LABEL: Record<LayerType, string> = {
  simpleFill: 'Dolgu',
  hatchFill: 'Tarama',
  patternFill: 'Desen',
  imageFill: 'Görüntü dolgusu',
  centroidMarker: 'İç noktada işaret',
  simpleLine: 'Çizgi',
  markerLine: 'Çizgi boyunca işaret',
  shape: 'Şekil',
  svg: 'SVG çizimi',
  text: 'Yazı',
  raster: 'Görüntü',
};

export const LAYER_TYPES: Record<'fill' | 'line' | 'marker', LayerType[]> = {
  fill: ['simpleFill', 'hatchFill', 'patternFill', 'imageFill', 'simpleLine', 'markerLine', 'centroidMarker'],
  line: ['simpleLine', 'markerLine'],
  marker: ['shape', 'text', 'svg', 'raster'],
};

/** Layers that hold a marker symbol of their own (edited as child rows). */
export const hasMarker = (l: AnyLayer): l is Extract<AnyLayer, { marker: unknown }> => l.type === 'markerLine' || l.type === 'patternFill' || l.type === 'centroidMarker';

const dot = (id: string): MarkerLayer => ({ id, type: 'shape', shape: 'circle', size: 1, fill: 'ink' });

export function newLayer(type: LayerType, id: string, context: 'fill' | 'line' | 'marker'): AnyLayer {
  switch (type) {
    case 'simpleFill':
      return { id, type, color: '#C9D6E3' };
    case 'hatchFill':
      return { id, type, angle: 45, spacing: 2, width: 0.2, color: 'ink' };
    case 'patternFill':
      return { id, type, spacingX: 3, spacingY: 3, marker: { type: 'marker', layers: [dot('0')] } };
    case 'imageFill':
      return { id, type, asset: '', tileSize: 5 };
    case 'centroidMarker':
      return { id, type, marker: { type: 'marker', layers: [{ id: '0', type: 'text', text: { expr: 'etiket', fallback: 'A' }, size: 3, font: 'sans', weight: 700, color: 'ink' }] } };
    case 'simpleLine':
      return { id, type, color: 'ink', width: context === 'fill' ? 0.25 : 0.35 };
    case 'markerLine':
      return { id, type, placement: 'interval', interval: 6, offsetAlong: 3, rotate: true, marker: { type: 'marker', layers: [dot('0')] } };
    case 'shape':
      return { id, type, shape: 'circle', size: 3, fill: 'ink', stroke: null, strokeWidth: 0.2 };
    case 'svg':
      return { id, type, asset: '', size: 5, fill: 'ink' };
    case 'text':
      return { id, type, text: 'A', size: 3, font: 'sans', weight: 700, color: 'ink' };
    case 'raster':
      return { id, type, asset: '', size: 5 };
  }
}

const num = (v: unknown) => (typeof v === 'number' ? String(Math.round(v * 100) / 100) : 'ƒ');
const unitOf = (l: AnyLayer) => (l.unit === 'px' ? 'px' : l.unit === 'm' ? 'm' : 'mm');

export function summary(l: AnyLayer): string {
  const u = unitOf(l);
  switch (l.type) {
    case 'simpleFill':
      return typeof l.color === 'string' ? l.color : 'ifadeden renk';
    case 'hatchFill':
      return `${l.angle}° · ${num(l.spacing)} ${u} aralık`;
    case 'patternFill':
      return `${num(l.spacingX)} × ${num(l.spacingY)} ${u}${l.stagger ? ', şaşırtmalı' : ''}${l.jitter ? ', dağınık' : ''}`;
    case 'imageFill':
      return l.asset ? `${num(l.tileSize)} ${u} döşeme` : 'çizim seçilmedi';
    case 'centroidMarker':
      return l.position === 'centroid' ? 'ağırlık merkezinde' : 'alanın içinde';
    case 'simpleLine':
      return `${num(l.width)} ${u}${l.dash?.length ? ', kesikli' : ''}${l.offset ? (typeof l.offset === 'number' ? `, ${num(l.offset)} ${u} kaydırılmış` : ', ifadeyle kaydırılmış') : ''}${l.wave ? ', dalgalı' : ''}`;
    case 'markerLine':
      return l.placement === 'interval' ? `her ${num(l.interval)} ${u}${l.group && l.group.count > 1 ? `, ${l.group.count}'li` : ''}` : (PLACEMENTS.find((p) => p.value === l.placement)?.label ?? '');
    case 'shape':
      return `${SHAPES.find((s) => s.value === l.shape)?.label ?? l.shape} · ${num(l.size)} ${u}`;
    case 'svg':
    case 'raster':
      return l.asset ? `${num(l.size)} ${u}` : 'çizim seçilmedi';
    case 'text':
      return typeof l.text === 'string' ? `“${l.text}”` : 'öznitelikten';
  }
}

// ── Option lists ───────────────────────────────────────────────────────

const UNITS: { value: SizeUnit; label: string }[] = [
  { value: 'mm', label: 'Kâğıt mm' },
  { value: 'px', label: 'Ekran px' },
  { value: 'm', label: 'Harita m' },
];

export const SHAPES: { value: ShapeName; label: string }[] = [
  { value: 'circle', label: 'Daire' },
  { value: 'ring', label: 'Halka (noktalı)' },
  { value: 'square', label: 'Kare' },
  { value: 'rectangle', label: 'Dikdörtgen' },
  { value: 'diamond', label: 'Baklava' },
  { value: 'triangle', label: 'Üçgen' },
  { value: 'pentagon', label: 'Beşgen' },
  { value: 'hexagon', label: 'Altıgen' },
  { value: 'octagon', label: 'Sekizgen' },
  { value: 'star', label: 'Yıldız' },
  { value: 'cross', label: 'Artı' },
  { value: 'x', label: 'Çarpı' },
  { value: 'line', label: 'Çizgi' },
  { value: 'arrow', label: 'Ok' },
  { value: 'arrowhead', label: 'Ok ucu' },
  { value: 'chevron', label: 'Açık ok ucu (V)' },
  { value: 'semicircle', label: 'Yarım daire' },
  { value: 'quartercircle', label: 'Çeyrek daire' },
  { value: 'gear', label: 'Dişli' },
  { value: 'arc', label: 'Yay (açık)' },
];

/** Shapes drawn as lines only: they have no fill and no hole. */
const OPEN_SHAPE_NAMES = new Set<ShapeName>(['cross', 'x', 'line', 'arrow', 'chevron', 'arc']);

const PLACEMENTS: { value: MarkerPlacement; label: string }[] = [
  { value: 'interval', label: 'Aralıklı' },
  { value: 'vertex', label: 'Her köşede' },
  { value: 'innerVertex', label: 'İç köşelerde' },
  { value: 'first', label: 'Başta' },
  { value: 'last', label: 'Sonda' },
  { value: 'center', label: 'Ortada' },
  { value: 'segmentCenter', label: 'Kenar ortalarında' },
];

const ANCHORS: { value: Anchor; label: string }[] = [
  { value: 'center', label: 'Orta' },
  { value: 'top', label: 'Üst' },
  { value: 'bottom', label: 'Alt' },
  { value: 'left', label: 'Sol' },
  { value: 'right', label: 'Sağ' },
  { value: 'top-left', label: 'Sol üst' },
  { value: 'top-right', label: 'Sağ üst' },
  { value: 'bottom-left', label: 'Sol alt' },
  { value: 'bottom-right', label: 'Sağ alt' },
];

const FONTS: { value: NonNullable<TextMarker['font']>; label: string }[] = [
  { value: 'sans', label: 'Arial' },
  { value: 'narrow', label: 'Arial Narrow' },
  { value: 'serif', label: 'Times' },
  { value: 'ui', label: 'Arayüz yazısı' },
  { value: 'mono', label: 'Eş aralıklı' },
];

const WEIGHTS: { value: string; label: string }[] = [
  { value: '400', label: 'Normal' },
  { value: '500', label: 'Orta' },
  { value: '600', label: 'Yarı kalın' },
  { value: '700', label: 'Kalın' },
  { value: '900', label: 'Siyah (Arial Black)' },
];

// ── Forms ──────────────────────────────────────────────────────────────

export interface FormEnv extends FieldEnv {
  /** Assets the symbol may draw with: SVG and raster drawings of the library. */
  assets: readonly { id: string; name: string; format: string }[];
  /** Adds an SVG file or a PNG/JPEG picture to the user's library; the new asset id, or null. */
  importSvg(): Promise<string | null>;
  /** Opens the SVG editor on a drawing (or a new one); the saved asset id, or null. */
  drawSvg(id?: string): Promise<string | null>;
  /** Where the layer sits: an area symbol's line layers may choose rings. */
  context: 'fill' | 'line' | 'marker';
}

type Patch = Partial<AnyLayer> & Record<string, unknown>;

export function layerForm(l: AnyLayer, set: (patch: Patch) => void, env: FormEnv): HTMLElement {
  const u = unitOf(l);
  const n = (label: string, value: number, key: string, opts: { step?: number; min?: number; max?: number; unit?: string | null } = {}) =>
    row(label, numberInput(value, (v) => set({ [key]: v }), { label, unit: opts.unit === null ? undefined : (opts.unit ?? u), step: opts.step, min: opts.min, max: opts.max }));
  const ddNum = (label: string, value: number | { expr: string; fallback?: number } | undefined, key: string, fallback: number, opts: { min?: number } = {}) =>
    row(label, dataDefined(value, fallback, (v) => set({ [key]: v }), (v, s) => numberInput(v, s, { label, unit: u, min: opts.min }), label));
  const ddColor = (label: string, value: string | null | { expr: string; fallback?: string } | undefined, key: string, allowNone = false) =>
    row(label, dataDefined<string | null>(value ?? null, null, (v) => set({ [key]: v }), (v, s) => colorInput(v, s, env, { label, allowNone }), label));
  const common: Child[] = [
    row('Birim', select(l.unit ?? 'mm', UNITS, (v) => set({ unit: v }), 'Birim'), 'Kâğıt mm çizim ölçeğiyle büyür; ekran px sabit kalır; harita m gerçek boydur.'),
    row('Saydamlık', numberInput(Math.round((1 - (l.opacity ?? 1)) * 100), (v) => set({ opacity: 1 - Math.min(100, Math.max(0, v)) / 100 }), { label: 'Saydamlık', unit: '%', step: 5, min: 0, max: 100 })),
    row('Görünür', dataDefined(l.enabled ?? true, true, (v) => set({ enabled: v }), (v, s) => checkbox(v, s, 'Çizilsin'), 'Görünür'), 'ƒ ile koşula bağlanabilir: ör. [Nitelik] = \'Arsa\''),
  ];
  const assetPick = (value: string, key: string) => {
    const options = [{ value: '', label: 'Çizim seçin…' }, ...env.assets.map((a) => ({ value: a.id, label: a.name }))];
    const sel = select(value, options, (v) => set({ [key]: v }), 'Çizim');
    const btn = (label: string, title: string, run: () => void) => {
      const b = h('button', { class: 'btn btn--small', type: 'button', title }, label);
      b.addEventListener('click', run);
      return b;
    };
    const current = env.assets.find((a) => a.id === value);
    return row(
      'Çizim',
      h(
        'div',
        { class: 'sdf__assetcol' },
        sel,
        h(
          'div',
          { class: 'sdf__assetrow' },
          btn('Yeni çizim…', 'SVG çizim düzenleyicisinde yeni bir çizim yapar', () => void env.drawSvg().then((id) => id && set({ [key]: id }))),
          current?.format === 'svg' ? btn('Düzenle…', 'Seçili çizimi düzenleyicide açar (sistem çiziminin kopyası)', () => void env.drawSvg(value).then((id) => id && set({ [key]: id }))) : null,
          btn('Dosya al…', 'Bilgisayardan SVG, PNG ya da JPEG alır (Kitaplığım\'a eklenir)', () => void env.importSvg().then((id) => id && set({ [key]: id }))),
        ),
      ),
    );
  };
  const specific: Child[] = [];
  switch (l.type) {
    case 'simpleFill':
      specific.push(ddColor('Renk', l.color, 'color'));
      break;
    case 'hatchFill':
      specific.push(
        ddColor('Renk', l.color, 'color'),
        n('Açı', l.angle, 'angle', { unit: '°', step: 5 }),
        pair(n('Aralık', l.spacing, 'spacing', { min: 0.01 }), n('Kalınlık', l.width, 'width', { min: 0 })),
        n('Kaydırma', l.offset ?? 0, 'offset'),
        row('Kesik', dashInput(l.dash, (v) => set({ dash: v }), 'Kesik'), 'Çizgi ve boşluk uzunlukları sırayla.'),
        l.dash?.length ? n('Kesik kaydırma', l.dashOffset ?? 0, 'dashOffset') : null,
      );
      break;
    case 'patternFill':
      specific.push(
        pair(n('Aralık Y (sağa)', l.spacingX, 'spacingX', { min: 0.01 }), n('Aralık X (yukarı)', l.spacingY, 'spacingY', { min: 0.01 })),
        row('Dizilim', checkbox(!!l.stagger, (v) => set({ stagger: v }), 'Şaşırtmalı (her iki satırda bir yarım kaydır)')),
        n('Açı', l.angle ?? 0, 'angle', { unit: '°', step: 5 }),
        pair(n('Kaydırma Y', l.offset?.[0] ?? 0, 'offsetX'), n('Kaydırma X', l.offset?.[1] ?? 0, 'offsetY')),
        pair(n('Dağınıklık', Math.round((l.jitter ?? 0) * 100), 'jitterPct', { unit: '%', min: 0, max: 100, step: 5 }), n('Doluluk', Math.round((l.coverage ?? 1) * 100), 'coveragePct', { unit: '%', min: 0, max: 100, step: 5 })),
        n('Rastgele tohum', l.seed ?? 0, 'seed', { unit: null, step: 1 }),
        h('div', { class: 'sdf__hint' }, 'Dağınıklık ve doluluk yalnızca şekil işaretlerinde uygulanır (kumsal, serbest noktalama).'),
      );
      break;
    case 'imageFill':
      specific.push(assetPick(l.asset, 'asset'), n('Döşeme genişliği', l.tileSize, 'tileSize', { min: 0.01 }), n('Açı', l.angle ?? 0, 'angle', { unit: '°', step: 5 }));
      break;
    case 'centroidMarker':
      specific.push(row('Konum', select(l.position ?? 'pointOnSurface', [{ value: 'pointOnSurface', label: 'Alanın içinde (her zaman)' }, { value: 'centroid', label: 'Ağırlık merkezi' }], (v) => set({ position: v }), 'Konum')));
      break;
    case 'simpleLine':
      specific.push(
        ddColor('Renk', l.color, 'color'),
        ddNum('Kalınlık', l.width, 'width', 0.25, { min: 0 }),
        row('Kesik', dashInput(l.dash, (v) => set({ dash: v }), 'Kesik'), 'Çizgi ve boşluk uzunlukları sırayla, en çok 8 değer.'),
        l.dash?.length ? n('Kesik kaydırma', l.dashOffset ?? 0, 'dashOffset') : null,
        pair(
          row('Uç', select(l.cap ?? 'butt', [{ value: 'butt', label: 'Düz' }, { value: 'round', label: 'Yuvarlak' }, { value: 'square', label: 'Kare' }], (v) => set({ cap: v }), 'Uç')),
          ddNum('Kaydırma', l.offset, 'offset', 0),
        ),
        h('div', { class: 'sdf__hint' }, env.context === 'fill' ? 'Artı kaydırma çizgiyi alanın içine alır.' : 'Artı kaydırma çizim yönünün soluna alır.'),
        env.context === 'fill' ? row('Halkalar', select(l.rings ?? 'all', [{ value: 'all', label: 'Hepsi' }, { value: 'exterior', label: 'Yalnızca dış sınır' }, { value: 'interior', label: 'Yalnızca adalar' }], (v) => set({ rings: v }), 'Halkalar')) : null,
        waveForm(l.wave, (w) => set({ wave: w }), u),
        pair(n('Yumuşatma', l.blur ?? 0, 'blur', { min: 0 }), pair(n('Gölge sağa', l.shift?.[0] ?? 0, 'shiftX'), n('Gölge yukarı', l.shift?.[1] ?? 0, 'shiftY'))),
        h('div', { class: 'sdf__hint' }, 'Yumuşatma kenarı bu genişlikte soldurur; gölge kaydırması çizgiyi sayfada hep aynı yöne taşır (alt gölge için sağa ve aşağı).'),
      );
      break;
    case 'markerLine':
      specific.push(
        row('Yerleşim', select(l.placement, PLACEMENTS, (v) => set({ placement: v }), 'Yerleşim')),
        l.placement === 'interval' ? pair(n('Aralık', l.interval ?? 6, 'interval', { min: 0.01 }), n('İlk uzaklık', l.offsetAlong ?? 0, 'offsetAlong')) : null,
        pair(ddNum('Kaydırma', l.offset, 'offset', 0), row('Döndür', checkbox(l.rotate !== false, (v) => set({ rotate: v }), 'Çizgiyle dönsün'))),
        pair(n('Grup', l.group?.count ?? 1, 'groupCount', { unit: 'adet', min: 1, max: 20, step: 1 }), n('Grup aralığı', l.group?.spacing ?? 1, 'groupSpacing', { min: 0 })),
        env.context === 'fill' ? row('Halkalar', select(l.rings ?? 'all', [{ value: 'all', label: 'Hepsi' }, { value: 'exterior', label: 'Yalnızca dış sınır' }, { value: 'interior', label: 'Yalnızca adalar' }], (v) => set({ rings: v }), 'Halkalar')) : null,
      );
      break;
    case 'shape':
      specific.push(
        row('Şekil', select(l.shape, SHAPES, (v) => set({ shape: v }), 'Şekil')),
        l.shape === 'rectangle' ? pair(ddNum('Genişlik', l.size, 'size', 3, { min: 0 }), n('Yükseklik', l.height ?? (typeof l.size === 'number' ? l.size : 3), 'height', { min: 0 })) : ddNum('Boyut', l.size, 'size', 3, { min: 0 }),
        ddColor('Dolgu', l.fill, 'fill', true),
        ddColor('Çizgi', l.stroke, 'stroke', true),
        n('Çizgi kalınlığı', l.strokeWidth ?? 0.2, 'strokeWidth', { min: 0 }),
        l.shape === 'gear'
          ? pair(n('Diş sayısı', l.teeth ?? 12, 'teeth', { unit: 'adet', min: 3, max: 64, step: 1 }), n('Diş derinliği', Math.round((l.teethDepth ?? 0.2) * 100), 'teethDepthPct', { unit: '%', min: 2, max: 60, step: 1 }))
          : null,
        l.shape === 'arc' ? n('Açıklık', l.sweep ?? 180, 'sweep', { unit: '°', min: 1, max: 360, step: 5 }) : null,
        !OPEN_SHAPE_NAMES.has(l.shape) ? n('Delik', Math.round((l.hole ?? 0) * 100), 'holePct', { unit: '%', min: 0, max: 95, step: 5 }) : null,
        !OPEN_SHAPE_NAMES.has(l.shape) ? h('div', { class: 'sdf__hint' }, 'Delik, yarıçapın yüzdesi kadar ortadan yuvarlak boşluk bırakır (dişli göbeği, pul).') : null,
        ...placementRows(l, set, u),
      );
      break;
    case 'svg':
      specific.push(assetPick(l.asset, 'asset'), ddNum('Genişlik', l.size, 'size', 5, { min: 0 }), ddColor('Renk (currentColor)', l.fill, 'fill', true), ddColor('İkinci renk', l.stroke, 'stroke', true), ...placementRows(l, set, u));
      break;
    case 'raster':
      specific.push(assetPick(l.asset, 'asset'), ddNum('Genişlik', l.size, 'size', 5, { min: 0 }), ...placementRows(l, set, u));
      break;
    case 'text':
      specific.push(
        row('Metin', dataDefined(l.text, '', (v) => set({ text: v }), (v, s) => textInput(v, s, { label: 'Metin' }), 'Metin'), 'ƒ ile öznitelikten: ör. \'E=\' || [Emsal]'),
        ddNum('Harf yüksekliği', l.size, 'size', 3, { min: 0 }),
        pair(row('Yazı tipi', select(l.font ?? 'ui', FONTS, (v) => set({ font: v }), 'Yazı tipi')), row('Kalınlık', select(String(l.weight ?? 400), WEIGHTS, (v) => set({ weight: Number(v) as TextMarker['weight'] }), 'Kalınlık'))),
        row('Stil', checkbox(!!l.italic, (v) => set({ italic: v }), 'İtalik')),
        ddColor('Renk', l.color, 'color'),
        row('Hale', colorInput(l.halo?.color ?? null, (c) => set({ halo: c ? { color: c, width: l.halo?.width ?? 0.3 } : null }), env, { label: 'Hale rengi', allowNone: true })),
        l.halo ? n('Hale kalınlığı', l.halo.width, 'haloWidth', { min: 0 }) : null,
        ...placementRows(l, set, u),
      );
      break;
  }
  return h('div', { class: 'sdf__form' }, h('div', { class: 'sdf__group' }, specific), h('div', { class: 'sdf__group sdf__group--common' }, h('div', { class: 'sdf__grouptitle' }, 'Genel'), common));
}

function placementRows(l: MarkerLayer, set: (p: Patch) => void, u: string): Child[] {
  return [
    row(
      'Döndürme',
      dataDefined(l.rotation ?? 0, 0, (v) => set({ rotation: v }), (v, s) => numberInput(v, s, { label: 'Döndürme', unit: '°', step: 5 }), 'Döndürme'),
      'Saat yönünün tersine; çizgi boyunca işaretlerde çizginin yönüne eklenir.',
    ),
    pair(
      row('Kaydırma Y', numberInput(l.offset?.[0] ?? 0, (v) => set({ offsetX: v }), { label: 'Kaydırma Y', unit: u })),
      row('Kaydırma X', numberInput(l.offset?.[1] ?? 0, (v) => set({ offsetY: v }), { label: 'Kaydırma X', unit: u })),
    ),
    row('Çapa', select(l.anchor ?? 'center', ANCHORS, (v) => set({ anchor: v }), 'Çapa'), 'Noktanın işaretin neresine düştüğü.'),
  ];
}

function waveForm(w: LineWave | undefined, set: (w: LineWave | undefined) => void, u: string): HTMLElement {
  const host = h('div', { class: 'sdf__wave' });
  const render = (cur: LineWave | undefined) => {
    const shape = select(cur?.shape ?? 'none', [{ value: 'none', label: 'Düz çizgi' }, { value: 'sine', label: 'Dalga (sinüs)' }, { value: 'zigzag', label: 'Zikzak' }, { value: 'square', label: 'Kare dalga' }] as { value: string; label: string }[], (v) => {
      const next = v === 'none' ? undefined : { shape: v as LineWave['shape'], length: cur?.length ?? 5, amplitude: cur?.amplitude ?? 0.8, spacing: cur?.spacing, connect: cur?.connect ?? true };
      set(next);
      render(next);
    }, 'Dalga');
    const parts: Child[] = [row('Biçim', shape)];
    if (cur) {
      const upd = (patch: Partial<LineWave>) => {
        cur = { ...cur!, ...patch };
        set(cur);
      };
      parts.push(
        pair(row('Dalga boyu', numberInput(cur.length, (v) => upd({ length: v }), { label: 'Dalga boyu', unit: u, min: 0.01 })), row('Genlik', numberInput(cur.amplitude, (v) => upd({ amplitude: v }), { label: 'Genlik', unit: u, min: 0 }))),
        pair(row('Tekrar', numberInput(cur.spacing ?? cur.length, (v) => upd({ spacing: v }), { label: 'Tekrar', unit: u, min: 0.01 })), row('Aralar', checkbox(cur.connect !== false, (v) => upd({ connect: v }), 'Düz çizgiyle bağla'))),
        pair(
          row('Evre', checkbox(cur.offsetAlong !== undefined, (v) => (upd({ offsetAlong: v ? 0 : undefined }), render(cur)), 'Çizgi başından')),
          cur.offsetAlong !== undefined ? row('İlk dalga', numberInput(cur.offsetAlong, (v) => upd({ offsetAlong: v }), { label: 'İlk dalga', unit: u, min: 0 })) : null,
        ),
        h('div', { class: 'sdf__hint' }, 'Çizgi başından başlayan dalgalar, aynı aralık ve ilk uzaklıkla yerleşen işaretlerle adım adım gider; kapalıyken dalgalar çizgiye ortalanır.'),
      );
    }
    host.replaceChildren(h('div', { class: 'sdf__grouptitle' }, 'Dalga'), ...parts.flat().filter(Boolean) as Node[]);
  };
  render(w);
  return host;
}

/** Turns a form patch (with the helper keys offsetX, jitterPct …) into a layer. */
export function applyPatch(l: AnyLayer, patch: Patch): AnyLayer {
  const p: Record<string, unknown> = { ...patch };
  const cur = l as unknown as Record<string, unknown>;
  const offset = (cur.offset as readonly [number, number] | undefined) ?? [0, 0];
  if ('shiftX' in p || 'shiftY' in p) {
    const cs = (cur.shift as readonly [number, number] | undefined) ?? [0, 0];
    const next: [number, number] = [Number(p.shiftX ?? cs[0]), Number(p.shiftY ?? cs[1])];
    p.shift = next[0] || next[1] ? next : undefined;
    delete p.shiftX;
    delete p.shiftY;
  }
  if ('offsetX' in p || 'offsetY' in p) {
    p.offset = [p.offsetX ?? offset[0], p.offsetY ?? offset[1]];
    delete p.offsetX;
    delete p.offsetY;
  }
  for (const [from, to] of [['holePct', 'hole'], ['teethDepthPct', 'teethDepth']] as const)
    if (from in p) {
      p[to] = Number(p[from]) / 100;
      delete p[from];
    }
  if ('jitterPct' in p) {
    p.jitter = Number(p.jitterPct) / 100;
    delete p.jitterPct;
  }
  if ('coveragePct' in p) {
    p.coverage = Number(p.coveragePct) / 100;
    delete p.coveragePct;
  }
  if ('groupCount' in p || 'groupSpacing' in p) {
    const g = (cur.group as { count: number; spacing: number } | undefined) ?? { count: 1, spacing: 1 };
    const count = Math.max(1, Math.round(Number(p.groupCount ?? g.count)));
    p.group = count > 1 ? { count, spacing: Number(p.groupSpacing ?? g.spacing) } : undefined;
    delete p.groupCount;
    delete p.groupSpacing;
  }
  if ('haloWidth' in p) {
    const halo = cur.halo as { color: string; width: number } | null | undefined;
    p.halo = halo ? { ...halo, width: Number(p.haloWidth) } : halo;
    delete p.haloWidth;
  }
  if (p.dash === null) p.dash = undefined;
  return { ...l, ...p } as AnyLayer;
}
