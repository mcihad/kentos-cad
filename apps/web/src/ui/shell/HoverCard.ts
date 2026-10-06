import type { AppContext } from '../../app/context';
import type { Formatter } from '../../app/format';
import { watchAll } from '../../core/signal';
import { ENTITY_KIND_LABEL, entityArea, entityLength, type Entity } from '../../model/entities';
import { pathCounts } from '../../model/pathCounts';
import { spaceLength } from '../../product/elevationValues';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';
import { besidePointer, type Size } from '../widgets/placeBeside';

const DELAY_MS = 500;

/**
 * The registered (tapu) area as the card shows it: the attribute as written,
 * not parsed, rounded or converted (it is the deed's value, CLAUDE.md §7,
 * §23.1), with “m²” after a plain decimal number (a point or a comma); null
 * when there is none.
 */
export function deedAreaText(attrs: Readonly<Record<string, string>>): string | null {
  const text = (attrs['Tapu alanı (m²)'] ?? '').trim();
  if (!text) return null;
  return /^\d+([.,]\d+)?$/.test(text) ? `${text} m²` : text;
}

/**
 * Rollover card: resting the mouse on an object (select tool) shows what
 * it is — kind, layer, length or area, parcel data — without selecting it.
 */
export class HoverCard extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private timer = 0;
  private shownId: number | null = null;
  /** The card's size, measured once when shown: its content does not change while it follows the pointer. */
  private size: Size = { w: 0, h: 0 };

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.el = h('div', { class: 'hover-card', hidden: true, 'aria-hidden': 'true' });
    host.append(this.el);
    const { selection, view, tools, prefs } = ctx;
    this.d.add(
      watchAll([selection.hover, tools.activeId, prefs.hoverInfo], () => {
        clearTimeout(this.timer);
        this.hide();
        const id = selection.hover.value;
        if (id === null || tools.activeId.value !== 'select' || !prefs.hoverInfo.value) return;
        this.timer = window.setTimeout(() => this.show(id), DELAY_MS);
      }),
    );
    this.d.add(
      view.cursorWorld.subscribe((w) => {
        if (!w) {
          clearTimeout(this.timer);
          this.hide();
        } else if (this.shownId !== null) this.place();
      }),
    );
    this.d.add(() => clearTimeout(this.timer));
  }

  private show(id: number): void {
    const e = this.ctx.doc.get(id);
    if (!e) return;
    this.shownId = id;
    replaceChildren(this.el, ...this.content(e));
    this.el.hidden = false;
    this.size = { w: this.el.offsetWidth, h: this.el.offsetHeight };
    this.place();
  }

  private hide(): void {
    this.shownId = null;
    this.el.hidden = true;
  }

  /** Right of and below the pointer; left of it or above it near the drawing's edges, never outside (placeBeside.ts). */
  private place(): void {
    const w = this.ctx.view.cursorWorld.value;
    if (!w) return;
    const camera = this.ctx.view.camera;
    const at = besidePointer(camera.worldToScreen(w), this.size, { w: camera.width, h: camera.height });
    this.el.style.transform = `translate(${at.x}px, ${at.y}px)`;
  }

  private content(e: Entity): HTMLElement[] {
    const { doc, format, view } = this.ctx;
    const layer = doc.layers.get(e.layerId);
    const a = e.attrs;
    const rows = cardRows(e, format);
    return [
      h(
        'div',
        { class: 'hover-card__head' },
        h('b', null, a.Parsel ? `Parsel ${a.Parsel}` : e.label ? `${ENTITY_KIND_LABEL[e.kind]} ${e.label}` : ENTITY_KIND_LABEL[e.kind]),
        layer
          ? h('span', { class: 'hover-card__layer' }, h('span', { class: 'swatch', style: `--swatch:${colorSwatch(e.color ?? layer.style.color, view.palette)}` }), h('span', null, layer.name))
          : null,
      ),
      ...(rows.length ? [h('dl', { class: 'hover-card__rows' }, rows.map(([k, v]) => [h('dt', null, k), h('dd', { class: 'num' }, v)]))] : []),
    ];
  }
}

/**
 * The rows under the card's head, name and value: the parcel's data, the areas, the length or perimeter and, beside
 * it, the length in space when every vertex has an elevation (`3B uzunluk`, `3B çevre`; docs/adr/0142), the radius,
 * a text, a point's elevation.
 */
export function cardRows(e: Entity, format: Formatter): [string, string][] {
  const rows: [string, string][] = [];
  const a = e.attrs;
  if (a.Ada) rows.push(['Ada', a.Ada]);
  if (a.Mahalle) rows.push(['Mahalle', a.Mahalle]);
  if (a.Nitelik) rows.push(['Nitelik', a.Nitelik]);
  // Registered (tapu) area next to the computed one: the difference is what a surveyor checks.
  const deed = deedAreaText(a);
  if (deed !== null) rows.push(['Tapu alanı', deed]);
  const area = entityArea(e);
  if (area !== null) rows.push([deed !== null ? 'Hesaplanan alan' : 'Alan', format.area(area)]);
  if (e.kind === 'polygon') {
    // A multi-part area's parts, and every part's holes (docs/adr/0143).
    const { parts, holes } = pathCounts(e);
    if (e.parts?.length) rows.push(['Parça', String(parts)]);
    if (holes) rows.push(['Ada (delik)', String(holes)]);
  }
  // A multi-part polyline's parts (docs/adr/0174).
  if (e.kind === 'polyline' && e.parts?.length) rows.push(['Parça', String(e.parts.length + 1)]);
  const length = entityLength(e);
  if (length !== null) rows.push([e.kind === 'polygon' || e.kind === 'circle' ? 'Çevre' : 'Uzunluk', format.length(length)]);
  const space = spaceLength(e);
  if (space) rows.push([space.label, format.length(space.value)]);
  if (e.kind === 'circle' || e.kind === 'arc') rows.push(['Yarıçap', format.length(e.r)]);
  // A multi-line text's breaks as ⏎ (docs/adr/0182): the card's rows are one line each.
  if (e.kind === 'text') rows.push(['Metin', e.text.replaceAll('\n', ' ⏎ ')]);
  if (e.kind === 'point') {
    // A multi-point object's points, and their elevation when they share one (docs/adr/0174).
    if (e.parts?.length) rows.push(['Nokta', String(e.parts.length + 1)]);
    const z = e.z;
    if (z !== undefined && (e.parts ?? []).every((q) => q.z === z)) rows.push(['Kot', format.length(z)]);
  }
  return rows;
}
