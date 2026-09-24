import type { AppContext } from '../../app/context';
import type { CanvasPalette } from '../../render/color';
import { drawSymbolPreview } from '../../render/symbolPreview';
import { legendOf, type LegendGroup } from '../../style/legend';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { checkbox } from '../style/designerFields';
import { drawNow } from './thumbs';

/**
 * Lejant: what the drawing's symbols mean, layer by layer (plan
 * açıklamaları). Layers can be left out; the legend is saved as a PNG
 * drawn on white paper with black ink, whatever the screen theme, ready
 * to be placed on a sheet.
 */

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

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.body = h('div', { class: 'leg__body' });
    this.status = h('div', { class: 'lsty__status', role: 'status' });
    const save = h('button', { class: 'btn btn--primary', type: 'button' }, icon('export', 16), 'PNG olarak kaydet');
    save.addEventListener('click', () => void this.savePng());
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    const dialog = new Dialog({ title: 'Lejant', width: 760, className: 'dialog--legend', content: [this.body], footer: [this.status, h('div', { class: 'dialog__foot-spacer' }), close, save] });
    close.addEventListener('click', () => dialog.close());
    this.render();
  }

  private compute(): LegendGroup[] {
    const { doc, styles } = this.ctx;
    const L = doc.layers;
    // Top of the layer list first, as the drawing stacks them.
    const layers = L.leaves()
      .filter((n) => !this.visibleOnly || L.isVisible(n.id))
      .map((n) => ({ id: n.id, name: n.name, style: n.style }));
    return legendOf(layers, { entities: (id) => doc.byLayer(id), symbol: (r) => styles.library.symbol(r), itemName: (id) => styles.library.get(id)?.name });
  }

  private render(): void {
    this.groups = this.compute();
    const opts = h(
      'div',
      { class: 'leg__opts' },
      checkbox(this.visibleOnly, (v) => ((this.visibleOnly = v), this.render()), 'Yalnızca görünen katmanlar'),
      checkbox(this.headings, (v) => ((this.headings = v), this.render()), 'Katman adlarını başlık yaz'),
    );
    const groups = this.groups.map((g) => {
      const on = h('input', { type: 'checkbox', checked: !this.left.has(g.layerId), 'aria-label': `${g.layerName} lejantta` });
      on.addEventListener('change', () => (on.checked ? this.left.delete(g.layerId) : this.left.add(g.layerId), this.render()));
      return h(
        'section',
        { class: `leg__group${this.left.has(g.layerId) ? ' leg__group--off' : ''}` },
        h('label', { class: 'leg__head' }, on, h('span', null, g.layerName), h('span', { class: 'smgr__count' }, String(g.entries.length))),
        g.entries.map((e) => {
          const c = h('canvas', { class: 'leg__pic', width: '56', height: '32', style: 'width:56px;height:32px' });
          if (e.symbol) queueMicrotask(() => drawNow(this.ctx, c, e.symbol!));
          return h('div', { class: 'leg__row' }, c, h('span', null, e.label));
        }),
      );
    });
    const count = this.groups.filter((g) => !this.left.has(g.layerId)).reduce((s, g) => s + g.entries.length, 0);
    this.status.textContent = `${count} satır`;
    replaceChildren(this.body, opts, groups.length ? groups : h('p', { class: 'lsty__help' }, 'Lejanta girecek çizilmiş nesne yok.'));
  }

  /** The legend on white paper (2× for print), downloaded as PNG. */
  private async savePng(): Promise<void> {
    const groups = this.groups.filter((g) => !this.left.has(g.layerId));
    if (!groups.length) return void (this.status.textContent = 'Lejantta satır yok.');
    const paper: CanvasPalette = { ...this.ctx.view.palette, background: [1, 1, 1, 1], ink: '#000000', paper: '#FFFFFF', fg: '#111111', fgDim: '#555555' };
    const S = 2;
    const W = 520;
    const ROW = 30;
    const rows = groups.reduce((n, g) => n + g.entries.length + (this.headings ? 1 : 0), 0);
    const H = 56 + rows * ROW + 16;
    const c = document.createElement('canvas');
    c.width = W * S;
    c.height = H * S;
    const g = c.getContext('2d')!;
    g.scale(S, S);
    g.fillStyle = '#FFFFFF';
    g.fillRect(0, 0, W, H);
    g.fillStyle = '#000000';
    g.font = '700 18px Arial, "Liberation Sans", sans-serif';
    g.fillText('LEJANT', 20, 34);
    g.font = '400 11px Arial, "Liberation Sans", sans-serif';
    g.fillStyle = '#555555';
    g.fillText(this.ctx.doc.name.value, W - 20 - g.measureText(this.ctx.doc.name.value).width, 34);
    let y = 56;
    for (const grp of groups) {
      if (this.headings) {
        g.fillStyle = '#000000';
        g.font = '700 12px Arial, "Liberation Sans", sans-serif';
        g.fillText(grp.layerName, 20, y + 19);
        y += ROW;
      }
      for (const e of grp.entries) {
        if (e.symbol) {
          const pic = document.createElement('canvas');
          pic.width = 56;
          pic.height = 24;
          drawSymbolPreview(pic, e.symbol, { palette: paper, library: this.ctx.styles.library, background: '#FFFFFF' });
          g.drawImage(pic, 20, y + 3, 56, 24);
          g.strokeStyle = '#BBBBBB';
          g.lineWidth = 0.5;
          g.strokeRect(20, y + 3, 56, 24);
        }
        g.fillStyle = '#000000';
        g.font = '400 12px Arial, "Liberation Sans", sans-serif';
        g.fillText(e.label, 90, y + 19);
        y += ROW;
      }
    }
    const blob = await new Promise<Blob | null>((ok) => c.toBlob(ok, 'image/png'));
    if (!blob) return;
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'lejant.png';
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    this.status.textContent = 'Lejant PNG olarak kaydedildi (beyaz kâğıt, 2× çözünürlük).';
  }
}
