import { attachmentViews, openAttachment, openViewer } from '../../app/attachments';
import type { AppContext } from '../../app/context';
import { objectActions, objectCard, runAction } from '../../app/objectActions';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { besidePointer } from '../widgets/placeBeside';

/** The card's class, which the traces and the layout check find it by. */
export const INFO_CARD_CLASS = 'info-card';

/**
 * The Bilgi tool's card (docs/adr/0225 §5; the desktop's `info_card.rs`): the object `ToolManager.infoCard` names, beside
 * the drawing's point it was clicked at and following the view, never outside the drawing. Its layer's card's title,
 * rows and text; the object's photos (a click opens the viewer) and other attachments; its layer's actions as buttons.
 * A broken template's error is said under the title. × closes it, as Esc and a click on nothing do.
 */
export class InfoCard extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.el = h('div', { class: INFO_CARD_CLASS, hidden: true, role: 'dialog', 'aria-label': 'Bilgi kartı' });
    host.append(this.el);
    const { tools, doc, view } = ctx;
    this.d.add(watchAll([tools.infoCard, doc.styles], () => this.render()));
    // The object's or its layer's change (an attachment, a renamed attribute, the card) shows at once.
    this.d.add(doc.events.on('changed', () => this.render()));
    this.d.add(view.camera.changed.subscribe(() => this.place()));
  }

  private render(): void {
    const shown = this.ctx.tools.infoCard.value;
    const e = shown ? this.ctx.doc.get(shown.id) : undefined;
    if (!shown || !e) {
      this.el.hidden = true;
      return;
    }
    const card = objectCard(this.ctx, e);
    const close = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Kapat', title: 'Kapat (Esc)' }, icon('close', 14));
    close.addEventListener('click', () => this.ctx.tools.infoCard.set(null));
    const views = card.attachments ? attachmentViews(this.ctx, e) : [];
    const photos = views.filter((v) => v.kind === 'photo' && v.src);
    const others = views.filter((v) => !(v.kind === 'photo' && v.src));
    const actions = card.actions ? objectActions(this.ctx, e) : [];
    replaceChildren(
      this.el,
      h('div', { class: 'info-card__head' }, h('div', { class: 'info-card__title' }, card.title), close),
      card.error ? h('div', { class: 'props__note props__note--warn' }, card.error) : null,
      card.rows.length ? h('dl', { class: 'info-card__rows' }, card.rows.map(([k, v]) => [h('dt', null, k), h('dd', null, v || '—')])) : null,
      card.text ? h('div', { class: 'info-card__text' }, card.text) : null,
      photos.length
        ? h(
            'div',
            { class: 'info-card__photos' },
            photos.map((v) => {
              const img = h('img', { src: v.src, alt: v.attachment.name, title: v.attachment.name, draggable: 'false' });
              img.addEventListener('click', () => openViewer(this.ctx, e, v.index));
              return img;
            }),
          )
        : null,
      others.length
        ? h(
            'div',
            { class: 'info-card__links' },
            others.map((v) => {
              const b = h('button', { class: 'btn btn--small btn--quiet', type: 'button', title: v.source }, icon(v.kind === 'photo' ? 'photo' : v.kind === 'document' ? 'document' : 'attachmentLink', 14), h('span', null, v.attachment.name));
              b.addEventListener('click', () => openAttachment(this.ctx, e, v.index));
              return b;
            }),
          )
        : null,
      actions.length
        ? h(
            'div',
            { class: 'info-card__actions' },
            actions.map((a) => {
              const b = h('button', { class: 'btn btn--small', type: 'button', title: a.error ?? a.target ?? '', disabled: a.error !== null }, icon(a.kind === 'file' ? 'document' : 'externalOpen', 14), h('span', null, a.name));
              b.addEventListener('click', () => runAction(this.ctx, a));
              return b;
            }),
          )
        : null,
    );
    this.el.hidden = false;
    this.place();
  }

  /** Right of and below the clicked point; left of it or above it near the drawing's edges, never outside. */
  private place(): void {
    const shown = this.ctx.tools.infoCard.value;
    if (!shown || this.el.hidden) return;
    const camera = this.ctx.view.camera;
    const at = besidePointer(camera.worldToScreen(shown.at), { w: this.el.offsetWidth, h: this.el.offsetHeight }, { w: camera.width, h: camera.height });
    this.el.style.transform = `translate(${at.x}px, ${at.y}px)`;
  }
}
