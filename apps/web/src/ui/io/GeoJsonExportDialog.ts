import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { GeoJsonWriteInput } from '../../contracts/generated/GeoJsonWriteInput';
import { formats } from '../../io/client';
import type { Entity } from '../../model/entities';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { exportName, field, kindCounts, reportText, summaryLine } from './common';
import { saveExport } from './save';
import { SCOPE_LABEL, scopeEntities, type ExportScope } from './scope';

/**
 * GeoJSON dışa aktar (docs/adr/0046): the objects of the selection, of the
 * visible layers or of the whole drawing as a FeatureCollection
 * (crates/shared/formats geojson). Coordinates are written exactly and never
 * transformed: a WGS 84 project (EPSG:4326) gives RFC 7946 GeoJSON; any
 * other its own coordinates with the 2008 `crs` member naming them, which
 * the window says before writing (RFC 7946 knows only WGS 84, and there is
 * no coordinate transformation yet). What GeoJSON cannot hold is said before
 * writing (the summary) and after (the writer's report, in the log).
 */
export function openGeoJsonExport(ctx: AppContext): void {
  new GeoJsonExportDialog(ctx);
}

const GEOJSON: FileKind = { description: 'GeoJSON', accept: { 'application/geo+json': ['.geojson'] } };
const CURVES = new Set(['circle', 'arc', 'ellipse', 'spline']);
const UNWRITTEN = new Set(['text', 'dimension', 'xline', 'ray']);

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

const bulged = (e: Entity) => (e.kind === 'polyline' || e.kind === 'polygon') && ((e.bulges ?? []).some((b) => Math.abs(b) > 1e-12) || (e.holes ?? []).some((r) => (r.bulges ?? []).some((b) => Math.abs(b) > 1e-12)));

class GeoJsonExportDialog {
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
      title: 'GeoJSON dışa aktar',
      width: 820,
      className: 'dialog--io',
      content: [this.form, this.layersHost, this.summary],
      footer: [this.status, cancel, this.primary],
    });
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.run());
    this.render();
  }

  /** The scope's objects by layer (the tree's order). */
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
    replaceChildren(this.form, field('Yazılacak nesneler', scope, 'GeoJSON FeatureCollection (UTF-8); koordinatlar yuvarlanmadan, tam yazılır.', 'grow'));
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
      return h(
        'tr',
        null,
        h('td', { class: 'io-table__line' }, box),
        h('td', null, layer ? h('span', { class: 'swatch', style: `--swatch:${colorSwatch(layer.style.color, palette)}` }) : null, ' ', name),
        h('td', { class: 'num' }, String(all.get(id)?.length ?? 0)),
        h('td', { class: 'io-table__skip' }, layer ? `kentos.layer: “${layer.name}”` : 'katmansız'),
      );
    });
    replaceChildren(
      this.layersHost,
      h('table', { class: 'io-table' }, h('thead', null, h('tr', null, h('th', { class: 'io-table__line' }, every), h('th', null, 'Katman'), h('th', null, 'Nesne'), h('th', null, "GeoJSON'da"))), h('tbody', null, rows)),
    );
  }

  private renderSummary(): void {
    const { ctx } = this;
    const list = [...this.chosen().values()].flat();
    const kinds = new Map<string, number>();
    for (const e of list) kinds.set(e.kind, (kinds.get(e.kind) ?? 0) + 1);
    const written = list.filter((e) => !UNWRITTEN.has(e.kind));
    const curves = list.filter((e) => CURVES.has(e.kind) || bulged(e)).length;
    const hatches = kinds.get('hatch') ?? 0;
    const areas = list.filter((e) => e.kind === 'polygon' || e.kind === 'hatch').length;
    const unwritten = list.length - written.length;
    const data = list.some((e) => e.label || Object.keys(e.attrs).length);
    const styled = list.filter((e) => e.color || e.symbol).length;
    const crs = ctx.doc.crs.value;
    const rfc = crs.srid === 4326;
    replaceChildren(
      this.summary,
      written.length
        ? summaryLine('ok', `${written.length} nesne yazılacak: ${kindCounts(kinds)}.`)
        : summaryLine('warn', 'Yazılacak nesne yok. Başka bir kapsam ya da en az bir katman seçin; yazı, ölçü ve sonsuz doğrular GeoJSON\'a yazılmaz.'),
      rfc
        ? summaryLine('ok', 'Proje WGS 84 (EPSG:4326) sisteminde: dosya RFC 7946 GeoJSON olur (boylam, enlem).')
        : summaryLine(
            'warn',
            h('strong', null, 'Dosya RFC 7946 GeoJSON olmayacak. '),
            `Proje ${crs.name} (EPSG:${crs.srid}) sisteminde; RFC 7946 yalnız WGS 84 boylam, enlem ister. Koordinatlar dönüştürülmeden projenin sisteminde yazılır ve eski (2008) crs üyesiyle adlandırılır: QGIS ve GDAL doğru okur, yalnız RFC 7946'yı bilen programlar (web haritaları) yanlış yere koyar. WGS 84'e koordinat dönüşümü henüz yok (Koordinat → Datum dönüşümü, geliştirme aşamasında).`,
          ),
      curves ? summaryLine('info', `${curves} eğri (daire, yay, elips, spline, yaylı çizgi) GeoJSON'da olmadığı için uygulamanın kendi örneklemesiyle (turda 72 adım) çizgiye çevrilir.`) : null,
      hatches ? summaryLine('info', `${hatches} tarama alan (Polygon) olarak yazılır; deseni yazılmaz.`) : null,
      areas ? summaryLine('info', "Alan halkaları RFC 7946'nın sağ el kuralına göre yazılır (dış sınır saat yönünün tersine); köşeler değişmez.") : null,
      unwritten ? summaryLine('warn', `${unwritten} yazı, ölçü ya da sonsuz doğru GeoJSON'da gösterilemez; yazılmaz.`) : null,
      data ? summaryLine('info', 'Öznitelikler metin özellik (properties) olarak yazılır; katman ve etiket KentOS\'un “kentos” üyesinde: KentOS geri okur, öbür programlar yok sayar.') : null,
      styled ? summaryLine('info', `${styled} nesnenin kendi rengi ya da sembolü GeoJSON'a yazılmaz.`) : null,
    );
    this.primary.disabled = this.writing || !written.length;
  }

  private async run(): Promise<void> {
    if (this.primary.disabled) return;
    const { ctx } = this;
    const chosen = this.chosen();
    const layers = ctx.doc.layers;
    const input: GeoJsonWriteInput = {
      entities: [...chosen.values()].flat(),
      layers: [...chosen.keys()].flatMap((id) => {
        const l = layers.get(id);
        return l ? [{ id, name: l.name }] : [];
      }),
      srid: ctx.doc.crs.value.srid,
      name: ctx.doc.name.value.replace(/\.kcad$/i, '') || 'cizim',
    };
    this.writing = true;
    this.primary.disabled = true;
    this.status.textContent = 'Yazılıyor…';
    this.status.dataset.kind = 'info';
    try {
      const out = await formats().writeGeoJson(input);
      const name = await saveExport(ctx, out.bytes, exportName(ctx, '.geojson'), GEOJSON);
      if (!name) {
        this.status.textContent = '';
        return;
      }
      const count = Object.values(out.report.counts).reduce((s, n) => s + (n ?? 0), 0);
      ctx.log.success(`“${name}” yazıldı: ${count} nesne (GeoJSON${input.srid === 4326 ? ', RFC 7946' : `, EPSG:${input.srid}`}).`);
      for (const item of out.report.notes) ctx.log.info(reportText(item));
      if (out.report.skipped.length) ctx.log.warn(`“${name}” içine yazılmayanlar: ${out.report.skipped.map(reportText).join(' ')}`);
      this.dialog.close();
    } catch (e) {
      this.status.textContent = `Yazılamadı: ${message(e)}`;
      this.status.dataset.kind = 'error';
    } finally {
      this.writing = false;
      this.renderSummary();
    }
  }
}
