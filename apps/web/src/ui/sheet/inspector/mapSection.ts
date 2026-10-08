import type { ItemKind } from '../../../contracts/generated/sheet/ItemKind';
import type { MapGrid } from '../../../contracts/generated/sheet/MapGrid';
import { fixed } from '../../../core/displayNumber';
import { h, type Child } from '../../dom';
import { icon } from '../../icons';
import { segmented, toggleSwitch } from '../../widgets/controls';
import { tooltip } from '../../widgets/tooltip';
import { flag, kinds, labelled, mmField, numField, pair, patchEach, patchKind, patchKindEach, pick, shared, type SectionCtx } from './parts';

/**
 * A map frame's section (docs/sheet/design.md §3.1, §11): its scale from the
 * engine's standard list or typed, “Çizim ölçeğini al” taking the project's
 * plot scale and “Görünüme sığdır” the scale and centre that show the drawing
 * area's view in the frame (the smallest standard scale that holds it), its
 * centre (Y east, X north, metres) with “Görünümden al” taking the drawing
 * area's centre, its own turn, the layers it shows (all, or some), its grids
 * (karelaj) and the band their labels are written in, and an overview's map.
 * Several maps share a value or show “—”.
 */

type MapKind = Extract<ItemKind, { type: 'map' }>;

export interface MapHooks {
  /** The drawing area's centre (ground metres): Görünümden al. */
  viewCentre(): { x: number; y: number };
  /** How much ground the drawing area shows (metres): Görünüme sığdır. */
  viewSize(): { width: number; height: number };
  /** The project's plot scale's denominator: Çizim ölçeğini al. */
  plotScale(): number;
  /** The layers of the drawing, top first: their ids and names. */
  layers(): { id: string; name: string }[];
}

export function mapSection(c: SectionCtx, hooks: MapHooks): Child[] {
  const v = <V>(of: (k: MapKind) => V) => shared(c, 'map', of);
  const fixedView = v((k) => k.view.type) === 'fixed';
  const scale = v((k) => (k.view.type === 'fixed' ? k.view.scale : null));
  const standards = c.host.engine()?.standardScales() ?? [];
  const options = [...new Set([...standards, ...(scale && !standards.includes(scale) ? [scale] : [])])].sort((a, b) => a - b).map((s) => ({ value: s, label: `1/${s}`, detail: standards.includes(s) ? undefined : 'standart değil' }));
  const centre = v((k) => (k.view.type === 'fixed' ? (k.view.center ?? null) : null));
  const layers = v((k) => k.layers);
  const takeView = h('button', { class: 'btn btn--small', type: 'button', disabled: c.readOnly !== null || !fixedView }, icon('target', 14), 'Görünümden al');
  takeView.addEventListener('click', () => patchKind(c, 'Harita merkezi: görünümden', { view: { center: hooks.viewCentre() } }));
  c.d.add(tooltip(takeView, () => ({ title: 'Görünümden al', description: 'Haritanın merkezi çizim alanının şu anki merkezi olur; ölçek değişmez.', note: c.readOnly ?? undefined })));
  // The project's plot scale, and the scale and centre that show the drawing area's view (each map by its own frame and turn).
  const plot = Math.max(1, Math.round(hooks.plotScale()));
  const takeScale = h('button', { class: 'btn btn--small', type: 'button', disabled: c.readOnly !== null || !fixedView || scale === plot }, icon('plotScale', 14), 'Çizim ölçeğini al');
  takeScale.addEventListener('click', () => patchKind(c, 'Harita ölçeği: çizimden', { view: { scale: plot } }));
  c.d.add(tooltip(takeScale, () => ({ title: 'Çizim ölçeğini al', description: `Haritanın ölçeği projenin çizim ölçeği (1/${plot}) olur; merkez değişmez. Ölçek elle de yazılabilir.`, note: c.readOnly ?? (scale === plot ? 'Harita zaten çizim ölçeğinde.' : undefined) })));
  const engine = c.host.engine();
  const fit = h('button', { class: 'btn btn--small', type: 'button', disabled: c.readOnly !== null || !fixedView || !engine }, icon('zoomExtents', 14), 'Görünüme sığdır');
  fit.addEventListener('click', () => {
    const view = hooks.viewSize();
    const center = hooks.viewCentre();
    patchEach(c, 'Harita: görünüme sığdır', (item) => {
      const k = item.kind as MapKind;
      const s = engine?.fitViewScale(view.width, view.height, k.view.type === 'fixed' ? k.view.rotation : 0, item.frame) ?? null;
      return { kind: { view: s === null ? { center } : { center, scale: s } } };
    });
  });
  c.d.add(tooltip(fit, () => ({ title: 'Görünüme sığdır', description: 'Harita çizim alanında şu an görüneni gösterir: merkezi görünümün merkezi, ölçeği onu çerçeveye sığdıran en büyük standart ölçek olur.', note: c.readOnly ?? undefined })));
  const out: Child[] = [];
  if (!fixedView) out.push(h('p', { class: 'sheet-insp__hint' }, 'Atlas haritası: yeri ve ölçeği her atlas sayfasının nesnesinden gelir.'));
  else {
    out.push(
      pair(
        pick(c, 'Ölçek', 'map.scale', scale, options, (s) => patchKind(c, 'Harita ölçeği', { view: { scale: s } })),
        numField(c, 'Elle (1/…)', 'map.scaleTyped', scale, (s) => patchKind(c, 'Harita ölçeği', { view: { scale: Math.max(1, Math.round(s)) } }), { min: 1, max: 10_000_000 }),
      ),
      pair(
        numField(c, 'Merkez Y (doğu)', 'map.center.x', centre ? centre.x : null, (x) => patchKindEach(c, 'map', 'Harita merkezi', (k) => ({ view: { center: { x, y: k.view.type === 'fixed' ? (k.view.center?.y ?? hooks.viewCentre().y) : 0 } } })), { decimals: 2, unit: 'm' }),
        numField(c, 'Merkez X (kuzey)', 'map.center.y', centre ? centre.y : null, (y) => patchKindEach(c, 'map', 'Harita merkezi', (k) => ({ view: { center: { x: k.view.type === 'fixed' ? (k.view.center?.x ?? hooks.viewCentre().x) : 0, y } } })), { decimals: 2, unit: 'm' }),
      ),
      h('div', { class: 'sheet-insp__actions' }, takeScale, fit, takeView),
      numField(c, 'Harita dönüşü', 'map.rotation', v((k) => (k.view.type === 'fixed' ? k.view.rotation / 1000 : null)), (deg) => patchKind(c, 'Harita dönüşü', { view: { rotation: Math.round((((deg % 360) + 360) % 360) * 1000) } }), { decimals: 1, unit: '°' }),
    );
    if (!centre && v((k) => (k.view.type === 'fixed' ? !k.view.center : false))) out.push(h('p', { class: 'sheet-insp__hint' }, 'Haritanın yeri seçilmedi: “Görünümden al” ile çizim alanının merkezini verin.'));
  }
  // Layers: all of them, or a list.
  const list = layers?.type === 'list' ? layers.layers : null;
  out.push(
    labelled(
      'Katmanlar',
      segmented<'all' | 'list'>({
        label: 'Katmanlar',
        value: layers === null ? 'all' : layers.type === 'list' ? 'list' : 'all',
        options: [
          { value: 'all', label: 'Görünenler' },
          { value: 'list', label: 'Seçilenler' },
        ],
        onChange: (t) => patchKind(c, 'Haritanın katmanları', { layers: t === 'all' ? { type: 'all' } : { type: 'list', layers: hooks.layers().map((l) => l.id) } }),
      }),
      layers?.type === 'theme' ? `“${layers.name}” teması: bu uygulamada katman teması yok; görünen bütün katmanlar çizilir.` : layers?.type === 'list' ? 'Yalnız aşağıda açık olan katmanlar çizilir.' : 'Çizimde görünen bütün katmanlar çizilir.',
    ),
  );
  if (list)
    out.push(
      h(
        'div',
        { class: 'sheet-layers' },
        hooks.layers().map((l) =>
          h(
            'label',
            { class: 'sheet-toggle' },
            toggleSwitch({ label: l.name, checked: list.includes(l.id), disabled: c.readOnly !== null, onChange: (on) => patchKind(c, 'Haritanın katmanları', { layers: { type: 'list', layers: on ? [...list, l.id] : list.filter((x) => x !== l.id) } }) }),
            h('span', null, l.name),
          ),
        ),
      ),
    );
  out.push(...gridFields(c), mmField(c, 'Yazı bandı', 'map.labelBand', v((k) => k.labelBand), (labelBand) => patchKind(c, 'Yazı bandı', { labelBand }), { min: 0 }));
  if (v((k) => k.overviewOf ?? '') !== null && c.items.length === 1) {
    const others = c.sheet.items.filter((i) => i.kind.type === 'map' && i.id !== c.items[0].id);
    if (others.length) out.push(pick(c, 'Genel bakış', 'map.overview', v((k) => k.overviewOf ?? ''), [{ value: '', label: 'Değil' }, ...others.map((o) => ({ value: o.id, label: `“${o.name}” haritasının yerini gösterir` }))], (id) => patchKind(c, 'Genel bakış', { overviewOf: id || null })));
  }
  return out;
}

const GRID_KIND = [
  { value: 'cross' as const, label: 'Artı (karelaj)' },
  { value: 'lines' as const, label: 'Çizgi' },
  { value: 'ticks' as const, label: 'Kenar çentikleri' },
  { value: 'frameOnly' as const, label: 'Yalnız çerçevede' },
];
const GRID_FRAME = [
  { value: 'none' as const, label: 'Yok' },
  { value: 'line' as const, label: 'Çizgi' },
  { value: 'ticks' as const, label: 'Çentik' },
  { value: 'zebra' as const, label: 'Zebra' },
];

/** The grids of one chosen map: each switched on or off, its kind, interval (0: from the scale), frame and labels. */
function gridFields(c: SectionCtx): Child[] {
  if (c.items.length !== 1) return [h('p', { class: 'sheet-insp__hint' }, 'Karelaj tek harita seçiliyken düzenlenir.')];
  const k = kinds(c, 'map')[0];
  const put = (i: number, g: Partial<MapGrid> | null) =>
    patchKindEach(c, 'map', g ? 'Karelaj' : 'Karelajı kaldır', (m) => ({ grids: g ? m.grids.map((x, j) => (j === i ? { ...x, ...g } : x)) : m.grids.filter((_, j) => j !== i) }));
  const add = h('button', { class: 'btn btn--small', type: 'button', disabled: c.readOnly !== null || !c.ctx.commands.isEnabled('sheet.grid') }, icon('sheetGrid', 14), 'Karelaj ekle');
  add.addEventListener('click', () => c.ctx.commands.execute('sheet.grid'));
  c.d.add(tooltip(add, () => ({ title: 'Karelaj ekle', description: 'Proje türünün şablonlarındaki karelajdan başlar; aralığı ölçekten seçilir.', note: c.ctx.commands.get('sheet.grid')?.whyDisabled?.() ?? undefined })));
  const out: Child[] = [h('h4', { class: 'sheet-insp__sub-head' }, 'Karelaj')];
  if (!k.grids.length) out.push(h('p', { class: 'sheet-insp__hint' }, 'Bu haritada karelaj yok.'));
  k.grids.forEach((g, i) => {
    const remove = h('button', { class: 'ibtn', type: 'button', 'aria-label': `${g.name} karelajını kaldır`, disabled: c.readOnly !== null }, icon('trash', 14));
    remove.addEventListener('click', () => put(i, null));
    out.push(
      h(
        'div',
        { class: 'sheet-grid' },
        h('div', { class: 'sheet-insp__row' }, h('span', { class: 'sheet-insp__label' }, g.name), h('span', { class: 'sheet-grid__tools' }, toggleSwitch({ label: `${g.name} açık`, checked: g.enabled, disabled: c.readOnly !== null, onChange: (enabled) => put(i, { enabled }) }), remove)),
        pick(c, 'Biçim', `grid.${i}.kind`, g.kind, GRID_KIND, (kind) => put(i, { kind })),
        pair(
          numField(c, 'Aralık Y', `grid.${i}.ix`, g.interval[0], (x) => put(i, { interval: [Math.max(0, x), g.interval[1]] }), { min: 0, decimals: 0, unit: 'm' }),
          numField(c, 'Aralık X', `grid.${i}.iy`, g.interval[1], (y) => put(i, { interval: [g.interval[0], Math.max(0, y)] }), { min: 0, decimals: 0, unit: 'm' }),
        ),
        h('p', { class: 'sheet-insp__hint' }, g.interval[0] || g.interval[1] ? `Her ${fixed(g.interval[0], 0)} m (Y) ve ${fixed(g.interval[1], 0)} m (X).` : 'Aralık 0: ölçekten seçilir (kâğıtta yaklaşık 10 cm).'),
        pick(c, 'Çerçeve', `grid.${i}.frame`, g.frame, GRID_FRAME, (frame) => put(i, { frame })),
        flag(c, 'Koordinat yazıları', g.labels.show, (show) => put(i, { labels: { ...g.labels, show } })),
      ),
    );
  });
  out.push(h('div', { class: 'sheet-insp__actions' }, add));
  return out;
}
