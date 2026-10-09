import type { LayerTime } from '../contracts/generated/LayerTime';
import type { Entity } from '../model/entities';
import { floorTime, readTime, showTime, writeTime } from '../model/time';
import { entitiesCreate } from '../product/entitiesCreate';
import { entitiesSet } from '../product/entitiesSet';
import { newObjectOf } from './layerMoveTool';
import { SelectionFirstTool } from './modifyTools';

/**
 * Yeni sürüm oluştur and Sona erdir (docs/adr/0210 §7; the desktop's `kentos_interaction::time_version`): the selected
 * objects (picked first when nothing is selected), each on a ranged temporal layer and with its start before the date
 * and its end after it, end at the date. Yeni sürüm writes beside each a copy with every property, starting at the date
 * and ending where the object ended; the copies are selected. The date is the slider's moment, or today's midnight
 * with the slider closed; another one is typed (05.03.2024, 2024-03-05). One undo step named after the tool, through
 * `cad.entities.set` and `cad.entities.create`; one object that cannot take it, on a locked layer, or whose dates
 * do not read, and nothing is written. A layer whose start or end field is of the date kind (docs/adr/0199) takes the
 * day alone: the date is that day's midnight (§3). The messages are the words the desktop says too.
 */
export class TimeVersionTool extends SelectionFirstTool {
  readonly id: 'timeVersion' | 'timeEnd';
  protected readonly label: string;
  private date = 0;

  constructor(ctx: ConstructorParameters<typeof SelectionFirstTool>[0], id: 'timeVersion' | 'timeEnd') {
    super(ctx);
    this.id = id;
    this.label = id === 'timeVersion' ? 'Yeni sürüm oluştur' : 'Sona erdir';
  }

  /** The slider's moment while it is open, else today's midnight as a date is written (docs/adr/0210 §7). */
  private defaultDate(): number {
    const t = this.ctx.time;
    if (t.open.value) return t.moment();
    const now = new Date();
    return Date.UTC(now.getFullYear(), now.getMonth(), now.getDate());
  }

  protected begin(): void {
    this.date = this.defaultDate();
  }

  protected stagePrompt(): string {
    return `tarih ${showTime(this.date, 'day')}: Enter yazar, başka bir tarih yazılabilir`;
  }

  protected point(): void {}

  override acceptPoint(): boolean {
    return false;
  }

  override input(text: string): boolean {
    if (this.picking) return false;
    const r = readTime(text.trim());
    if (r.kind === 'empty') return false;
    if (r.kind === 'unreadable') {
      this.ctx.log.warn(`“${text.trim()}” bir tarih olarak okunamadı; 05.03.2024 ya da 2024-03-05 gibi yazın.`);
      return true;
    }
    this.date = r.t;
    this.write();
    return true;
  }

  override confirm(): void {
    if (this.picking) return super.confirm();
    this.write();
  }

  /** One step back: from the date to the picking of objects (the selection stays); from picking out of the tool. */
  cancel(): boolean {
    if (this.picking) return false;
    this.picking = true;
    this.refresh();
    return true;
  }

  private write(): void {
    const { doc, log, selection } = this.ctx;
    const layers = doc.layers;
    const list = this.targets();
    if (!list.length) {
      this.picking = true;
      return this.refresh();
    }
    const D = this.date;
    const when = showTime(D, 'day');
    const byLayer = new Map<string, { rule: LayerTime; at: number; list: Entity[] }>();
    let outside = 0;
    for (const e of list) {
      const node = layers.get(e.layerId);
      const rule = node?.time;
      const name = node?.name ?? e.layerId;
      if (!rule || rule.end == null)
        return void log.warn(`“${name}” katmanı başlangıç ve bitiş alanı olan bir zamansal katman değil; sürüm yalnız aralıklı katmanda yapılır. Katmanın zaman ayarlarına bakın. Hiçbir nesne yazılmadı.`);
      if (layers.isLocked(e.layerId)) return void log.warn(`“${name}” katmanı kilitli; hiçbir nesne yazılmadı. Kilidi Katmanlar panelinden açın.`);
      const s = readTime(e.attrs[rule.start] ?? '');
      const t = readTime(e.attrs[rule.end] ?? '');
      if (s.kind === 'unreadable' || t.kind === 'unreadable')
        return void log.warn(`“${name}” katmanındaki bir nesnenin başlangıcı ya da bitişi tarih olarak okunamıyor; önce Öznitelikler’den düzeltin. Hiçbir nesne yazılmadı.`);
      const start = s.kind === 'moment' ? s.t : -Infinity;
      const end = t.kind === 'moment' ? t.t : Infinity;
      // A date field of the layer's (docs/adr/0199) takes the day alone: the date is that day's midnight.
      const days = (node?.fields ?? []).some((f) => f.kind === 'date' && (f.name === rule.start || f.name === rule.end));
      const at = days ? floorTime(D, 'day') : D;
      if (!(start < at && at < end)) outside++;
      const group = byLayer.get(e.layerId);
      if (group) group.list.push(e);
      else byLayer.set(e.layerId, { rule, at, list: [e] });
    }
    if (outside) return void log.warn(`${outside} nesnenin zamanı ${when} anını içermiyor: başlangıcı bu tarihten önce, bitişi sonra olmalı. Hiçbir nesne yazılmadı.`);
    const version = this.id === 'timeVersion';
    const group = doc.beginGroup(this.label);
    const copies: number[] = [];
    for (const [layerId, { rule, at, list: own }] of byLayer) {
      const text = writeTime(at, false);
      // The copies as the objects are before their end changes: each starts at the date and ends where the object
      // ended (an object without an end makes one without).
      const objects = own.map((e) => {
        const attrs: Record<string, string> = { ...e.attrs, [rule.start]: text };
        if (e.attrs[rule.end!] === undefined) delete attrs[rule.end!];
        return { ...newObjectOf(e), attrs };
      });
      const set = entitiesSet.execute({ doc }, { uids: own.map((e) => doc.uidOf(e.id)!), attrs: { [rule.end!]: text }, operation: 'attributes' });
      if (set.status !== 'completed') {
        group.cancel();
        return void ('error' in set && log.warn(set.error.message));
      }
      if (!version) continue;
      const made = entitiesCreate.execute({ doc }, { layerId, objects });
      if (made.status !== 'completed') {
        group.cancel();
        return void ('error' in made && log.warn(made.error.message));
      }
      copies.push(...made.output.ids);
    }
    group.end();
    if (version) {
      selection.set(copies);
      log.success(`Yeni sürüm: ${list.length} nesne ${when} tarihinde sona erdi, ${copies.length} yeni sürümü yazıldı ve seçildi.`);
    } else {
      selection.clear();
      log.success(`Sona erdi: ${list.length} nesne, ${when}.`);
    }
    this.picking = true;
    this.refresh();
  }
}
