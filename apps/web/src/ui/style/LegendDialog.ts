import type { AppContext } from '../../app/context';
import type { CanvasPalette } from '../../render/color';
import { drawSymbolPreview } from '../../render/symbolPreview';
import { LEGEND_PAPER, LEGEND_TEXTS, legendLayers, legendLayout, legendOf, type LegendGroup, type LegendText } from '../../style/legend';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { checkbox } from '../style/designerFields';
import { Thumbs } from './thumbs';

/**
 * Lejant: what the drawing's symbols mean, layer by layer (plan
 * açıklamaları). Layers can be left out; the legend is saved as a PNG
 * drawn on white paper with black ink, whatever the screen theme, ready
 * to be placed on a sheet.
 */

/** The tallest picture a browser draws, in pixels; the desktop keeps the same limit. */
const MAX_PICTURE = 32_767;

export function openLegend(ctx: AppContext): void {
  new LegendDialog(ctx);
}

class LegendDialog {
  private readonly ctx: AppContext;
  private readonly body: HTMLElement;
  private readonly status: HTMLElement;
  private visibleOnly = true;
  private headings = true;
  private readonly left = new Set<string>();
  private groups: LegendGroup[] = [];
  /** Pictures drawn as they scroll into view: a legend can have hundreds of rows. */
  private readonly thumbs: Thumbs;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.body = h('div', { class: 'leg__body' });
    this.thumbs = new Thumbs(ctx, this.body);
    this.status = h('div', { class: 'lsty__status', role: 'status' });
    const save = h('button', { class: 'btn btn--primary', type: 'button' }, icon('export', 16), LEGEND_TEXTS.save);
    save.addEventListener('click', () => void this.savePng());
    const close = h('button', { class: 'btn', type: 'button' }, LEGEND_TEXTS.close);
    const dialog = new Dialog({ title: LEGEND_TEXTS.title, width: 760, className: 'dialog--legend', content: [this.body], footer: [this.status, h('div', { class: 'dialog__foot-spacer' }), close, save], onClose: () => this.thumbs.dispose() });
    close.addEventListener('click', () => dialog.close());
    this.render();
  }

  private compute(): LegendGroup[] {
    const { doc, styles } = this.ctx;
    const L = doc.layers;
    // Top of the layer list first, as the drawing stacks them.
    const layers = legendLayers(
      L.leaves().map((n) => ({ id: n.id, name: n.name, style: n.style, visible: L.isVisible(n.id) })),
      this.visibleOnly,
    );
    return legendOf(layers, { entities: (id) => doc.byLayer(id), symbol: (r) => styles.library.symbol(r), itemName: (id) => styles.library.get(id)?.name });
  }

  private render(): void {
    this.groups = this.compute();
    const opts = h(
      'div',
      { class: 'leg__opts' },
      checkbox(this.visibleOnly, (v) => ((this.visibleOnly = v), this.render()), LEGEND_TEXTS.visibleOnly),
      checkbox(this.headings, (v) => ((this.headings = v), this.render()), LEGEND_TEXTS.headings),
    );
    const groups = this.groups.map((g) => {
      const on = h('input', { type: 'checkbox', checked: !this.left.has(g.layerId), 'aria-label': `${g.layerName} lejantta` });
      on.addEventListener('change', () => (on.checked ? this.left.delete(g.layerId) : this.left.add(g.layerId), this.render()));
      return h(
        'section',
        { class: `leg__group${this.left.has(g.layerId) ? ' leg__group--off' : ''}` },
        h('label', { class: 'leg__head' }, on, h('span', null, g.layerName), h('span', { class: 'smgr__count' }, String(g.entries.length))),
        g.entries.map((e) => {
          const c = e.symbol ? this.thumbs.canvas(e.symbol, 56, 32) : h('canvas', { width: '56', height: '32', style: 'width:56px;height:32px' });
          c.classList.add('leg__pic');
          return h('div', { class: 'leg__row' }, c, h('span', null, e.label));
        }),
      );
    });
    const count = this.groups.filter((g) => !this.left.has(g.layerId)).reduce((s, g) => s + g.entries.length, 0);
    this.status.textContent = LEGEND_TEXTS.rows(count);
    replaceChildren(this.body, opts, groups.length ? groups : h('p', { class: 'lsty__help' }, LEGEND_TEXTS.nothing));
  }

  /** The legend on white paper (2× for print), downloaded as PNG; where everything goes is legendLayout's. */
  private async savePng(): Promise<void> {
    const groups = this.groups.filter((g) => !this.left.has(g.layerId));
    if (!groups.length) return void (this.status.textContent = LEGEND_TEXTS.noRows);
    const paper: CanvasPalette = { ...this.ctx.view.palette, ...LEGEND_PAPER, background: [...LEGEND_PAPER.background] };
    const layout = legendLayout(groups, this.headings, this.ctx.doc.name.value);
    const S = layout.scale;
    // Taller than a browser draws (32 767 px): the canvas would stay empty and nothing be saved.
    if (layout.height * S > MAX_PICTURE) {
      const most = Math.floor((MAX_PICTURE / S - 72) / 30);
      return void (this.status.textContent = `Lejant PNG için çok uzun: ${layout.rows.length} satır, en çok ${most} satır olur. Bazı katmanları dışarıda bırakın ya da başlıkları kapatın.`);
    }
    const c = document.createElement('canvas');
    c.width = layout.width * S;
    c.height = layout.height * S;
    const g = c.getContext('2d')!;
    g.scale(S, S);
    g.fillStyle = layout.background;
    g.fillRect(0, 0, layout.width, layout.height);
    const text = (t: LegendText) => {
      g.font = t.font;
      g.fillStyle = t.color;
      g.fillText(t.text, t.align === 'right' ? t.x - g.measureText(t.text).width : t.x, t.y);
    };
    text(layout.heading);
    text(layout.name);
    const entries = groups.flatMap((grp) => grp.entries);
    let entry = 0;
    for (const row of layout.rows) {
      const symbol = row.kind === 'entry' ? entries[entry++].symbol : null;
      const p = row.picture;
      if (symbol && p) {
        const pic = document.createElement('canvas');
        pic.width = p.w;
        pic.height = p.h;
        drawSymbolPreview(pic, symbol, { palette: paper, library: this.ctx.styles.library, background: layout.background });
        g.drawImage(pic, p.x, p.y, p.w, p.h);
        g.strokeStyle = p.frame;
        g.lineWidth = p.frameWidth;
        g.strokeRect(p.x, p.y, p.w, p.h);
      }
      text(row.label);
    }
    const blob = await new Promise<Blob | null>((ok) => c.toBlob(ok, 'image/png'));
    if (!blob) return;
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = LEGEND_TEXTS.file;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    this.status.textContent = LEGEND_TEXTS.saved;
  }
}
