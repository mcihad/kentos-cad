import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { attributeRows, turnOf } from '../../model/blocks';
import { DIMENSION_STYLE_LABEL, layoutDimension } from '../../model/geom/dimension';
import { ENTITY_KIND_LABEL, HATCH_PATTERN_LABEL, drawsLines, entityArea, entityLength, type DimensionEntity, type Entity, type HatchPatternType, type LeaderEntity, type TextEntity } from '../../model/entities';
import { angleDeg, bearingGrad, dist } from '../../model/geometry';
import { sweep } from '../../model/geom/arc';
import { isFullEllipse, majorLength } from '../../model/geom/ellipse';
import { Panel } from '../dock/Panel';
import { h, replaceChildren } from '../dom';
import { geometryClassOf } from '../../style/geometry';
import { colorSwatch, layerSwatch } from '../layers/swatch';
import { DRAW_COLORS, LINE_WEIGHTS, weightText } from '../ribbon/fields';
import type { MenuItem } from '../widgets/PopupMenu';
import { PropertyGrid, type PropRow, type PropSection } from '../widgets/PropertyGrid';
import { commonElevationRow, lineEndRow, pathElevationRow, spaceRow } from './elevationRows';
import { cornerRows, holeRows } from './pathRows';
import { dimensionRows } from './dimensionRows';
import { leaderRows } from './leaderRows';
import { textRows } from './textRows';
import { setGeometry, setProperties, uidsOf } from './write';
import { fixed } from '../../core/displayNumber';
import { crsCode } from '../../geo/crs';

/**
 * Öznitelikler: geometry and GIS attributes of the selection, editable. It
 * writes through product commands (./write.ts): the layer, colour and
 * attributes with `cad.entities.set`, a geometry value with `cad.entities.edit`.
 */
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
          { label: 'SRID', value: crsCode(doc.crs.value), numeric: true },
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
            // On a hidden layer they vanish from the drawing (still selected): the command's warning says so.
            run: () => setProperties(this.ctx, { uids: uidsOf(this.ctx, ids), layerId: l.id, operation: 'layer' }),
          }),
        ),
    };
  }

  /** A colour as the panel names it: “Çeşitli” for a mixed selection, “Katmana göre”, or the colour's name. */
  private colorText(current: string | undefined | null): string {
    if (current === null) return 'Çeşitli';
    if (current === undefined) return 'Katmana göre';
    return DRAW_COLORS.find((x) => x.value === current)?.name ?? current;
  }

  /** A line weight as the panel names it: “Çeşitli” for a mixed selection, “Katmana göre”, or “0.35 mm”. */
  private weightText(current: number | undefined | null): string {
    if (current === null) return 'Çeşitli';
    if (current === undefined) return 'Katmana göre';
    return weightText(current);
  }

  /** Kalınlık ▾: by layer, then the weights (an imported one that is not in the list among them; docs/adr/0139). */
  private weightEditor(ids: number[], current: number | undefined | null): PropRow['editor'] {
    const set = (lineWeight: number | null) => setProperties(this.ctx, { uids: uidsOf(this.ctx, ids), lineWeight, operation: 'lineWeight' });
    const weights = typeof current === 'number' && !LINE_WEIGHTS.includes(current) ? [...LINE_WEIGHTS, current].sort((a, b) => a - b) : LINE_WEIGHTS;
    return {
      type: 'select',
      display: () => ({ text: this.weightText(current) }),
      items: () => [
        { label: 'Katmana göre', radio: true, checked: current === undefined, run: () => set(null) },
        { kind: 'separator' },
        ...weights.map((w): MenuItem => ({ label: weightText(w), radio: true, checked: current === w, run: () => set(w) })),
      ],
    };
  }

  /** A symbol as the panel names it: “Çeşitli”, “Katman stiline göre”, or its name in the library. */
  private symbolText(current: string | undefined | null): string {
    if (current === null) return 'Çeşitli';
    if (current === undefined) return 'Katman stiline göre';
    return this.ctx.styles.library.get(current)?.name ?? 'Kitaplıkta yok';
  }

  private colorEditor(ids: number[], current: string | undefined | null): PropRow['editor'] {
    const set = (color: string | undefined) => setProperties(this.ctx, { uids: uidsOf(this.ctx, ids), color: color ?? null, operation: 'color' });
    return {
      type: 'select',
      display: () => {
        const c = DRAW_COLORS.find((x) => x.value === current);
        return c ? { text: c.name, swatch: colorSwatch(c.value, this.ctx.view.palette) } : { text: this.colorText(current) };
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
    const { commands } = this.ctx;
    return {
      type: 'select',
      display: () => ({ text: this.symbolText(current) }),
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
        { label: 'Renk', value: this.colorText(e.color), editor: locked ? undefined : this.colorEditor([e.id], e.color) },
        ...(drawsLines(e) ? [{ label: 'Kalınlık', value: this.weightText(e.lineWeight), editor: locked ? undefined : this.weightEditor([e.id], e.lineWeight) }] : []),
        ...(geometryClassOf(e) ? [{ label: 'Sembol', value: this.symbolText(e.symbol), editor: locked ? undefined : this.symbolEditor(e.symbol) }] : []),
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
            { label: 'Alan', value: fixed(m2, this.ctx.doc.settings.areaDecimals.value), numeric: true, unit: 'm²' },
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
                  if (Number.isFinite(n)) setGeometry(this.ctx, e, { p: { ...e.p, [axis]: n } });
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
          lineEndRow(this.ctx, e, 0, locked),
          num('Bitiş Y', e.b.x),
          num('Bitiş X', e.b.y),
          lineEndRow(this.ctx, e, 1, locked),
          num('Uzunluk', dist(e.a, e.b), 'm'),
          ...spaceRow(this.ctx, e),
          { label: 'Semt', value: f.bearing(bearingGrad(e.a, e.b), false), numeric: true, unit: f.angleUnitLabel },
        );
        break;
      case 'polyline':
      case 'polygon': {
        geo.push(...cornerRows(e));
        geo.push(pathElevationRow(this.ctx, e, locked));
        geo.push(num(e.kind === 'polygon' ? 'Çevre' : 'Uzunluk', entityLength(e)!, 'm'), ...spaceRow(this.ctx, e));
        if (e.kind === 'polygon') {
          // Net area: holes (adalar) are already taken out.
          geo.push(...holeRows(e));
          geo.push(...area(entityArea(e)!));
        }
        break;
      }
      case 'circle':
        geo.push(num('Merkez Y', e.c.x), num('Merkez X', e.c.y), num('Yarıçap', e.r, 'm'), ...area(entityArea(e)!));
        break;
      case 'arc': {
        const d = (rad: number) => fixed((rad * 180) / Math.PI, 4);
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
        const d = (rad: number) => fixed(((((rad * 180) / Math.PI) % 360) + 360) % 360, 4);
        const a = majorLength(e);
        const full = isFullEllipse(e);
        geo.push(
          num('Merkez Y', e.c.x),
          num('Merkez X', e.c.y),
          num('Büyük yarı eksen', a, 'm'),
          num('Küçük yarı eksen', a * e.ratio, 'm'),
          { label: 'Eksen açısı', value: fixed(((angleDeg({ x: 0, y: 0 }, e.major) % 360) + 360) % 360, 4), numeric: true, unit: '°' },
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
          { label: 'Doğrultu', value: fixed(((angleDeg({ x: 0, y: 0 }, e.dir) % 360) + 360) % 360, 4), numeric: true, unit: '°' },
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
                  if (Number.isFinite(x) && (key !== 'height' || x > 0)) setGeometry(this.ctx, e, { [key]: x });
                },
              },
        });
        const style = e.style ?? 'aligned';
        const l = layoutDimension(e);
        geo.push({ label: 'Tür', value: DIMENSION_STYLE_LABEL[style] });
        // What it measured, by its unit (docs/adr/0147 §2).
        if (l?.unit === 'angle') geo.push({ label: style === 'azimuth' ? 'Ölçülen semt' : 'Ölçülen açı', value: f.angle(l.value, false), numeric: true, unit: f.angleUnitLabel });
        else if (l?.unit === 'percent') geo.push({ label: 'Ölçülen eğim', value: f.percent(l.value), numeric: true, unit: '%' });
        else if (l?.unit === 'coordinate') geo.push(num(l.prefix === 'Y=' ? 'Ölçülen Y' : 'Ölçülen X', l.value, 'm'));
        else if (l) geo.push(num(style === 'diameter' ? 'Ölçülen çap' : style === 'radius' || style === 'jogged' ? 'Ölçülen yarıçap' : 'Ölçülen uzunluk', l.value, 'm'));
        if (style === 'aligned') geo.push({ label: 'Semt', value: f.bearing(bearingGrad(e.a, e.b), false), numeric: true, unit: f.angleUnitLabel });
        // An ordinate has no offset; a jogged radius's is where its jog is.
        if (style !== 'ordinate') geo.push(n(style === 'angular' ? 'Yay yarıçapı' : style === 'radius' || style === 'diameter' ? 'Dışa uzantı' : style === 'jogged' ? 'Kırık uzaklığı' : 'Ötelenme', 'offset', e.offset));
        geo.push(
          n('Yazı yüksekliği', 'height', e.height),
          {
            label: 'Yazı',
            value: e.text ?? '',
            editor: locked ? undefined : { type: 'text', commit: (v) => setGeometry(this.ctx, e, { text: v.trim() || undefined }) },
          },
          // Zemin, an ordinate's axis, a slope's elevations, an arc length's radius and angle (docs/adr/0147 §7).
          ...dimensionRows(this.ctx, [e], locked),
        );
        break;
      }
      case 'hatch': {
        const types = Object.keys(HATCH_PATTERN_LABEL) as HatchPatternType[];
        const setPattern = (patch: Partial<typeof e.pattern>) => setGeometry(this.ctx, e, { pattern: { ...e.pattern, ...patch } });
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
          { label: 'Açı', value: fixed(e.pattern.angle, 2), numeric: true, unit: '°', editor: numEdit('angle') },
          { label: 'Aralık', value: f.length(e.pattern.spacing, false), numeric: true, unit: 'm', editor: numEdit('spacing') },
          ...area(entityArea(e)!),
        );
        break;
      }
      case 'text':
        geo.push(
          // Trimmed, as the in-place editor stores it; an empty text is not taken.
          { label: 'Metin', value: e.text, editor: locked ? undefined : { type: 'text', commit: (v) => v.trim() && setGeometry(this.ctx, e, { text: v.trim() }) } },
          {
            ...num('Yükseklik', e.height, 'm'),
            editor: locked
              ? undefined
              : {
                  type: 'number',
                  commit: (t: string) => {
                    const x = parseFloat(t.replace(',', '.'));
                    if (Number.isFinite(x) && x > 0) setGeometry(this.ctx, e, { height: x });
                  },
                },
          },
          {
            label: 'Açı',
            value: fixed(e.rotation, 2),
            numeric: true,
            unit: '°',
            editor: locked
              ? undefined
              : {
                  type: 'number',
                  commit: (t: string) => {
                    const x = parseFloat(t.replace(',', '.'));
                    if (Number.isFinite(x)) setGeometry(this.ctx, e, { rotation: ((x % 360) + 360) % 360 });
                  },
                },
          },
          // Hiza, Genişlik çarpanı and Zemin (docs/adr/0145 §6).
          ...textRows(this.ctx, [e], locked),
          num('Konum Y', e.p.x),
          num('Konum X', e.p.y),
        );
        break;
      // The block, its place, scale, turn and mirroring, through `cad.entities.edit`'s properties (docs/adr/0144 §6);
      // the desktop's rows are the same. A scale not above zero is not taken; a turn is typed in degrees.
      case 'insert': {
        const name = this.ctx.doc.block(e.block)?.name ?? '(tanımsız)';
        const numEdit = (patch: (x: number) => Record<string, unknown> | null) =>
          locked
            ? undefined
            : ({
                type: 'number',
                commit: (t: string) => {
                  const x = parseFloat(t.replace(',', '.'));
                  const changed = Number.isFinite(x) ? patch(x) : null;
                  if (changed) setGeometry(this.ctx, e, changed);
                },
              } as const);
        const mirrored = e.mirror === true;
        geo.push(
          {
            label: 'Blok',
            value: name,
            editor: locked
              ? undefined
              : {
                  type: 'select',
                  display: () => ({ text: name }),
                  items: () => this.ctx.doc.blocks.value.map((b) => ({ label: b.name, radio: true, checked: b.id === e.block, run: () => setGeometry(this.ctx, e, { block: b.id }) })),
                },
          },
          { ...num('Konum Y', e.p.x), editor: numEdit((x) => ({ p: { ...e.p, x } })) },
          { ...num('Konum X', e.p.y), editor: numEdit((y) => ({ p: { ...e.p, y } })) },
          { label: 'Ölçek', value: fixed(e.scale, 4), numeric: true, editor: numEdit((x) => (x > 0 ? { scale: x } : null)) },
          { label: 'Dönüş', value: fixed((e.rotation * 180) / Math.PI, 4), numeric: true, unit: '°', editor: numEdit((x) => ({ rotation: turnOf(x) })) },
          {
            label: 'Aynalı',
            value: mirrored ? 'Evet' : 'Hayır',
            editor: locked
              ? undefined
              : {
                  type: 'select',
                  display: () => ({ text: mirrored ? 'Evet' : 'Hayır' }),
                  items: () => [true, false].map((m) => ({ label: m ? 'Evet' : 'Hayır', radio: true, checked: m === mirrored, run: () => setGeometry(this.ctx, e, { mirror: m }) })),
                },
          },
        );
        break;
      }
      // Its note, height, turn, arrowhead and mask, its corners and length (docs/adr/0146 §7). The desktop's are the same.
      case 'leader':
        geo.push(...leaderRows(this.ctx, [e], locked), { label: 'Köşe sayısı', value: String(e.pts.length), numeric: true }, num('Uzunluk', entityLength(e) ?? 0, 'm'));
        break;
    }

    const sections: PropSection[] = [general, { id: 'geometry', title: 'Geometri', rows: geo }];
    // An insert's block attributes (docs/adr/0144 §7): the definition's tags in its order, with what the insert shows.
    const shown = e.kind === 'insert' ? attributeRows(this.ctx.doc.block(e.block), e.attrs) : [];
    if (shown.length) {
      sections.push({
        id: 'blockAttrs',
        title: 'Blok öznitelikleri',
        rows: shown.map(({ tag, value }) => ({
          label: tag,
          value,
          numeric: /^-?\d+([.,]\d+)?$/.test(value),
          editor: locked ? undefined : { type: 'text', commit: (v: string) => setProperties(this.ctx, { uids: uidsOf(this.ctx, [e.id]), attrs: { [tag]: v }, operation: 'attributes' }) },
        })),
      });
    }
    const tags = new Set(shown.map((r) => r.tag));
    const keys = Object.keys(e.attrs).filter((k) => !tags.has(k));
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
                  // Keep the drawn number in sync with the cadastral attribute.
                  const label = (k === 'Parsel' || k === 'Ada') && e.label === e.attrs[k] ? { label: v } : {};
                  setProperties(this.ctx, { uids: uidsOf(this.ctx, [e.id]), attrs: { [k]: v }, ...label, operation: 'attributes' });
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
    const symbol = ents.every((e) => e.symbol === ents[0].symbol) ? ents[0].symbol : null;
    // The weight of the objects drawn with lines; the others have none to show.
    const lined = ents.filter(drawsLines);
    const weight = lined.every((e) => e.lineWeight === lined[0]?.lineWeight) ? lined[0]?.lineWeight : null;
    // With a locked object in the selection nothing is editable: the values are still named.
    const rows: PropRow[] = [
      { label: 'Katman', value: 'Kilitli katman içeriyor', editor: anyLocked ? undefined : this.layerEditor(ids, layer) },
      { label: 'Renk', value: this.colorText(color), editor: anyLocked ? undefined : this.colorEditor(ids, color) },
      ...(lined.length
        ? [{ label: 'Kalınlık', value: this.weightText(weight), editor: anyLocked ? undefined : this.weightEditor(lined.map((e) => e.id), weight) }]
        : []),
      { label: 'Sembol', value: this.symbolText(symbol), editor: anyLocked ? undefined : this.symbolEditor(symbol) },
      ...commonElevationRow(this.ctx, ents, anyLocked),
    ];
    const totals: PropRow[] = [];
    const f = this.ctx.format;
    if (length > 0) totals.push({ label: 'Toplam uzunluk', value: f.length(length, false), numeric: true, unit: 'm' });
    if (area > 0) totals.push({ label: 'Toplam alan', value: f.area(area, false), numeric: true, unit: f.areaUnitLabel });
    const sections: PropSection[] = [{ id: 'general', title: 'Ortak özellikler', rows }];
    // The selection's texts: their Hiza, Genişlik çarpanı and Zemin, common or “Çeşitli” (docs/adr/0145 §6).
    const texts = ents.filter((e): e is TextEntity => e.kind === 'text');
    if (texts.length) sections.push({ id: 'texts', title: texts.length === ents.length ? 'Yazı' : `Yazılar (${texts.length})`, rows: textRows(this.ctx, texts, anyLocked) });
    // The selection's leaders: their note, height, turn, arrowhead and mask, common or “Çeşitli” (docs/adr/0146 §7).
    const leaders = ents.filter((e): e is LeaderEntity => e.kind === 'leader');
    if (leaders.length)
      sections.push({ id: 'leaders', title: leaders.length === ents.length ? 'Kılavuz' : `Kılavuzlar (${leaders.length})`, rows: leaderRows(this.ctx, leaders, anyLocked) });
    // The selection's dimensions: their Zemin, and an ordinate's axis, a slope's elevations, common or “Çeşitli” (docs/adr/0147 §7).
    const dims = ents.filter((e): e is DimensionEntity => e.kind === 'dimension');
    if (dims.length) sections.push({ id: 'dimensions', title: dims.length === ents.length ? 'Ölçü' : `Ölçüler (${dims.length})`, rows: dimensionRows(this.ctx, dims, anyLocked) });
    if (totals.length) sections.push({ id: 'totals', title: 'Toplamlar', rows: totals });
    return sections;
  }
}
