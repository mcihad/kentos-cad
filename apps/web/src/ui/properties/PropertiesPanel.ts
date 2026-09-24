import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { DIMENSION_STYLE_LABEL, layoutDimension } from '../../model/geom/dimension';
import { ENTITY_KIND_LABEL, HATCH_PATTERN_LABEL, entityArea, entityLength, type Entity, type HatchPatternType } from '../../model/entities';
import { angleDeg, bearingGrad, dist } from '../../model/geometry';
import { sweep } from '../../model/geom/arc';
import { isFullEllipse, majorLength } from '../../model/geom/ellipse';
import { Panel } from '../dock/Panel';
import { h, replaceChildren } from '../dom';
import { geometryClassOf } from '../../style/geometry';
import { colorSwatch, layerSwatch } from '../layers/swatch';
import { DRAW_COLORS } from '../toolbar/fields';
import type { MenuItem } from '../widgets/PopupMenu';
import { PropertyGrid, type PropRow, type PropSection } from '../widgets/PropertyGrid';

/** Öznitelikler: geometry and GIS attributes of the selection, editable. */
export class PropertiesPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly grid = new PropertyGrid();
  private readonly summary = h('div', { class: 'props-summary' });
  private readonly empty = h('div', { class: 'empty' });
  private scheduled = false;

  constructor(ctx: AppContext) {
    super({ title: 'Öznitelikler', className: 'panel--props' });
    this.ctx = ctx;
    this.body.append(this.summary, this.empty, this.grid.el);
    const refresh = () => this.schedule();
    this.d.add(watchAll([ctx.selection.ids, ctx.doc.layers.version, ctx.doc.layers.active, ctx.doc.settings.changed, ctx.format.changed], refresh));
    this.d.add(ctx.doc.events.on('changed', refresh));
    this.d.add(ctx.doc.events.on('attrs', refresh));
    this.render();
  }

  private schedule(): void {
    if (this.scheduled) return;
    this.scheduled = true;
    queueMicrotask(() => {
      this.scheduled = false;
      this.render();
    });
  }

  private render(): void {
    const { doc, selection } = this.ctx;
    const ents = [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
    this.empty.hidden = ents.length > 0;
    if (!ents.length) {
      this.setMeta('');
      this.summary.hidden = true;
      replaceChildren(this.empty, 
        h('p', { class: 'empty__title' }, 'Seçili nesne yok'),
        h('p', { class: 'empty__text' }, 'Özelliklerini görmek için çizimde bir nesneye tıklayın. Birden fazla nesne için sürükleyerek seçin.'),
      );
      this.grid.render(this.documentSections());
      return;
    }
    this.summary.hidden = false;
    if (ents.length === 1) {
      const e = ents[0];
      const layer = doc.layers.get(e.layerId);
      this.setMeta(`#${e.id}`);
      replaceChildren(this.summary, 
        h('span', { class: 'props-summary__kind' }, ENTITY_KIND_LABEL[e.kind], e.label ? h('span', { class: 'props-summary__label' }, e.label) : null),
        layer ? h('span', { class: 'props-summary__layer' }, h('span', { class: 'swatch', style: `--swatch:${layerSwatch(layer, this.ctx.view.palette)}` }), doc.layers.path(layer.id)) : null,
      );
      this.grid.render(this.entitySections(e));
    } else {
      this.setMeta(`${ents.length} nesne`);
      const kinds = new Map<string, number>();
      ents.forEach((e) => kinds.set(ENTITY_KIND_LABEL[e.kind], (kinds.get(ENTITY_KIND_LABEL[e.kind]) ?? 0) + 1));
      replaceChildren(this.summary, 
        h('span', { class: 'props-summary__kind' }, `${ents.length} nesne seçili`),
        h('span', { class: 'props-summary__layer' }, [...kinds].map(([k, n]) => `${n} ${k.toLocaleLowerCase('tr-TR')}`).join(', ')),
      );
      this.grid.render(this.multiSections(ents));
    }
  }

  private documentSections(): PropSection[] {
    const { doc } = this.ctx;
    const active = doc.layers.get(doc.layers.active.value);
    return [
      {
        id: 'doc',
        title: 'Çizim',
        rows: [
          { label: 'Dosya', value: doc.name.value },
          { label: 'Koordinat sistemi', value: doc.crs.value.name },
          { label: 'SRID', value: `EPSG:${doc.crs.value.srid}`, numeric: true },
          { label: 'Çizim ölçeği', value: `1:${doc.settings.plotScale.value}`, numeric: true },
          { label: 'Nesne sayısı', value: String(doc.size), numeric: true },
          { label: 'Etkin katman', value: active ? doc.layers.path(active.id) : '—' },
        ],
      },
    ];
  }

  private layerEditor(ids: number[], currentId: string | null): PropRow['editor'] {
    const { doc } = this.ctx;
    return {
      type: 'select',
      display: () => {
        const n = currentId ? doc.layers.get(currentId) : null;
        return n ? { text: n.name, swatch: layerSwatch(n, this.ctx.view.palette) } : { text: 'Çeşitli' };
      },
      items: () =>
        doc.layers.leaves().map(
          (l): MenuItem => ({
            label: doc.layers.path(l.id),
            swatch: layerSwatch(l, this.ctx.view.palette),
            radio: true,
            checked: l.id === currentId,
            disabled: doc.layers.isLocked(l.id),
            run: () => doc.transact('Katman değiştir', () => ids.forEach((id) => doc.update(id, { layerId: l.id }))),
          }),
        ),
    };
  }

  private colorEditor(ids: number[], current: string | undefined | null): PropRow['editor'] {
    const { doc } = this.ctx;
    const set = (color: string | undefined) => doc.transact('Renk değiştir', () => ids.forEach((id) => doc.update(id, { color })));
    return {
      type: 'select',
      display: () => {
        if (current === null) return { text: 'Çeşitli' };
        const c = DRAW_COLORS.find((x) => x.value === current);
        return c ? { text: c.name, swatch: colorSwatch(c.value, this.ctx.view.palette) } : { text: 'Katmana göre' };
      },
      items: () => [
        { label: 'Katmana göre', radio: true, checked: current === undefined, run: () => set(undefined) },
        { kind: 'separator' },
        ...DRAW_COLORS.map((c): MenuItem => ({ label: c.name, swatch: colorSwatch(c.value, this.ctx.view.palette), radio: true, checked: current === c.value, run: () => set(c.value) })),
      ],
    };
  }

  /** An object's own symbol (drawn instead of its layer's style); picked from the library. */
  private symbolEditor(current: string | undefined | null): PropRow['editor'] {
    const { commands, styles } = this.ctx;
    return {
      type: 'select',
      display: () => (current === null ? { text: 'Çeşitli' } : current ? { text: styles.library.get(current)?.name ?? 'Kitaplıkta yok' } : { text: 'Katman stiline göre' }),
      items: () => [
        { label: 'Katman stiline göre', radio: true, checked: current === undefined, run: () => commands.execute('style.clearSymbol') },
        { kind: 'separator' },
        { label: 'Kitaplıktan seç…', icon: 'styles', run: () => commands.execute('style.assign') },
        { label: 'Katman stili…', icon: 'layerStyle', run: () => commands.execute('style.layerStyle') },
      ],
    };
  }

  private entitySections(e: Entity): PropSection[] {
    const { doc } = this.ctx;
    const locked = doc.layers.isLocked(e.layerId);
    const general: PropSection = {
      id: 'general',
      title: 'Genel',
      rows: [
        { label: 'Tür', value: ENTITY_KIND_LABEL[e.kind] },
        { label: 'Katman', value: '', editor: locked ? undefined : this.layerEditor([e.id], e.layerId) },
        { label: 'Renk', value: e.color ?? 'Katmana göre', editor: locked ? undefined : this.colorEditor([e.id], e.color) },
        ...(geometryClassOf(e) ? [{ label: 'Sembol', value: e.symbol ? (this.ctx.styles.library.get(e.symbol)?.name ?? e.symbol) : 'Katman stiline göre', editor: locked ? undefined : this.symbolEditor(e.symbol) }] : []),
      ],
    };
    if (locked) general.rows[1].value = doc.layers.path(e.layerId) + ' (kilitli)';

    const f = this.ctx.format;
    const geo: PropRow[] = [];
    const num = (label: string, v: number, unit?: string): PropRow => ({ label, value: f.length(v, false), numeric: true, unit });
    const area = (m2: number): PropRow[] =>
      this.ctx.doc.settings.areaUnit.value === 'm2'
        ? [{ label: 'Alan', value: f.area(m2, false), numeric: true, unit: 'm²' }]
        : [
            { label: 'Alan', value: m2.toFixed(this.ctx.doc.settings.areaDecimals.value), numeric: true, unit: 'm²' },
            { label: 'Alan', value: f.area(m2, false), numeric: true, unit: f.areaUnitLabel },
          ];
    switch (e.kind) {
      case 'point': {
        const edit = (axis: 'x' | 'y') =>
          locked
            ? undefined
            : ({
                type: 'number',
                commit: (v: string) => {
                  const n = parseFloat(v.replace(',', '.'));
                  if (Number.isFinite(n)) doc.update(e.id, { p: { ...e.p, [axis]: n } } as Partial<Entity>);
                },
              } as const);
        geo.push({ ...num('Y (sağa)', e.p.x, 'm'), editor: edit('x') }, { ...num('X (yukarı)', e.p.y, 'm'), editor: edit('y') });
        if (e.z !== undefined) geo.push(num('Z (kot)', e.z, 'm'));
        break;
      }
      case 'line':
        geo.push(
          num('Başlangıç Y', e.a.x),
          num('Başlangıç X', e.a.y),
          num('Bitiş Y', e.b.x),
          num('Bitiş X', e.b.y),
          num('Uzunluk', dist(e.a, e.b), 'm'),
          { label: 'Semt', value: f.bearing(bearingGrad(e.a, e.b), false), numeric: true, unit: f.angleUnitLabel },
        );
        break;
      case 'polyline':
      case 'polygon': {
        geo.push({ label: 'Köşe sayısı', value: String(e.pts.length), numeric: true });
        geo.push(num(e.kind === 'polygon' ? 'Çevre' : 'Uzunluk', entityLength(e)!, 'm'));
        if (e.kind === 'polygon') {
          // Net area: holes (adalar) are already taken out.
          if (e.holes?.length) geo.push({ label: 'Ada (delik)', value: String(e.holes.length), numeric: true });
          geo.push(...area(entityArea(e)!));
        }
        break;
      }
      case 'circle':
        geo.push(num('Merkez Y', e.c.x), num('Merkez X', e.c.y), num('Yarıçap', e.r, 'm'), ...area(entityArea(e)!));
        break;
      case 'arc': {
        const d = (rad: number) => ((rad * 180) / Math.PI).toFixed(4);
        geo.push(
          num('Merkez Y', e.c.x),
          num('Merkez X', e.c.y),
          num('Yarıçap', e.r, 'm'),
          { label: 'Başlangıç açısı', value: d(e.a0), numeric: true, unit: '°' },
          { label: 'Bitiş açısı', value: d(e.a1), numeric: true, unit: '°' },
          { label: 'Yay açısı', value: d(sweep(e.a0, e.a1)), numeric: true, unit: '°' },
          num('Yay uzunluğu', entityLength(e)!, 'm'),
        );
        break;
      }
      case 'ellipse': {
        const d = (rad: number) => ((((rad * 180) / Math.PI) % 360 + 360) % 360).toFixed(4);
        const a = majorLength(e);
        const full = isFullEllipse(e);
        geo.push(
          num('Merkez Y', e.c.x),
          num('Merkez X', e.c.y),
          num('Büyük yarı eksen', a, 'm'),
          num('Küçük yarı eksen', a * e.ratio, 'm'),
          { label: 'Eksen açısı', value: (((angleDeg({ x: 0, y: 0 }, e.major) % 360) + 360) % 360).toFixed(4), numeric: true, unit: '°' },
          ...(full
            ? [num('Çevre', entityLength(e)!, 'm'), ...area(entityArea(e)!)]
            : [
                { label: 'Başlangıç parametresi', value: d(e.t0), numeric: true, unit: '°' },
                { label: 'Bitiş parametresi', value: d(e.t1), numeric: true, unit: '°' },
                num('Yay uzunluğu', entityLength(e)!, 'm'),
              ]),
        );
        break;
      }
      case 'xline':
      case 'ray':
        geo.push(
          num(e.kind === 'ray' ? 'Başlangıç Y' : 'Geçtiği nokta Y', e.p.x),
          num(e.kind === 'ray' ? 'Başlangıç X' : 'Geçtiği nokta X', e.p.y),
          { label: 'Doğrultu', value: (((angleDeg({ x: 0, y: 0 }, e.dir) % 360) + 360) % 360).toFixed(4), numeric: true, unit: '°' },
          { label: 'Semt', value: f.bearing(bearingGrad(e.p, { x: e.p.x + e.dir.x, y: e.p.y + e.dir.y }), false), numeric: true, unit: f.angleUnitLabel },
        );
        break;
      case 'spline':
        geo.push(
          { label: 'Nokta sayısı', value: String(e.pts.length), numeric: true },
          { label: 'Kapalı', value: e.closed ? 'Evet' : 'Hayır' },
          num('Uzunluk', entityLength(e)!, 'm'),
        );
        break;
      case 'dimension': {
        const n = (label: string, key: 'offset' | 'height', v: number): PropRow => ({
          ...num(label, v, 'm'),
          editor: locked
            ? undefined
            : {
                type: 'number',
                commit: (t: string) => {
                  const x = parseFloat(t.replace(',', '.'));
                  if (Number.isFinite(x) && (key !== 'height' || x > 0)) doc.update(e.id, { [key]: x } as Partial<Entity>);
                },
              },
        });
        const style = e.style ?? 'aligned';
        const l = layoutDimension(e);
        geo.push({ label: 'Tür', value: DIMENSION_STYLE_LABEL[style] });
        if (l?.unit === 'angle') geo.push({ label: 'Ölçülen açı', value: f.angle(l.value, false), numeric: true, unit: f.angleUnitLabel });
        else if (l) geo.push(num(style === 'diameter' ? 'Ölçülen çap' : style === 'radius' ? 'Ölçülen yarıçap' : 'Ölçülen uzunluk', l.value, 'm'));
        if (style === 'aligned') geo.push({ label: 'Semt', value: f.bearing(bearingGrad(e.a, e.b), false), numeric: true, unit: f.angleUnitLabel });
        geo.push(
          n(style === 'angular' ? 'Yay yarıçapı' : style === 'radius' || style === 'diameter' ? 'Dışa uzantı' : 'Ötelenme', 'offset', e.offset),
          n('Yazı yüksekliği', 'height', e.height),
          {
            label: 'Yazı',
            value: e.text ?? '',
            editor: locked ? undefined : { type: 'text', commit: (v) => doc.update(e.id, { text: v.trim() || undefined } as Partial<Entity>) },
          },
        );
        break;
      }
      case 'hatch': {
        const types = Object.keys(HATCH_PATTERN_LABEL) as HatchPatternType[];
        const setPattern = (patch: Partial<typeof e.pattern>) => doc.update(e.id, { pattern: { ...e.pattern, ...patch } } as Partial<Entity>);
        const numEdit = (key: 'angle' | 'spacing') =>
          locked
            ? undefined
            : ({
                type: 'number',
                commit: (t: string) => {
                  const x = parseFloat(t.replace(',', '.'));
                  if (Number.isFinite(x) && (key === 'angle' || x > 0)) setPattern({ [key]: x });
                },
              } as const);
        geo.push(
          {
            label: 'Desen',
            value: HATCH_PATTERN_LABEL[e.pattern.type],
            editor: locked
              ? undefined
              : {
                  type: 'select',
                  display: () => ({ text: HATCH_PATTERN_LABEL[e.pattern.type] }),
                  items: () => types.map((t) => ({ label: HATCH_PATTERN_LABEL[t], radio: true, checked: t === e.pattern.type, run: () => setPattern({ type: t }) })),
                },
          },
          { label: 'Açı', value: e.pattern.angle.toFixed(2), numeric: true, unit: '°', editor: numEdit('angle') },
          { label: 'Aralık', value: f.length(e.pattern.spacing, false), numeric: true, unit: 'm', editor: numEdit('spacing') },
          ...area(entityArea(e)!),
        );
        break;
      }
      case 'text':
        geo.push(
          { label: 'Metin', value: e.text, editor: locked ? undefined : { type: 'text', commit: (v) => v.trim() && doc.update(e.id, { text: v } as Partial<Entity>) } },
          {
            ...num('Yükseklik', e.height, 'm'),
            editor: locked
              ? undefined
              : {
                  type: 'number',
                  commit: (t: string) => {
                    const x = parseFloat(t.replace(',', '.'));
                    if (Number.isFinite(x) && x > 0) doc.update(e.id, { height: x } as Partial<Entity>);
                  },
                },
          },
          {
            label: 'Açı',
            value: e.rotation.toFixed(2),
            numeric: true,
            unit: '°',
            editor: locked
              ? undefined
              : {
                  type: 'number',
                  commit: (t: string) => {
                    const x = parseFloat(t.replace(',', '.'));
                    if (Number.isFinite(x)) doc.update(e.id, { rotation: ((x % 360) + 360) % 360 } as Partial<Entity>);
                  },
                },
          },
          num('Konum Y', e.p.x),
          num('Konum X', e.p.y),
        );
        break;
    }

    const sections: PropSection[] = [general, { id: 'geometry', title: 'Geometri', rows: geo }];
    const keys = Object.keys(e.attrs);
    if (keys.length) {
      sections.push({
        id: 'attrs',
        title: 'Öznitelik bilgileri',
        rows: keys.map((k) => ({
          label: k,
          value: e.attrs[k],
          numeric: /^-?\d+([.,]\d+)?$/.test(e.attrs[k]),
          editor: locked
            ? undefined
            : {
                type: 'text',
                commit: (v: string) => {
                  const patch: Partial<Entity> = { attrs: { ...e.attrs, [k]: v } };
                  // Keep the drawn number in sync with the cadastral attribute.
                  if ((k === 'Parsel' || k === 'Ada') && e.label === e.attrs[k]) patch.label = v;
                  doc.update(e.id, patch);
                },
              },
        })),
      });
    }
    return sections;
  }

  private multiSections(ents: Entity[]): PropSection[] {
    const ids = ents.map((e) => e.id);
    const layer = ents.every((e) => e.layerId === ents[0].layerId) ? ents[0].layerId : null;
    const color = ents.every((e) => e.color === ents[0].color) ? ents[0].color : null;
    const anyLocked = ents.some((e) => this.ctx.doc.layers.isLocked(e.layerId));
    // Summed by the geometry store: a selection can hold tens of thousands of objects.
    const { length, area } = this.ctx.view.measure(ids);
    const rows: PropRow[] = [
      { label: 'Katman', value: 'Kilitli katman içeriyor', editor: anyLocked ? undefined : this.layerEditor(ids, layer) },
      { label: 'Renk', value: '', editor: anyLocked ? undefined : this.colorEditor(ids, color) },
      { label: 'Sembol', value: '', editor: anyLocked ? undefined : this.symbolEditor(ents.every((e) => e.symbol === ents[0].symbol) ? ents[0].symbol : null) },
    ];
    const totals: PropRow[] = [];
    const f = this.ctx.format;
    if (length > 0) totals.push({ label: 'Toplam uzunluk', value: f.length(length, false), numeric: true, unit: 'm' });
    if (area > 0) totals.push({ label: 'Toplam alan', value: f.area(area, false), numeric: true, unit: f.areaUnitLabel });
    const sections: PropSection[] = [{ id: 'general', title: 'Ortak özellikler', rows }];
    if (totals.length) sections.push({ id: 'totals', title: 'Toplamlar', rows: totals });
    return sections;
  }
}
