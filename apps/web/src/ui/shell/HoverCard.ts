import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { ENTITY_KIND_LABEL, entityArea, entityLength, type Entity } from '../../model/entities';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';

const DELAY_MS = 500;

/**
 * Rollover card: resting the mouse on an object (select tool) shows what
 * it is — kind, layer, length or area, parcel data — without selecting it.
 */
export class HoverCard extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private timer = 0;
  private shownId: number | null = null;

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
    this.place();
  }

  private hide(): void {
    this.shownId = null;
    this.el.hidden = true;
  }

  private place(): void {
    const w = this.ctx.view.cursorWorld.value;
    if (!w) return;
    const s = this.ctx.view.camera.worldToScreen(w);
    this.el.style.transform = `translate(${Math.round(s.x + 18)}px, ${Math.round(s.y + 20)}px)`;
  }

  private content(e: Entity): HTMLElement[] {
    const { doc, format, view } = this.ctx;
    const layer = doc.layers.get(e.layerId);
    const rows: [string, string][] = [];
    const a = e.attrs;
    if (a.Ada) rows.push(['Ada', a.Ada]);
    if (a.Mahalle) rows.push(['Mahalle', a.Mahalle]);
    if (a.Nitelik) rows.push(['Nitelik', a.Nitelik]);
    // Registered (tapu) area next to the computed one: the difference is what a surveyor checks.
    const deed = parseFloat(a['Tapu alanı (m²)'] ?? '');
    if (Number.isFinite(deed)) rows.push(['Tapu alanı', format.area(deed)]);
    const area = entityArea(e);
    if (area !== null) rows.push([Number.isFinite(deed) ? 'Hesaplanan alan' : 'Alan', format.area(area)]);
    if (e.kind === 'polygon' && e.holes?.length) rows.push(['Ada (delik)', String(e.holes.length)]);
    const length = entityLength(e);
    if (length !== null) rows.push([e.kind === 'polygon' || e.kind === 'circle' ? 'Çevre' : 'Uzunluk', format.length(length)]);
    if (e.kind === 'circle' || e.kind === 'arc') rows.push(['Yarıçap', format.length(e.r)]);
    if (e.kind === 'text') rows.push(['Metin', e.text]);
    if (e.kind === 'point' && e.z !== undefined) rows.push(['Kot', format.length(e.z)]);
    return [
      h(
        'div',
        { class: 'hover-card__head' },
        h('b', null, a.Parsel ? `Parsel ${a.Parsel}` : e.label ? `${ENTITY_KIND_LABEL[e.kind]} ${e.label}` : ENTITY_KIND_LABEL[e.kind]),
        layer ? h('span', { class: 'hover-card__layer' }, h('span', { class: 'swatch', style: `--swatch:${colorSwatch(e.color ?? layer.style.color, view.palette)}` }), layer.name) : null,
      ),
      ...(rows.length ? [h('dl', { class: 'hover-card__rows' }, rows.map(([k, v]) => [h('dt', null, k), h('dd', { class: 'num' }, v)]))] : []),
    ];
  }
}
