import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { DxfWriteInput } from '../../contracts/generated/DxfWriteInput';
import type { DxfWriteLayer } from '../../contracts/generated/DxfWriteLayer';
import { formats } from '../../io/client';
import type { Entity } from '../../model/entities';
import { layoutDimension } from '../../model/geom/dimension';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { exportName, field, kindCounts, reportText, summaryLine } from './common';
import { saveExport } from './save';
import { SCOPE_LABEL, scopeEntities, type ExportScope } from './scope';

/**
 * DXF dışa aktar: the objects of the selection, of the visible layers or of
 * the whole drawing, with their layers, as an AutoCAD 2007 DXF (UTF-8,
 * metres; crates/shared/formats dxf/writer). Coordinates are written as the
 * shortest decimal that reads back to the same float64 (CLAUDE.md §23).
 * What DXF cannot hold (labels, attributes, symbols, theme colours, holes)
 * rides along as KentOS data that KentOS reads back; what changes on the
 * way is said before writing (the summary) and after (the writer's report,
 * in the log). An export is not a save: the drawing stays unsaved if it was.
 */
export function openDxfExport(ctx: AppContext): void {
  new DxfExportDialog(ctx);
}

const DXF: FileKind = { description: 'AutoCAD DXF', accept: { 'application/dxf': ['.dxf'] } };
/** Theme colours DXF has no colour for (ink is DXF colour 7 exactly). */
const THEMED = new Set(['fg', 'fg-dim', 'paper']);

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

class DxfExportDialog {
  private readonly ctx: AppContext;
  private scope: ExportScope;
  /** Layers the user left out. */
  private readonly excluded = new Set<string>();
  private writing = false;
  private readonly form = h('div', { class: 'io-row' });
  private readonly layersHost = h('div', { class: 'io-table-wrap' });
  private readonly summary = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Dışa aktar…');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.scope = scopeEntities(ctx, 'selection').length ? 'selection' : 'visible';
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'DXF dışa aktar',
      width: 820,
      className: 'dialog--io',
      content: [this.form, this.layersHost, this.summary],
      footer: [this.status, cancel, this.primary],
    });
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.run());
    this.render();
  }

  /** The scope's objects by layer (the tree's order; an object whose layer is gone goes to DXF layer 0). */
  private byLayer(scope: ExportScope): Map<string, Entity[]> {
    const out = new Map<string, Entity[]>(this.ctx.doc.layers.leaves().map((l) => [l.id, []]));
    for (const e of scopeEntities(this.ctx, scope)) {
      const list = out.get(e.layerId);
      if (list) list.push(e);
      else out.set(e.layerId, [e]);
    }
    for (const [id, list] of out) if (!list.length) out.delete(id);
    return out;
  }

  /** The objects that will be written: the scope's, less the layers left out. */
  private chosen(): Map<string, Entity[]> {
    const m = this.byLayer(this.scope);
    for (const id of this.excluded) m.delete(id);
    return m;
  }

  private render(): void {
    const { ctx } = this;
    const count = (s: ExportScope) => scopeEntities(ctx, s).length;
    const counts = { selection: count('selection'), visible: count('visible'), all: count('all') };
    const scope = segmented<ExportScope>({
      label: 'Yazılacak nesneler',
      options: (['selection', 'visible', 'all'] as ExportScope[]).map((s) => ({ value: s, label: `${SCOPE_LABEL[s]} (${counts[s]})`, disabled: !counts[s] })),
      value: this.scope,
      onChange: (s) => ((this.scope = s), this.render()),
    });
    replaceChildren(this.form, field('Yazılacak nesneler', scope, 'AutoCAD 2007 DXF (UTF-8, birim metre); koordinatlar yuvarlanmadan, tam yazılır.', 'grow'));
    this.renderLayers();
    this.renderSummary();
  }

  private renderLayers(): void {
    const { ctx } = this;
    const all = this.byLayer(this.scope);
    if (!all.size) {
      replaceChildren(this.layersHost, h('p', { class: 'io-empty' }, 'Bu kapsamda nesne yok.'));
      return;
    }
    const ids = [...all.keys()];
    const out = ids.filter((id) => this.excluded.has(id)).length;
    const every = h('input', { type: 'checkbox', 'aria-label': 'Bütün katmanlar', checked: out === 0 });
    every.indeterminate = out > 0 && out < ids.length;
    every.addEventListener('change', () => {
      for (const id of ids) {
        if (every.checked) this.excluded.delete(id);
        else this.excluded.add(id);
      }
      this.render();
    });
    const palette = ctx.view.palette;
    const layers = ctx.doc.layers;
    const rows = ids.map((id) => {
      const layer = layers.get(id);
      const name = layer ? layers.path(id) : 'Katmanı olmayan nesneler';
      const box = h('input', { type: 'checkbox', 'aria-label': `“${name}” katmanını yaz`, checked: !this.excluded.has(id) });
      box.addEventListener('change', () => {
        if (box.checked) this.excluded.delete(id);
        else this.excluded.add(id);
        this.render();
      });
      const state = !layer ? 'DXF katmanı 0 olarak' : [layers.isVisible(id) ? '' : "gizli: DXF'te kapalı", layers.isLocked(id) ? 'kilitli' : ''].filter(Boolean).join(', ');
      return h(
        'tr',
        null,
        h('td', { class: 'io-table__line' }, box),
        h('td', null, layer ? h('span', { class: 'swatch', style: `--swatch:${colorSwatch(layer.style.color, palette)}` }) : null, ' ', name),
        h('td', { class: 'num' }, String(all.get(id)?.length ?? 0)),
        h('td', { class: 'io-table__skip' }, state),
      );
    });
    replaceChildren(
      this.layersHost,
      h('table', { class: 'io-table' }, h('thead', null, h('tr', null, h('th', { class: 'io-table__line' }, every), h('th', null, 'Katman'), h('th', null, 'Nesne'), h('th', null, "DXF'te"))), h('tbody', null, rows)),
    );
  }

  private renderSummary(): void {
    const { ctx } = this;
    const chosen = this.chosen();
    const list = [...chosen.values()].flat();
    const kinds = new Map<string, number>();
    for (const e of list) kinds.set(e.kind, (kinds.get(e.kind) ?? 0) + 1);
    const layers = [...chosen.keys()].map((id) => ctx.doc.layers.get(id)).filter((l) => !!l);
    const dimensions = kinds.get('dimension') ?? 0;
    const islands = list.filter((e) => e.kind === 'polygon' && e.holes?.length).length;
    const data = list.some((e) => e.label || e.symbol || Object.keys(e.attrs).length);
    const themed = layers.some((l) => THEMED.has(l.style.color)) || list.some((e) => e.color && THEMED.has(e.color));
    const styled = layers.filter((l) => l.style.renderer || l.style.fill).length;
    const hidden = layers.filter((l) => !ctx.doc.layers.isVisible(l.id)).length;
    const names = new Map<string, number>();
    for (const l of layers) names.set(l.name.trim().toUpperCase(), (names.get(l.name.trim().toUpperCase()) ?? 0) + 1);
    const repeated = [...names.values()].some((n) => n > 1);
    replaceChildren(
      this.summary,
      list.length
        ? summaryLine('ok', `${list.length} nesne ${chosen.size} katmanla yazılacak: ${kindCounts(kinds)}.`)
        : summaryLine('warn', 'Yazılacak nesne yok. Başka bir kapsam ya da en az bir katman seçin.'),
      dimensions ? summaryLine('info', `${dimensions} ölçü çizgi, yay ve yazılarına patlatılarak yazılır: DXF'te ölçü olarak düzenlenemez, KentOS'a da ölçü olarak geri okunmaz.`) : null,
      islands ? summaryLine('info', `${islands} adalı alanın adaları ayrı kapalı çoklu çizgiler olarak yazılır; KentOS'a geri okununca yine adalı alan olur.`) : null,
      data ? summaryLine('info', 'Etiketler, öznitelikler ve semboller nesnelerle birlikte KentOS verisi olarak yazılır: başka programlar göstermez, KentOS geri okur.') : null,
      themed ? summaryLine('info', "Tema renkleri DXF'te sabit renk olur (ana mürekkep 7, ikincil 8); KentOS'a geri okununca yine tema rengidir.") : null,
      styled ? summaryLine('info', `${styled} katmanın stili (semboller, dolgular) DXF'e yazılmaz; rengi, çizgi tipi ve kalınlığı yazılır.`) : null,
      repeated ? summaryLine('info', "Aynı adlı katmanlar DXF'te grup adlarıyla ayrılır (“Grup - Katman”); DXF katmanları düz bir listedir.") : null,
      hidden ? summaryLine('info', `${hidden} gizli katman DXF'te kapalı yazılır.`) : null,
    );
    this.primary.disabled = this.writing || !list.length;
  }

  /** The layers as the writer receives them (the groups above each, outermost first). */
  private layerInput(id: string): DxfWriteLayer | null {
    const layers = this.ctx.doc.layers;
    const l = layers.get(id);
    if (!l) return null;
    const path: string[] = [];
    for (let p = layers.parentOf(id); p; p = layers.parentOf(p.id)) path.unshift(p.name);
    return { id, name: l.name, path, color: l.style.color, visible: layers.isVisible(id), locked: layers.isLocked(id), lineType: l.style.lineType, lineWeight: l.style.lineWeight };
  }

  private async run(): Promise<void> {
    if (this.primary.disabled) return;
    const { ctx } = this;
    const chosen = this.chosen();
    const settings = ctx.doc.settings;
    // A dimension without its own text is written with the value the drawing shows (project units).
    const entities = [...chosen.values()].flat().map((e) => {
      if (e.kind !== 'dimension' || e.text) return e;
      const l = layoutDimension(e);
      return l ? { ...e, text: ctx.view.dimensionText(l) } : e;
    });
    const input: DxfWriteInput = {
      entities,
      layers: [...chosen.keys()].map((id) => this.layerInput(id)).filter((l) => !!l),
      scale: settings.plotScale.value,
      lengthDecimals: settings.lengthDecimals.value,
      grads: settings.angleUnit.value === 'grad',
    };
    this.writing = true;
    this.primary.disabled = true;
    this.status.textContent = 'Yazılıyor…';
    this.status.dataset.kind = 'info';
    try {
      const out = await formats().writeDxf(input);
      const name = await saveExport(ctx, out.bytes, exportName(ctx, '.dxf'), DXF);
      if (!name) {
        this.status.textContent = '';
        return;
      }
      const written = Object.values(out.report.counts).reduce((s, n) => s + (n ?? 0), 0);
      ctx.log.success(`“${name}” yazıldı: ${written} nesne, ${input.layers.length} katman (AutoCAD 2007 DXF).`);
      for (const item of out.report.notes) ctx.log.info(reportText(item));
      if (out.report.skipped.length) ctx.log.warn(`“${name}” içine yazılmayanlar: ${out.report.skipped.map(reportText).join(' ')}`);
      this.dialog.close();
    } catch (e) {
      this.status.textContent = `Yazılamadı: ${message(e)}`;
      this.status.dataset.kind = 'error';
    } finally {
      this.writing = false;
      this.primary.disabled = ![...chosen.values()].some((l) => l.length);
    }
  }
}
