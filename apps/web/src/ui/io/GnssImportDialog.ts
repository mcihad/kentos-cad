import type { AppContext } from '../../app/context';
import type { FileKind, PickedFile } from '../../app/fileIO';
import type { GnssRead } from '../../contracts/generated/GnssRead';
import { fixed } from '../../core/displayNumber';
import { applyImport, layerNamed, type LayerTarget } from '../../io/apply';
import { formats } from '../../io/client';
import { POINT_LAYER_STYLE } from '../../io/coords';
import { formatDd } from '../../model/geom/crsTransform';
import { gnssEntities, placeGnss, type GnssPlan, type PlacedPoint } from '../../model/gnssImport';
import { crsSettings, datumChoices, ownSystem, type NamedSystem } from '../../model/projectCrs';
import { h, replaceChildren } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { baseName, extentLine, field, fileLine, select, summaryLine } from './common';
import { zoomToImported } from './zoom';

/**
 * GNSS içe aktar (docs/adr/0169 §6; the desktop's `exchange/gnss_import.rs`): a receiver's GPX or NMEA file, read by
 * the formats worker (crates/shared/formats/src/gnss), its positions moved from WGS 84 into the project's system the way
 * Koordinat dönüştür moves them, with the accuracy and what it rests on said (docs/adr/0167). The kinds of points to
 * take and how the unnamed are named are chosen; the points show as they will be written. A project without a system
 * takes none: GNSS positions have one, and they are never placed without it (CLAUDE.md §5). The points go in as one
 * undo step, onto a new layer or one of the drawing's.
 */
export function openGnssImport(ctx: AppContext, file: PickedFile, kind: FileKind): void {
  new GnssImportDialog(ctx, file, kind);
}

const PREVIEW_ROWS = 12;
const NEW = '__new__';
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** The kinds of points in the file's order of the list, as the window names them. */
const KINDS: readonly { kind: string; label: string }[] = [
  { kind: 'wpt', label: 'Yol noktaları' },
  { kind: 'rtept', label: 'Rota noktaları' },
  { kind: 'trkpt', label: 'İz noktaları' },
  { kind: 'gga', label: 'GGA konumları' },
];

/** The format as the file line names it. */
const FORMAT_NAME: Record<string, string> = { gpx: 'GPX 1.1', nmea: 'NMEA 0183' };

/** Problems shown in the summary; the rest are counted. */
const SHOWN_PROBLEMS = 5;

class GnssImportDialog {
  private readonly ctx: AppContext;
  private readonly kind: FileKind;
  private file: PickedFile;
  private read: GnssRead | null = null;
  private failed: string | null = null;
  private generation = 0;
  private reading = false;
  private closed = false;
  /** The kinds the user left out; every other kind in the file is taken. */
  private readonly off = new Set<string>();
  private prefix = 'G';
  private start = 1;
  private target = NEW;
  private plan: GnssPlan | null = null;
  /** The project's system, when it has one (docs/adr/0168 §1). */
  private readonly system: NamedSystem | null;
  private readonly nameInput: HTMLInputElement;
  private readonly prefixInput: HTMLInputElement;
  private readonly startInput: HTMLInputElement;
  private readonly fileHost = h('div');
  private readonly optionsHost = h('div', { class: 'io-row' });
  private readonly tableHost = h('div', { class: 'io-table-wrap' });
  private readonly summaryHost = h('div', { class: 'io-summary' });
  private readonly layerHost = h('div', { class: 'io-row' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'İçe aktar');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, file: PickedFile, kind: FileKind) {
    this.ctx = ctx;
    this.file = file;
    this.kind = kind;
    this.system = ownSystem(crsSettings(ctx.doc.settings));
    this.nameInput = h('input', { class: 'field', value: baseName(file.name), 'aria-label': 'Yeni katmanın adı', spellcheck: 'false', dataset: { key: 'name' } });
    this.nameInput.addEventListener('input', () => this.updateButton());
    this.nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && this.run());
    this.prefixInput = h('input', { class: 'field gnss-prefix', value: this.prefix, 'aria-label': 'Adsız noktaların ön eki', spellcheck: 'false', dataset: { key: 'prefix' } });
    this.prefixInput.addEventListener('input', () => {
      this.prefix = this.prefixInput.value.trim();
      this.update();
    });
    this.startInput = h('input', { class: 'field gnss-start', type: 'number', min: '0', step: '1', value: String(this.start), 'aria-label': 'İlk numara', dataset: { key: 'start' } });
    this.startInput.addEventListener('input', () => {
      const n = Number(this.startInput.value);
      this.start = Number.isInteger(n) && n >= 0 ? n : this.start;
      this.update();
    });
    const other = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Başka dosya…');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'GNSS içe aktar',
      width: 960,
      className: 'dialog--io',
      content: [this.fileHost, this.optionsHost, this.tableHost, this.summaryHost, this.layerHost],
      footer: [other, this.status, cancel, this.primary],
      onClose: () => {
        this.closed = true;
        if (this.reading) formats().cancel();
      },
    });
    other.addEventListener('click', () => void this.pickAnother());
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => this.run());
    this.renderLayer();
    void this.refresh();
  }

  /** Reads the file; a late answer to an older read is dropped. */
  private async refresh(): Promise<void> {
    const gen = ++this.generation;
    this.reading = true;
    this.say('Dosya okunuyor…');
    try {
      const r = await formats().readGnss(this.file.bytes);
      if (gen !== this.generation || this.closed) return;
      this.read = r;
      this.failed = null;
      this.say('');
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.read = null;
      this.failed = `Dosya okunamadı: ${message(e)}`;
      this.say('');
    } finally {
      if (gen === this.generation) this.reading = false;
    }
    this.off.clear();
    this.render();
  }

  private async pickAnother(): Promise<void> {
    const f = await this.ctx.files.pickForImport(this.kind);
    if (!f || this.closed) return;
    this.file = f;
    this.nameInput.value = baseName(f.name);
    void this.refresh();
  }

  /** The kinds in the file, with how many of each. */
  private counts(): Map<string, number> {
    const counts = new Map<string, number>();
    for (const p of this.read?.points ?? []) counts.set(p.kind, (counts.get(p.kind) ?? 0) + 1);
    return counts;
  }

  /** The points as they will be written, with the current choices. */
  private place(): void {
    const r = this.read;
    const to = this.system?.system;
    if (!r || !to || !this.system) {
      this.plan = null;
      return;
    }
    const kinds = [...this.counts().keys()].filter((k) => !this.off.has(k));
    this.plan = placeGnss(r.points, { kinds, prefix: this.prefix, start: this.start }, to, this.system.name, datumChoices(crsSettings(this.ctx.doc.settings)));
  }

  /** A choice changed: the points again, the table and the summary. */
  private update(): void {
    this.place();
    this.renderTable();
    this.renderSummary();
    this.updateButton();
  }

  private render(): void {
    const focus = (document.activeElement as HTMLElement | null)?.dataset?.key;
    this.renderFile();
    this.renderOptions();
    this.update();
    if (focus) this.dialog.body.querySelector<HTMLElement>(`[data-key="${focus}"]`)?.focus();
  }

  private renderFile(): void {
    const r = this.read;
    const meta = r ? `${FORMAT_NAME[r.format] ?? r.format}, ${r.points.length.toLocaleString('tr-TR')} konum, ${r.encoding}` : this.failed ? 'okunamadı' : 'okunuyor…';
    replaceChildren(this.fileHost, fileLine(this.file.name, meta));
  }

  private renderOptions(): void {
    const counts = this.counts();
    const boxes = KINDS.filter((k) => counts.has(k.kind)).map((k) => {
      const box = h('input', { type: 'checkbox', checked: !this.off.has(k.kind), dataset: { key: `kind-${k.kind}` } });
      box.addEventListener('change', () => {
        if (box.checked) this.off.delete(k.kind);
        else this.off.add(k.kind);
        this.update();
      });
      return h('label', { class: 'io-check' }, box, `${k.label} (${(counts.get(k.kind) ?? 0).toLocaleString('tr-TR')})`);
    });
    const naming = h('div', { class: 'gnss-naming' }, this.prefixInput, this.startInput);
    replaceChildren(
      this.optionsHost,
      boxes.length ? field('Alınacak noktalar', h('div', { class: 'gnss-kinds' }, boxes)) : null,
      field('Adsız noktalar', naming, 'Dosyada adı olmayan noktalar ön ek ve sırayla artan numarayla adlanır.', 'grow'),
    );
  }

  private renderTable(): void {
    const r = this.read;
    const plan = this.plan;
    if (!r || !plan) {
      replaceChildren(this.tableHost, h('p', { class: 'io-empty' }, !r ? (this.failed ? 'Önizleme yok.' : 'Dosya okunuyor…') : 'Önizleme yok: konumlar projenin sistemine çevrilemiyor.'));
      return;
    }
    if (!plan.placed.length) {
      replaceChildren(this.tableHost, h('p', { class: 'io-empty' }, r.points.length ? 'Seçili türde nokta yok.' : 'Dosyada konum yok.'));
      return;
    }
    const f = this.ctx.format;
    const geographic = this.system?.system?.kind === 'geographic';
    const [first, second] = geographic ? ['Enlem', 'Boylam'] : [f.axesText('Y (sağa)'), f.axesText('X (yukarı)')];
    const place = (p: PlacedPoint): [string, string] => (geographic ? [formatDd(p.p.y, true, 9), formatDd(p.p.x, false, 9)] : [f.coord(p.p.x), f.coord(p.p.y)]);
    const head = h(
      'tr',
      null,
      h('th', { class: 'io-table__line' }, 'Satır'),
      ['Ad', 'Kaynak', first, second, 'Kot (m)', 'Çözüm', 'Uydu', 'HDOP', 'Zaman'].map((t) => h('th', null, t)),
    );
    const rows = plan.placed.slice(0, PREVIEW_ROWS).map((p) => {
      const [a, b] = place(p);
      return h(
        'tr',
        null,
        h('td', { class: 'io-table__line' }, String(p.line)),
        h('td', null, p.name),
        h('td', null, p.attrs['Kaynak'] ?? ''),
        h('td', { class: 'num' }, a),
        h('td', { class: 'num' }, b),
        h('td', { class: 'num' }, p.z === null ? '—' : fixed(p.z, 3)),
        h('td', null, p.attrs['Çözüm'] ?? ''),
        h('td', { class: 'num' }, p.attrs['Uydu'] ?? ''),
        h('td', { class: 'num' }, p.attrs['HDOP'] ?? ''),
        h('td', null, p.attrs['Zaman'] ?? ''),
      );
    });
    replaceChildren(this.tableHost, h('table', { class: 'io-table gnss-table' }, h('thead', null, head), h('tbody', null, rows)));
  }

  private renderSummary(): void {
    const r = this.read;
    if (!r) {
      replaceChildren(this.summaryHost, this.failed ? summaryLine('error', this.failed) : summaryLine('info', 'Dosya okunuyor…'));
      return;
    }
    const lines: HTMLElement[] = [];
    const plan = this.plan;
    if (!this.system) {
      lines.push(summaryLine('error', "Projenin koordinat sistemi yok: GNSS konumları WGS 84'tedir ve yerel sisteme çevrilemez. Proje ayarlarından bir koordinat sistemi seçin ya da özel sistem tanımlayın."));
    } else if (!this.system.system) {
      lines.push(summaryLine('error', `“${this.system.name}” sisteminin tanımı dönüşümlerde kullanılamıyor (yerel sistemin tabanı kayıttaki izdüşümlü bir sistem değil); GNSS konumları ona çevrilemez.`));
    } else if (plan) {
      const n = plan.placed.length;
      if (n) lines.push(summaryLine('ok', `${n.toLocaleString('tr-TR')} nokta alınacak.`));
      else lines.push(summaryLine('warn', r.points.length ? 'Alınacak nokta yok; türlerden en az birini seçin.' : 'Dosyada alınacak konum yok.'));
      const way = plan.placed[0]?.attrs['Dönüşüm'];
      if (way) lines.push(summaryLine(way.endsWith('resmî dönüşüm değil') ? 'warn' : 'info', `Dönüşüm: ${way}.`));
      if (n) {
        const bare = plan.placed.filter((p) => p.z === null).length;
        const heights = 'Kot elipsoit yüksekliğidir (dosyadaki yükseklik ile geoit ayrımının toplamı); ortometrik yüksekliğe çevrilmez.';
        lines.push(summaryLine('info', bare ? `${heights} ${bare.toLocaleString('tr-TR')} noktanın elipsoit yüksekliği bilinmiyor: kotu boş, dosyadaki yüksekliği öznitelikte.` : heights));
      }
      if (plan.skipped.length) lines.push(this.problems(`${plan.skipped.length.toLocaleString('tr-TR')} nokta projenin sistemine çevrilemedi; alınmayacak:`, plan.skipped));
      if (n && this.system.system.kind !== 'geographic') lines.push(extentLine(this.ctx, bounds(plan.placed)));
    }
    if (r.problems.length) lines.push(this.problems(`Dosyanın ${r.problems.length.toLocaleString('tr-TR')} kaydı okunmadı:`, r.problems));
    replaceChildren(this.summaryHost, lines);
  }

  private problems(title: string, items: readonly { message: string }[]): HTMLElement {
    const shown = items.slice(0, SHOWN_PROBLEMS);
    return summaryLine(
      'warn',
      title,
      h('ul', { class: 'io-summary__list' }, shown.map((e) => h('li', null, e.message)), items.length > shown.length ? h('li', null, `… ve ${items.length - shown.length} kayıt daha.`) : null),
    );
  }

  private renderLayer(): void {
    const layers = this.ctx.doc.layers;
    const choices = [
      { value: NEW, label: 'Yeni katman' },
      ...layers.leaves().map((l) => ({ value: l.id, label: `${layers.path(l.id)}${layers.isLocked(l.id) ? ' (kilitli)' : !layers.isVisible(l.id) ? ' (gizli)' : ''}`, disabled: layers.isLocked(l.id) })),
    ];
    const pick = select('Hedef katman', choices, this.target, (v) => {
      this.target = v;
      this.renderLayer();
      this.updateButton();
    }, 'target');
    const hidden = this.target !== NEW && !layers.isVisible(this.target);
    replaceChildren(
      this.layerHost,
      field('Hedef katman', pick, hidden ? 'Katman gizli: noktalar alınır ama görünmez.' : null, 'wide'),
      this.target === NEW ? field('Yeni katmanın adı', this.nameInput, 'Bu adda bir katman varsa noktalar ona eklenir.', 'grow') : null,
    );
  }

  /** The layer the points go to. */
  private layerTarget(): LayerTarget | null {
    if (this.target !== NEW) return { kind: 'existing', id: this.target };
    const name = this.nameInput.value.trim();
    if (!name) return null;
    const same = layerNamed(this.ctx.doc, name);
    return same ? { kind: 'existing', id: same.id } : { kind: 'new', name, style: POINT_LAYER_STYLE, visible: true, locked: false };
  }

  private updateButton(): void {
    const t = this.layerTarget();
    const locked = t?.kind === 'existing' && this.ctx.doc.layers.isLocked(t.id);
    this.primary.disabled = this.reading || !this.plan || this.plan.placed.length === 0 || !t || locked;
    if (locked) this.say(`“${this.ctx.doc.layers.get((t as { id: string }).id)?.name}” katmanı kilitli; kilidini Katmanlar panelinden açın ya da başka bir katman seçin.`, 'error');
    else if (this.status.dataset.kind === 'error') this.say('');
  }

  private say(text: string, kind: 'info' | 'error' = 'info'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private run(): void {
    const plan = this.plan;
    const target = this.layerTarget();
    if (this.primary.disabled || !plan || !target) return;
    const { ctx } = this;
    const applied = applyImport(ctx.doc, gnssEntities(plan.placed), { label: `GNSS: ${this.file.name}`, layers: new Map([['', target]]) });
    if (!applied.ok) {
      this.say(applied.error, 'error');
      return;
    }
    const layerName = target.kind === 'new' ? target.name : (ctx.doc.layers.get(target.id)?.name ?? '');
    zoomToImported(ctx, applied.ids);
    ctx.log.success(`“${this.file.name}”: ${applied.ids.length} GNSS noktası “${layerName}” katmanına alındı${applied.created.length ? ' (yeni katman)' : ''}. Tek adımda geri alınabilir.`);
    const left = plan.skipped.length + (this.read?.problems.length ?? 0);
    if (left) ctx.log.warn(`“${this.file.name}”: ${left} kayıt alınmadı (okunamayan ya da projenin sistemine çevrilemeyen); nedenleri içe aktarma penceresinde yazılıydı.`);
    this.dialog.close();
  }
}

/** The extent of the placed points. */
function bounds(placed: readonly PlacedPoint[]): { minX: number; minY: number; maxX: number; maxY: number } {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const { p } of placed) {
    minX = Math.min(minX, p.x);
    minY = Math.min(minY, p.y);
    maxX = Math.max(maxX, p.x);
    maxY = Math.max(maxY, p.y);
  }
  return { minX, minY, maxX, maxY };
}

