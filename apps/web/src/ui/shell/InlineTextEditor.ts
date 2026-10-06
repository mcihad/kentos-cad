import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { isParagraph, textAlignShares, type Entity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { layoutDimension, type DimensionLayout } from '../../model/geom/dimension';
import { leaderLayout } from '../../model/geom/leader';
import type { TextInputRequest } from '../../viewport/ViewportController';
import { Component } from '../Component';
import { h } from '../dom';
import { setGeometry } from '../properties/write';

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
          this.close(true, true);
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
    if (!e || (e.kind !== 'text' && e.kind !== 'dimension' && e.kind !== 'leader')) return;
    // A multi-line text is the paragraph editor's (docs/adr/0182 §4).
    if (e.kind === 'text' && isParagraph(e)) return;
    this.close(true);
    const place = placementOf(e, this.ctx.view);
    if (!place) return;
    this.session = { kind: 'edit', id };
    this.input.value = e.kind === 'text' ? e.text : (e.text ?? '');
    // A leader without a note gets one typed here (docs/adr/0146 §7).
    this.input.placeholder = place.measured ?? (e.kind === 'leader' ? 'Notu yazın' : '');
    this.hint.textContent = 'Enter: kaydet · Esc: vazgeç';
    this.show(place);
    this.ctx.view.setEditing(id);
  }

  private openNew(req: TextInputRequest): void {
    this.close(true);
    this.session = { kind: 'new', req };
    // Artır's next number, selected: typing replaces it, Enter keeps it (docs/adr/0145 §6).
    this.input.value = req.initial ?? '';
    this.input.placeholder = req.placeholder ?? 'Yazıyı yazın';
    this.hint.textContent = req.hint ?? 'Enter: ekle · Esc: vazgeç';
    // The field stands where the text will, by its alignment and width factor.
    const [along, up] = textAlignShares(req.align ?? null);
    this.show({ at: req.at, height: req.height, rotation: req.rotation, along, up, widthFactor: req.widthFactor ?? 1 });
  }

  private show(place: Placement): void {
    this.place = place;
    this.el.hidden = false;
    this.position();
    this.input.focus();
    // A text being edited, or Artır's next number: selected, so typing replaces it.
    if (this.session?.kind !== 'edit' && !this.input.value) return;
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
    // The field's baseline sits at 85% of its height; its point is `along` of its width and `up` heights over the
    // baseline (the text's alignment, docs/adr/0145), and it turns and widens about that point.
    const up = p.up * px;
    Object.assign(this.el.style, {
      left: `${s.x}px`,
      top: `${s.y}px`,
      fontSize: `${px}px`,
      transform: `translate(${-p.along * 100}%, calc(-85% + ${up}px)) rotate(${-p.rotation}deg) scaleX(${p.widthFactor})`,
      transformOrigin: `${p.along * 100}% calc(85% - ${up}px)`,
    });
  }

  /** `enter`: closed by Enter, not by a click elsewhere (an empty field's Enter may mean something, docs/adr/0146 §7). */
  private close(commit: boolean, enter = false): void {
    const session = this.session;
    if (!session) return;
    this.session = null;
    this.place = null;
    this.el.hidden = true;
    const value = this.input.value.trim();
    if (session.kind === 'new') {
      if (commit && value) session.req.commit(value);
      else if (commit && enter && session.req.empty) session.req.empty();
      else session.req.cancel();
      return;
    }
    this.ctx.view.setEditing(null);
    const e = this.ctx.doc.get(session.id);
    // Written as Öznitelikler writes it (`cad.entities.edit`, the step “Değiştir”); a cleared dimension text shows its value again.
    if (commit && e) {
      if (e.kind === 'text' && value && value !== e.text) setGeometry(this.ctx, e, { text: value });
      if (e.kind === 'dimension' && value !== (e.text ?? '')) setGeometry(this.ctx, e, { text: value || undefined });
      // A leader's note; emptied, the arrow alone (docs/adr/0146 §7).
      if (e.kind === 'leader' && value !== (e.text ?? '')) setGeometry(this.ctx, e, { text: value || undefined });
    }
    this.ctx.view.focus();
  }
}

interface Placement {
  at: Vec2;
  height: number;
  rotation: number;
  /** Where `at` is on the text: shares along its width and of its height over the baseline (docs/adr/0145). */
  along: number;
  up: number;
  widthFactor: number;
  /** A dimension's own value, shown when its text is cleared. */
  measured?: string;
}

function placementOf(e: Entity, view: { dimensionText(l: DimensionLayout): string }): Placement | null {
  if (e.kind === 'text') {
    const [along, up] = textAlignShares(e.align ?? null);
    return { at: e.p, height: e.height, rotation: e.rotation, along, up, widthFactor: e.widthFactor ?? 1 };
  }
  if (e.kind === 'dimension') {
    const l = layoutDimension(e);
    return l ? { at: l.textAt, height: e.height, rotation: l.rotation, along: 0.5, up: 0, widthFactor: 1, measured: view.dimensionText(l) } : null;
  }
  // A leader's note stands past its landing, on the side its last segment goes (docs/adr/0146 §2); one without a
  // note is laid out as if it had one, so the field opens where its note will be.
  if (e.kind === 'leader') {
    const l = leaderLayout({ ...e, text: e.text ?? 'Not' });
    if (!l?.notePoint) return null;
    const [along, up] = textAlignShares(l.noteAlign ?? null);
    return { at: l.notePoint, height: e.height, rotation: e.rotation, along, up, widthFactor: 1 };
  }
  return null;
}
