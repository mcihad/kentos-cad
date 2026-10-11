import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { KmlLayer } from '../../contracts/generated/KmlLayer';
import { formats, type WrittenFile } from '../../io/client';
import type { Entity } from '../../model/entities';
import { hasArcs } from '../../model/pathCounts';
import { projectCrsName } from '../../model/projectCrs';
import { accuracyText } from '../../model/secondCrs';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { exportName, field, kindCounts, reportText, summaryLine, unreachedWhy } from './common';
import { saveExport } from './save';
import { SCOPE_LABEL, scopeEntities, type ExportScope } from './scope';
import { projectSystem, wayToWgs84 } from './wgs84';

/**
 * KML/KMZ ve GPX dışa aktar (docs/adr/0222 §4, §5; the desktop's `exchange/wgs84_export.rs`): the objects of the
 * selection, of the visible layers or of the whole drawing, moved from the project's system into WGS 84 longitude and
 * latitude by the shared writers in the formats worker, the way Koordinat dönüştür moves a point; the way and its
 * accuracy are said before writing. KML keeps the layers as folders, the colours and line weights as styles and the
 * attributes as data; GPX writes points as waypoints and lines as tracks or routes. A project without a coordinate
 * system writes neither: its coordinates are bound to no place. What the format cannot hold is said before writing and
 * after (the writer's report, in the log). An export is not a save.
 */
export function openKmlExport(ctx: AppContext): void {
  new Wgs84ExportDialog(ctx, 'kml');
}

export function openGpxExport(ctx: AppContext): void {
  new Wgs84ExportDialog(ctx, 'gpx');
}

type Format = 'kml' | 'gpx';

const KINDS: Record<string, FileKind> = {
  kml: { description: 'KML', accept: { 'application/vnd.google-earth.kml+xml': ['.kml'] } },
  kmz: { description: 'KMZ', accept: { 'application/vnd.google-earth.kmz': ['.kmz'] } },
  gpx: { description: 'GPX', accept: { 'application/gpx+xml': ['.gpx'] } },
};
const CURVES = new Set(['circle', 'arc', 'ellipse', 'spline']);
const UNWRITTEN = new Set(['dimension', 'table', 'image', 'xline', 'ray', 'raster', 'pointcloud']);

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

class Wgs84ExportDialog {
  private readonly ctx: AppContext;
  private readonly format: Format;
  private scope: ExportScope;
  /** Layers the user left out. */
  private readonly excluded = new Set<string>();
  /** KMZ rather than KML; routes rather than tracks. */
  private other = false;
  private writing = false;
  private readonly form = h('div', { class: 'io-row' });
  private readonly layersHost = h('div', { class: 'io-table-wrap' });
  private readonly summary = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Dışa aktar…');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, format: Format) {
    this.ctx = ctx;
    this.format = format;
    this.scope = scopeEntities(ctx, 'selection').length ? 'selection' : 'visible';
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: format === 'kml' ? 'KML/KMZ dışa aktar' : 'GPX dışa aktar',
      width: 820,
      className: 'dialog--io',
      content: [this.form, this.layersHost, this.summary],
      footer: [this.status, cancel, this.primary],
    });
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.run());
    this.render();
  }

  /** The format's name and its Turkish endings: “KML'de”, “KML'ye”; “GPX'te”, “GPX'e”. */
  private get names(): { name: string; at: string; to: string } {
    return this.format === 'kml' ? { name: 'KML', at: "KML'de", to: "KML'ye" } : { name: 'GPX', at: "GPX'te", to: "GPX'e" };
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
    const [kindLabel, first, second, hint] =
      this.format === 'kml'
        ? ['Dosya', 'KML', 'KMZ (sıkıştırılmış)', "KMZ, KML'nin sıkıştırılmış hâlidir (içinde doc.kml); Google Earth ikisini de açar."]
        : [
            'Çizgiler',
            'İz (trk)',
            'Rota (rte)',
            "İz (trk) kaydedilmiş bir yolu, rota (rte) izlenecek bir yolu anlatır; el GPS'leri ikisini de okur. Alanların sınırları her zaman iz olarak yazılır.",
          ];
    const kind = segmented<'one' | 'other'>({
      label: kindLabel,
      options: [
        { value: 'one', label: first },
        { value: 'other', label: second },
      ],
      value: this.other ? 'other' : 'one',
      onChange: (v) => ((this.other = v === 'other'), this.render()),
    });
    replaceChildren(
      this.form,
      field('Yazılacak nesneler', scope, 'Koordinatlar projenin sisteminden WGS 84 boylam ve enlemine çevrilir; on basamakla (yaklaşık 0,01 mm) yazılır.', 'grow'),
      field(kindLabel, kind, hint, 'grow'),
    );
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
      const written = !layer ? 'katmansız' : this.format === 'kml' ? `klasör “${layers.path(id)}”` : 'yol noktası, iz ya da rota';
      return h(
        'tr',
        null,
        h('td', { class: 'io-table__line' }, box),
        h('td', { class: 'io-table__name' }, layer ? h('span', { class: 'swatch', style: `--swatch:${colorSwatch(layer.style.color, palette)}` }) : null, ' ', name),
        h('td', { class: 'num' }, String(all.get(id)?.length ?? 0)),
        h('td', { class: 'io-table__skip' }, written),
      );
    });
    replaceChildren(
      this.layersHost,
      h(
        'table',
        { class: 'io-table' },
        h('thead', null, h('tr', null, h('th', { class: 'io-table__line' }, every), h('th', null, 'Katman'), h('th', null, 'Nesne'), h('th', null, this.format === 'kml' ? "KML'de" : "GPX'te"))),
        h('tbody', null, rows),
      ),
    );
  }

  private renderSummary(): void {
    const { ctx } = this;
    const { name: format, at, to } = this.names;
    const list = [...this.chosen().values()].flat();
    const kinds = new Map<string, number>();
    for (const e of list) kinds.set(e.kind, (kinds.get(e.kind) ?? 0) + 1);
    const unwritten = list.filter((e) => UNWRITTEN.has(e.kind)).length;
    const written = list.length - unwritten;
    const way = wayToWgs84(ctx, list.map((e) => e.id));
    const system = projectCrsName(ctx.doc.settings);
    const lines = [
      written
        ? summaryLine('ok', `${written} nesne yazılacak: ${kindCounts(kinds)}.`)
        : summaryLine('warn', `Yazılacak nesne yok. Başka bir kapsam ya da en az bir katman seçin; ölçü, tablo, resim, raster, nokta bulutu ve sonsuz doğrular ${to} yazılmaz.`),
      !projectSystem(ctx)
        ? summaryLine(
            'warn',
            `Proje yerel (koordinat sistemi yok): koordinatlar bir konuma bağlı değil, WGS 84'e çevrilemez; ${format} yazılamaz. Projeye Proje ayarları › Koordinat sistemi'nden bir sistem atayın.`,
          )
        : !way
          ? summaryLine('info', 'Dönüşüm, yazılacak nesneler seçilince gösterilir.')
          : 'error' in way
            ? summaryLine('warn', `Koordinatlar WGS 84'e çevrilemez: ortaları ${unreachedWhy(way.error)}. ${format} yazılamaz.`)
            : summaryLine(
                way.unofficial ? 'warn' : 'ok',
                `Koordinatlar ${system} sisteminden WGS 84 boylam ve enlemine çevrilir; dönüşüm: ${accuracyText(way)}. Kotlar olduğu gibi yazılır.`,
              ),
    ];
    const curves = list.filter((e) => CURVES.has(e.kind) || hasArcs(e)).length;
    if (curves) lines.push(summaryLine('info', `${curves} eğri (daire, yay, elips, spline, yaylı çizgi) ${at} olmadığı için uygulamanın kendi örneklemesiyle (turda 72 adım) çizgiye çevrilir.`));
    const texts = kinds.get('text') ?? 0;
    if (texts)
      lines.push(summaryLine('info', this.format === 'kml' ? `${texts} yazı, metni adı olan nokta yer imi olarak yazılır.` : `${texts} yazı, metni adı olan yol noktası (wpt) olarak yazılır.`));
    const hatches = kinds.get('hatch') ?? 0;
    if (hatches) lines.push(summaryLine('info', `${hatches} taramanın deseni yazılmaz; sınırı alan olarak yazılır.`));
    const areas = list.filter((e) => e.kind === 'polygon' || e.kind === 'hatch').length;
    if (areas && this.format === 'gpx') lines.push(summaryLine('info', `GPX'te alan yok: ${areas} alanın sınırı ve delikleri kapalı iz (trk) parçaları olarak yazılır.`));
    const inserts = kinds.get('insert') ?? 0;
    if (inserts) lines.push(summaryLine('info', `${inserts} blok yerleştirmesi, bloğunun nesneleriyle açılmış olarak yazılır; ${at} blok yoktur.`));
    if (unwritten) lines.push(summaryLine('warn', `${unwritten} ölçü, tablo, resim, raster, nokta bulutu ya da sonsuz doğru ${at} gösterilemez; yazılmaz.`));
    lines.push(
      summaryLine(
        'info',
        this.format === 'kml'
          ? 'Katmanlar grup yollarıyla klasör, nesneler yer imi olur: adı etiketi (yoksa Ad özniteliği), öznitelikleri veri (ExtendedData), rengi ve kalınlığı biçem (alanlar beşte iki saydam dolguyla).'
          : 'Yol noktalarının, izlerin ve rotaların adı etiket (yoksa Ad özniteliği), açıklaması Açıklama, türü Kod özniteliğidir; öbür öznitelikler ve renkler GPX\'e yazılmaz.',
      ),
    );
    replaceChildren(this.summary, lines);
    this.primary.disabled = this.writing || !written || !way || 'error' in way;
  }

  /** The layers as KML writes them: their names, groups (outermost first), colours and line weights. */
  private kmlLayers(ids: readonly string[]): KmlLayer[] {
    const layers = this.ctx.doc.layers;
    return ids.flatMap((id) => {
      const l = layers.get(id);
      if (!l) return [];
      const path: string[] = [];
      for (let p = layers.parentOf(id); p; p = layers.parentOf(p.id)) path.unshift(p.name);
      return [{ id, name: l.name, path, color: l.style.color, lineWeight: l.style.lineWeight }];
    });
  }

  private async run(): Promise<void> {
    const own = projectSystem(this.ctx);
    if (this.primary.disabled || !own) return;
    const { ctx } = this;
    const chosen = this.chosen();
    const entities = [...chosen.values()].flat();
    const name = ctx.doc.name.value.replace(/\.kcad$/i, '') || 'cizim';
    // An insert is written as its block's objects placed (docs/adr/0144 §5).
    const blocks = [...ctx.doc.blocks.value];
    this.writing = true;
    this.primary.disabled = true;
    this.status.textContent = 'Yazılıyor…';
    this.status.dataset.kind = 'info';
    try {
      let out: WrittenFile;
      let extension: string;
      if (this.format === 'kml') {
        out = await formats().writeKml({ entities, layers: this.kmlLayers([...chosen.keys()]), name, blocks, kmz: this.other }, own.from, own.choices);
        extension = this.other ? 'kmz' : 'kml';
      } else {
        out = await formats().writeGpx({ entities, name, blocks, routes: this.other }, own.from, own.choices);
        extension = 'gpx';
      }
      const saved = await saveExport(ctx, out.bytes, exportName(ctx, `.${extension}`), KINDS[extension]!);
      if (!saved) {
        this.status.textContent = '';
        return;
      }
      const count = Object.values(out.report.counts).reduce((s, n) => s + (n ?? 0), 0);
      ctx.log.success(`“${saved}” yazıldı: ${count} nesne (${extension.toUpperCase()}, WGS 84).`);
      for (const item of out.report.notes) ctx.log.info(reportText(item));
      if (out.report.skipped.length) ctx.log.warn(`“${saved}” içine yazılmayanlar: ${out.report.skipped.map(reportText).join(' ')}`);
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
