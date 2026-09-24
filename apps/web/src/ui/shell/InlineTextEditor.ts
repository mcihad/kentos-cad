import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import type { Entity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { layoutDimension, type DimensionLayout } from '../../model/geom/dimension';
import type { TextInputRequest } from '../../viewport/ViewportController';
import { Component } from '../Component';
import { h } from '../dom';

type Session =
  | { kind: 'edit'; id: number }
  | { kind: 'new'; req: TextInputRequest };

/**
 * In-place text field over the drawing, the same size and angle as the
 * text it makes. Two uses: editing an existing text or dimension value
 * (double click in the select tool), and typing new text (the text tool
 * asks through view.requestTextInput). Enter commits, Esc cancels; it
 * follows the drawing when the view pans or zooms.
 */
export class InlineTextEditor extends Component {
  readonly el: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly hint: HTMLElement;
  private readonly ctx: AppContext;
  private session: Session | null = null;
  private place: Placement | null = null;

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.input = h('input', { class: 'inline-text__input', spellcheck: 'false', 'aria-label': 'Yazı' });
    this.hint = h('div', { class: 'inline-text__hint' }, 'Enter: ekle · Esc: vazgeç');
    this.el = h('div', { class: 'inline-text', hidden: true }, this.input, this.hint);
    host.append(this.el);

    this.d.add(ctx.view.events.on('editText', ({ id }) => this.openEdit(id)));
    this.d.add(ctx.view.events.on('textInput', (req) => this.openNew(req)));
    this.d.add(
      listen<KeyboardEvent>(this.input, 'keydown', (e) => {
        e.stopPropagation(); // typing must not trigger tool shortcuts
        if (e.key === 'Enter') {
          e.preventDefault();
          this.close(true);
        } else if (e.key === 'Escape') {
          e.preventDefault();
          this.close(false);
        }
      }),
    );
    this.d.add(listen(this.input, 'blur', () => this.close(true)));
    // Panning or zooming moves the field along with its text.
    this.d.add(ctx.view.camera.changed.subscribe(() => this.position()));
  }

  private openEdit(id: number): void {
    const e = this.ctx.doc.get(id);
    if (!e || (e.kind !== 'text' && e.kind !== 'dimension')) return;
    this.close(true);
    const place = placementOf(e, this.ctx.view);
    if (!place) return;
    this.session = { kind: 'edit', id };
    this.input.value = e.kind === 'text' ? e.text : (e.text ?? '');
    this.input.placeholder = place.measured ?? '';
    this.hint.textContent = 'Enter: kaydet · Esc: vazgeç';
    this.show(place);
    this.ctx.view.setEditing(id);
  }

  private openNew(req: TextInputRequest): void {
    this.close(true);
    this.session = { kind: 'new', req };
    this.input.value = '';
    this.input.placeholder = 'Yazıyı yazın';
    this.hint.textContent = 'Enter: ekle · Esc: vazgeç';
    this.show({ at: req.at, height: req.height, rotation: req.rotation, centered: false });
  }

  private show(place: Placement): void {
    this.place = place;
    this.el.hidden = false;
    this.position();
    this.input.focus();
    if (this.session?.kind !== 'edit') return;
    this.input.select();
    // A double click that opened the editor ends on top of it; its default
    // word selection would replace ours, so select again afterwards.
    setTimeout(() => this.session?.kind === 'edit' && document.activeElement === this.input && this.input.select(), 0);
  }

  private position(): void {
    const p = this.place;
    if (!p || this.el.hidden) return;
    const cam = this.ctx.view.camera;
    const s = cam.worldToScreen(p.at);
    const px = Math.max(13, Math.min(48, p.height * cam.scale));
    Object.assign(this.el.style, {
      left: `${s.x}px`,
      top: `${s.y}px`,
      fontSize: `${px}px`,
      transform: `translate(${p.centered ? '-50%' : '0'}, -85%) rotate(${-p.rotation}deg)`,
      transformOrigin: p.centered ? '50% 85%' : '0 85%',
    });
  }

  private close(commit: boolean): void {
    const session = this.session;
    if (!session) return;
    this.session = null;
    this.place = null;
    this.el.hidden = true;
    const value = this.input.value.trim();
    if (session.kind === 'new') {
      if (commit && value) session.req.commit(value);
      else session.req.cancel();
      return;
    }
    this.ctx.view.setEditing(null);
    const e = this.ctx.doc.get(session.id);
    if (commit && e) {
      if (e.kind === 'text' && value && value !== e.text) this.ctx.doc.update(session.id, { text: value } as Partial<Entity>);
      if (e.kind === 'dimension' && value !== (e.text ?? '')) this.ctx.doc.update(session.id, { text: value || undefined } as Partial<Entity>);
    }
    this.ctx.view.focus();
  }
}

interface Placement {
  at: Vec2;
  height: number;
  rotation: number;
  centered: boolean;
  /** A dimension's own value, shown when its text is cleared. */
  measured?: string;
}

function placementOf(e: Entity, view: { dimensionText(l: DimensionLayout): string }): Placement | null {
  if (e.kind === 'text') return { at: e.p, height: e.height, rotation: e.rotation, centered: false };
  if (e.kind === 'dimension') {
    const l = layoutDimension(e);
    return l ? { at: l.textAt, height: e.height, rotation: l.rotation, centered: true, measured: view.dimensionText(l) } : null;
  }
  return null;
}
