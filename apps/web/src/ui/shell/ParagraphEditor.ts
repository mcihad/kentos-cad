import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { isParagraph, type TextAlign, type TextEntity, type TextRun } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { letterAt, textLayout, textLines, textRunsRetext, textRunsToggle, trimmedParagraph, type ParagraphText, type RunToggle } from '../../model/paragraph';
import type { ParagraphInputRequest } from '../../viewport/ViewportController';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { setGeometry } from '../properties/write';
import { DRAW_COLORS } from '../ribbon/fields';
import { tooltip } from '../widgets/tooltip';

type Session = { kind: 'edit'; id: number } | { kind: 'new'; req: ParagraphInputRequest };

/** What Simge ▾ puts at the cursor (the desktop's `SYMBOLS`). */
export const SYMBOLS: readonly (readonly [string, string])[] = [
  ['°', 'Derece'],
  ['±', 'Artı eksi'],
  ['⌀', 'Çap'],
  ['²', 'Kare'],
  ['³', 'Küp'],
  ['‰', 'Binde'],
  ['×', 'Çarpı'],
  ['≤', 'Küçük eşit'],
  ['≥', 'Büyük eşit'],
];

/** The text being written: where it stands and how it looks (its words and runs apart). */
type Place = Omit<ParagraphText, 'text' | 'runs' | 'font'> & { align?: TextAlign };

/**
 * The paragraph editor over the drawing (docs/adr/0182 §4; the desktop's `paragraph_editor.rs`), for two things:
 *
 * - Çok satırlı yazı: the tool asks for it at the box (`view.requestParagraphInput`); Tamam gives the text and its
 *   letter formats back, Vazgeç drops them;
 * - a double click on a multi-line text (a line break, a box, a spacing or letter formats) edits it in place, its drawn
 *   text hidden meanwhile; Tamam writes it as Öznitelikler does (`cad.entities.edit`, the step “Değiştir”).
 *
 * Enter starts a new line, Ctrl+Enter or Tamam keeps the text, Esc or Vazgeç drops it, a press on the drawing keeps
 * it. Above the text: Kalın, Eğik, Altı çizili, Üst simge and Alt simge over the chosen letters (none chosen: the word
 * at the cursor), Renk ▾ and Simge ▾. The drawing shows the text as it will be, lines, box and all.
 */
export class ParagraphEditor extends Component {
  readonly el: HTMLElement;
  private readonly area: HTMLTextAreaElement;
  private readonly menu: HTMLElement;
  private readonly ctx: AppContext;
  private session: Session | null = null;
  private place: Place | null = null;
  private text = '';
  private runs: TextRun[] = [];
  private open: 'color' | 'symbol' | null = null;

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    const tool = (name: string, title: string, t: RunToggle) => {
      const b = h('button', { class: 'ibtn paragraph-editor__tool', type: 'button', 'aria-label': title, 'data-format': String(t) }, icon(name, 16));
      this.d.add(tooltip(b, () => ({ title }), 'top'));
      this.d.add(listen(b, 'pointerdown', (e) => e.preventDefault()));
      this.d.add(listen(b, 'click', () => this.format(t)));
      return b;
    };
    const menuButton = (name: string, title: string, which: 'color' | 'symbol') => {
      const b = h('button', { class: 'ibtn paragraph-editor__tool paragraph-editor__menu-button', type: 'button', 'aria-label': title, 'data-menu': which }, icon(name, 16), icon('chevronDown', 10));
      this.d.add(tooltip(b, () => ({ title }), 'top'));
      this.d.add(listen(b, 'pointerdown', (e) => e.preventDefault()));
      this.d.add(listen(b, 'click', () => this.toggleMenu(which)));
      return b;
    };
    const tools = h(
      'div',
      { class: 'paragraph-editor__tools' },
      tool('textBold', 'Kalın (seçili harfler; seçim yoksa imlecin sözcüğü)', 'bold'),
      tool('textItalic', 'Eğik', 'italic'),
      tool('textUnderline', 'Altı çizili', 'underline'),
      tool('textSuperscript', 'Üst simge (m²)', 'super'),
      tool('textSubscript', 'Alt simge (H₂O)', 'sub'),
      h('span', { class: 'paragraph-editor__sep' }),
      menuButton('textColor', 'Renk: seçili harflerin rengi', 'color'),
      menuButton('textSymbol', 'Simge: imlecin yerine bir işaret', 'symbol'),
    );
    this.area = h('textarea', { class: 'paragraph-editor__text', spellcheck: 'false', rows: '3', placeholder: 'Yazıyı yazın; Enter yeni satır', 'aria-label': 'Çok satırlı yazı' });
    this.menu = h('div', { class: 'paragraph-editor__menu', hidden: true });
    const cancel = h('button', { class: 'btn btn--small', type: 'button', 'data-close': 'drop' }, 'Vazgeç');
    const keep = h('button', { class: 'btn btn--small btn--primary', type: 'button', 'data-close': 'keep' }, 'Tamam');
    this.d.add(listen(cancel, 'click', () => this.close(false)));
    this.d.add(listen(keep, 'click', () => this.close(true)));
    const foot = h('div', { class: 'paragraph-editor__foot' }, h('span', { class: 'paragraph-editor__hint' }, 'Ctrl+Enter: ekle · Esc: vazgeç'), cancel, keep);
    this.el = h('div', { class: 'paragraph-editor', hidden: true }, tools, this.area, this.menu, foot);
    host.append(this.el);

    this.d.add(ctx.view.events.on('paragraphInput', (req) => this.openNew(req)));
    this.d.add(ctx.view.events.on('editText', ({ id }) => this.openEdit(id)));
    this.d.add(listen(this.area, 'input', () => this.edited()));
    this.d.add(
      listen<KeyboardEvent>(this.el, 'keydown', (e) => {
        e.stopPropagation(); // typing must not trigger tool shortcuts
        if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
          e.preventDefault();
          this.close(true);
        } else if (e.key === 'Escape') {
          e.preventDefault();
          this.close(false);
        }
      }),
    );
    // A press on the drawing keeps the text, as the desktop's.
    this.d.add(
      listen<PointerEvent>(document, 'pointerdown', (e) => {
        if (this.session && !this.el.contains(e.target as Node)) this.close(true);
      }, { capture: true }),
    );
    // Panning or zooming moves the editor along with its box.
    this.d.add(ctx.view.camera.changed.subscribe(() => this.position()));
  }

  private openNew(req: ParagraphInputRequest): void {
    this.close(true);
    this.session = { kind: 'new', req };
    this.show({ p: req.at, height: req.height, rotation: req.rotation, align: 'topLeft', boxWidth: req.boxWidth, lineSpacing: req.lineSpacing, mask: req.mask }, '', []);
  }

  private openEdit(id: number): void {
    const e = this.ctx.doc.get(id);
    if (!e || e.kind !== 'text' || !isParagraph(e)) return;
    this.close(true);
    this.session = { kind: 'edit', id };
    this.show(placeOf(e), e.text, e.runs ?? []);
    this.ctx.view.setEditing(id);
  }

  private show(place: Place, text: string, runs: readonly TextRun[]): void {
    this.place = place;
    this.text = text;
    this.runs = [...runs];
    this.area.value = text;
    this.setMenu(null);
    this.el.hidden = false;
    this.position();
    this.preview();
    this.area.focus();
  }

  /** The text changed: its runs follow its letters; the drawing shows it. */
  private edited(): void {
    const text = this.area.value;
    this.runs = textRunsRetext(this.runs, this.text, text);
    this.text = text;
    this.preview();
    this.position();
  }

  /** A format over the chosen letters: the selection, else the word at the cursor, else nothing (said). */
  private format(t: RunToggle): void {
    const range = this.chosen();
    if (!range) {
      this.ctx.log.info('Biçim için harfleri seçin ya da imleci bir sözcüğe koyun.');
      return;
    }
    this.runs = textRunsToggle(this.runs, [...this.text].length, range[0], range[1], t);
    this.setMenu(null);
    this.preview();
    this.area.focus();
  }

  private chosen(): [number, number] | null {
    const a = letterAt(this.text, this.area.selectionStart);
    const b = letterAt(this.text, this.area.selectionEnd);
    if (a !== b) return [Math.min(a, b), Math.max(a, b)];
    const letters = [...this.text];
    const word = (i: number) => i >= 0 && i < letters.length && !/\s/.test(letters[i]);
    let [s, e] = [a, a];
    while (s > 0 && word(s - 1)) s--;
    while (word(e)) e++;
    return s < e ? [s, e] : null;
  }

  private toggleMenu(which: 'color' | 'symbol'): void {
    this.setMenu(this.open === which ? null : which);
  }

  private setMenu(which: 'color' | 'symbol' | null): void {
    this.open = which;
    this.menu.hidden = which === null;
    this.menu.replaceChildren();
    if (which === 'color') {
      const own = h('button', { class: 'btn btn--ghost btn--small', type: 'button', 'data-color': '' }, 'Yazının rengi');
      this.d.add(listen(own, 'pointerdown', (e) => e.preventDefault()));
      this.d.add(listen(own, 'click', () => this.format({ color: null })));
      this.menu.append(own);
      for (const c of DRAW_COLORS) {
        const swatch = h('button', { class: 'paragraph-editor__swatch', type: 'button', 'aria-label': c.name, 'data-color': c.value });
        swatch.style.background = c.value === 'ink' ? 'var(--c-text)' : c.value;
        this.d.add(tooltip(swatch, () => ({ title: c.name }), 'bottom'));
        this.d.add(listen(swatch, 'pointerdown', (e) => e.preventDefault()));
        this.d.add(listen(swatch, 'click', () => this.format({ color: c.value })));
        this.menu.append(swatch);
      }
    } else if (which === 'symbol') {
      for (const [ch, name] of SYMBOLS) {
        const b = h('button', { class: 'paragraph-editor__symbol', type: 'button', 'aria-label': name }, ch);
        this.d.add(tooltip(b, () => ({ title: name }), 'bottom'));
        this.d.add(listen(b, 'pointerdown', (e) => e.preventDefault()));
        this.d.add(listen(b, 'click', () => this.insert(ch)));
        this.menu.append(b);
      }
    }
    this.position();
  }

  /** A sign at the cursor, as if typed. */
  private insert(ch: string): void {
    const [s, e] = [this.area.selectionStart, this.area.selectionEnd];
    this.area.setRangeText(ch, s, e, 'end');
    this.edited();
    this.setMenu(null);
    this.area.focus();
  }

  /** The drawing shows the text as it will be (the core's label records). */
  private preview(): void {
    const p = this.place;
    if (!p) return;
    const runs = this.runs;
    const font = this.ctx.doc.settings.drawingFont.value;
    const records = textLines({ ...p, text: this.text, ...(runs.length && { runs }), font });
    this.ctx.view.setParagraphPreview({ records, text: this.text, runs });
  }

  /**
   * Beside the text, never over it: its left at the box's left, above the box's top when there is room, else under
   * the text's last line (the drawing shows the text as it will be).
   */
  private position(): void {
    const p = this.place;
    if (!p || this.el.hidden) return;
    const cam = this.ctx.view.camera;
    const font = this.ctx.doc.settings.drawingFont.value;
    const laid = textLayout({ ...p, text: this.text || ' ', ...(this.runs.length && { runs: this.runs }), font });
    const r = (p.rotation * Math.PI) / 180;
    const [c, s] = [Math.cos(r), Math.sin(r)];
    // The origin: the point less its shares along the box and up from the first baseline; the top a height up.
    const [along, up] = laid.shares;
    const o = { x: p.p.x - c * along + s * up, y: p.p.y - s * along - c * up };
    const top: Vec2 = { x: o.x - s * p.height, y: o.y + c * p.height };
    const at = cam.worldToScreen(top);
    const width = Math.min(640, Math.max(380, (p.boxWidth ?? 0) * cam.scale || 400));
    const room = this.ctx.view.clientRect();
    this.el.style.width = `${width}px`;
    const height = this.el.offsetHeight;
    // The text's depth under its top: its lines a pitch apart, the last one's descent.
    const depth = (Math.max(1, laid.lines.length) - 1) * laid.pitch * cam.scale + p.height * 1.4 * cam.scale;
    const above = at.y - height - 8;
    const y = above >= 4 ? above : Math.min(at.y + depth + 8, room.height - height - 4);
    Object.assign(this.el.style, {
      left: `${Math.max(4, Math.min(at.x, room.width - width - 4))}px`,
      top: `${Math.max(4, y)}px`,
    });
  }

  private close(commit: boolean): void {
    const session = this.session;
    if (!session) return;
    this.session = null;
    this.place = null;
    this.el.hidden = true;
    this.setMenu(null);
    this.ctx.view.setParagraphPreview(null);
    if (session.kind === 'new') {
      if (commit) session.req.commit(this.text, this.runs);
      else session.req.cancel();
      return;
    }
    this.ctx.view.setEditing(null);
    const e = this.ctx.doc.get(session.id);
    if (commit && e?.kind === 'text') {
      const { text, runs } = trimmedParagraph(this.text, this.runs);
      const same = text === e.text && JSON.stringify(runs) === JSON.stringify(e.runs ?? []);
      // Written as Öznitelikler writes it (`cad.entities.edit`, the step “Değiştir”).
      if (text && !same) setGeometry(this.ctx, e, { text, runs: runs.length ? runs : undefined });
    }
    this.ctx.view.focus();
  }
}

/** Where a text stands and how it looks, its words and runs apart. */
function placeOf(t: TextEntity): Place {
  return {
    p: t.p,
    height: t.height,
    rotation: t.rotation,
    ...(t.align && { align: t.align }),
    ...(t.widthFactor !== undefined && { widthFactor: t.widthFactor }),
    ...(t.boxWidth !== undefined && { boxWidth: t.boxWidth }),
    ...(t.lineSpacing !== undefined && { lineSpacing: t.lineSpacing }),
    ...(t.mask && { mask: true }),
  };
}
