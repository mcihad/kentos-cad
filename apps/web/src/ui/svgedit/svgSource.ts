import { sourceText } from '../../style/svg/exportSvg';
import { importSummary } from '../../style/svg/importSvg';
import { h } from '../dom';
import { icon } from '../icons';
import { readSvg, xmlError } from './readSvg';
import type { FileHost } from './svgFile';

/**
 * The XML source view (Inkscape's XML editor, as text): the drawing's SVG
 * below the canvas, one element per line with its id. The selected
 * shapes' elements are highlighted, and clicking an element selects its
 * shape. Edited text is read back with Uygula (Ctrl+Enter) as one undo
 * step; an XML error names its line and column and puts the caret there.
 * While the text is edited the view no longer follows the drawing.
 */

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export class SourcePanel {
  readonly el: HTMLElement;
  open = false;
  private readonly host: FileHost;
  private readonly area: HTMLTextAreaElement;
  private readonly back: HTMLElement;
  private readonly gutter: HTMLElement;
  private readonly note: HTMLElement;
  private readonly applyBtn: HTMLButtonElement;
  private readonly revertBtn: HTMLButtonElement;
  private generated = '';
  private spans = new Map<string, [number, number]>();
  private edited = false;
  private shownSel = '';
  private height = 0;

  constructor(host: FileHost, onClose: () => void) {
    this.host = host;
    this.area = h('textarea', { class: 'svgs__text mono', spellcheck: 'false', wrap: 'off', 'aria-label': 'SVG kaynağı', dataset: { escape: 'local' } });
    this.back = h('pre', { class: 'svgs__back mono', 'aria-hidden': 'true' });
    this.gutter = h('pre', { class: 'svgs__gutter mono', 'aria-hidden': 'true' });
    this.note = h('span', { class: 'svgs__note', role: 'status' });
    this.applyBtn = h('button', { class: 'btn btn--small btn--primary', type: 'button', disabled: true, title: 'Metni çizime uygular (Ctrl+Enter); Ctrl+Z geri alır' }, icon('check', 14), 'Uygula');
    this.revertBtn = h('button', { class: 'btn btn--small', type: 'button', disabled: true, title: 'Düzenlemeyi bırakır, çizimin kaynağına döner' }, 'Geri al');
    const copy = h('button', { class: 'ibtn', type: 'button', title: 'Kaynağı panoya kopyala', 'aria-label': 'Kaynağı panoya kopyala' }, icon('copy', 14));
    const close = h('button', { class: 'ibtn', type: 'button', title: 'Kaynağı kapat (Ctrl+Shift+X)', 'aria-label': 'Kaynağı kapat' }, icon('close', 14));
    const grip = h('div', { class: 'svgs__grip', title: 'Sürükleyerek yüksekliği değiştirin' });
    this.el = h(
      'section',
      { class: 'svgs', hidden: true, 'aria-label': 'SVG kaynağı' },
      grip,
      h('div', { class: 'svgs__head' }, h('span', { class: 'sdes__title' }, 'SVG kaynağı'), this.note, h('div', { class: 'dialog__foot-spacer' }), this.revertBtn, this.applyBtn, copy, close),
      h('div', { class: 'svgs__body' }, this.gutter, h('div', { class: 'svgs__code' }, this.back, this.area)),
    );
    this.applyBtn.addEventListener('click', () => this.apply());
    this.revertBtn.addEventListener('click', () => this.revert());
    copy.addEventListener('click', () => void navigator.clipboard.writeText(this.area.value).then(() => this.say('Panoya kopyalandı.'), () => this.say('Tarayıcı panoya yazmaya izin vermedi.', true)));
    close.addEventListener('click', onClose);
    this.area.addEventListener('input', () => {
      this.edited = this.area.value !== this.generated;
      this.applyBtn.disabled = this.revertBtn.disabled = !this.edited;
      this.say(this.edited ? 'Düzenlendi: Uygula (Ctrl+Enter) ya da Geri al.' : '');
      this.paint();
    });
    this.area.addEventListener('scroll', () => this.sync());
    this.area.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        this.apply();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        this.host.canvas.el.focus();
      } else if (e.key === 'Tab' && !e.shiftKey) {
        e.preventDefault();
        this.area.setRangeText('  ', this.area.selectionStart, this.area.selectionEnd, 'end');
        this.area.dispatchEvent(new Event('input'));
      }
    });
    // A click on an element selects its shape (while the text follows the drawing).
    this.area.addEventListener('click', () => {
      if (this.edited) return;
      const at = this.area.selectionStart;
      for (const [id, [a, b]] of this.spans)
        if (at >= a && at <= b) {
          this.shownSel = id;
          this.host.select([id]);
          return;
        }
    });
    grip.addEventListener('pointerdown', (e) => {
      grip.setPointerCapture(e.pointerId);
      const y0 = e.clientY;
      const h0 = this.el.getBoundingClientRect().height;
      const max = (this.el.parentElement?.getBoundingClientRect().height ?? 600) - 120;
      const move = (ev: PointerEvent) => {
        this.height = Math.max(120, Math.min(max, h0 + y0 - ev.clientY));
        this.el.style.flexBasis = `${this.height}px`;
      };
      const up = () => {
        grip.removeEventListener('pointermove', move);
        grip.removeEventListener('pointerup', up);
        this.host.canvas.render();
      };
      grip.addEventListener('pointermove', move);
      grip.addEventListener('pointerup', up);
    });
  }

  toggle(): void {
    this.open = !this.open;
    this.el.hidden = !this.open;
    if (this.open) this.update();
  }

  /** Follows the drawing and its selection (unless the text is being edited). */
  update(): void {
    if (!this.open) return;
    if (!this.edited) {
      const { text, spans } = sourceText(this.host.doc);
      this.spans = spans;
      if (text !== this.generated) {
        this.generated = text;
        this.area.value = text;
      }
    }
    this.paint();
  }

  /** Line numbers and the highlight of the selected shapes' elements. */
  private paint(): void {
    const text = this.area.value;
    const lines = text.split('\n').length;
    this.gutter.textContent = Array.from({ length: lines }, (_, i) => i + 1).join('\n');
    const ranges = this.edited ? [] : [...this.host.selection].map((id) => this.spans.get(id)).filter((r): r is [number, number] => !!r).sort((a, b) => a[0] - b[0]);
    let html = '';
    let at = 0;
    for (const [a, b] of ranges) {
      html += `${esc(text.slice(at, a))}<mark>${esc(text.slice(a, b))}</mark>`;
      at = b;
    }
    this.back.innerHTML = `${html}${esc(text.slice(at))}\n`;
    // A new selection scrolls its first element into view.
    const sel = [...this.host.selection].sort().join(',');
    if (ranges.length && sel !== this.shownSel && document.activeElement !== this.area) {
      const line = text.slice(0, ranges[0][0]).split('\n').length - 1;
      const lh = parseFloat(getComputedStyle(this.area).lineHeight) || 16;
      const top = line * lh;
      if (top < this.area.scrollTop || top > this.area.scrollTop + this.area.clientHeight - 2 * lh) this.area.scrollTop = Math.max(0, top - this.area.clientHeight / 3);
    }
    this.shownSel = sel;
    this.sync();
  }

  private sync(): void {
    this.back.scrollTop = this.area.scrollTop;
    this.back.scrollLeft = this.area.scrollLeft;
    this.gutter.scrollTop = this.area.scrollTop;
  }

  private say(text: string, error = false): void {
    this.note.textContent = text;
    this.note.classList.toggle('svgs__note--error', error);
  }

  private apply(): void {
    const text = this.area.value;
    const err = xmlError(text);
    if (err) {
      // The caret goes where the parser stopped.
      const lines = text.split('\n');
      const offset = lines.slice(0, err.line - 1).reduce((s, l) => s + l.length + 1, 0) + Math.max(0, err.column - 1);
      this.area.focus();
      this.area.setSelectionRange(offset, Math.min(text.length, offset + 1));
      return this.say(`Satır ${err.line}, sütun ${err.column}: ${err.message}`, true);
    }
    const read = readSvg(text, { editor: true, symbolColor: null });
    if ('error' in read) return this.say(read.error, true);
    const host = this.host;
    const keep = [...host.selection].filter((id) => read.doc.shapes.some((s) => s.id === id));
    this.edited = false;
    host.change('source', () => (host.doc = read.doc));
    host.select(keep);
    const { done, lost } = importSummary(read.report);
    this.applyBtn.disabled = this.revertBtn.disabled = true;
    this.say(`Uygulandı: ${done}${lost.length ? `; ${lost.join(', ')}` : ''}. Ctrl+Z geri alır.`, lost.length > 0);
    this.update();
  }

  private revert(): void {
    this.edited = false;
    this.generated = '';
    this.applyBtn.disabled = this.revertBtn.disabled = true;
    this.say('');
    this.update();
  }
}
