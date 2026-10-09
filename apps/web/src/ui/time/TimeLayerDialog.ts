import '../../styles/time.css';
import type { AppContext } from '../../app/context';
import type { LayerTime } from '../../contracts/generated/LayerTime';
import { layerTimes, showTime, timeValues } from '../../model/time';
import { layerTimeProblem } from '../../model/temporalRules';
import { layersTime } from '../../product/layersTime';
import { h, replaceChildren } from '../dom';
import { field, select, summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const TIME_LAYER_TITLE = 'Zaman ayarları';

/** “Yok”: no end (the objects are moments) or no key. */
const NONE = '';

/**
 * Zaman ayarları (docs/adr/0210 §10; the desktop's `time_layer.rs`): Katman (the active one, or the one the layer
 * tree's menu named), Başlangıç alanı, Bitiş alanı (Yok: the objects are moments), Kimlik alanı (Yok), Birikimli; the
 * fields offered are the layer's own and its objects' attribute names. What the values give is said as they are
 * chosen: how many objects have a time, how many have none, values that do not read as a date, the range. Kaydet
 * writes through `cad.layers.time` (one step “Zaman ayarları”); Zamanı kaldır takes the setting away.
 */
export function openTimeLayer(ctx: AppContext, layerId?: string): void {
  const { doc, log } = ctx;
  const layers = doc.layers;
  const candidates = layers.leaves().filter((l) => !l.service);
  if (!candidates.length) return void log.warn('Çizimde nesne tutan katman yok.');
  const first = layerId ?? layers.active.value;
  let layer = candidates.find((l) => l.id === first)?.id ?? candidates[0].id;
  let start = '';
  let end = NONE;
  let key = NONE;
  let cumulative = false;

  /** The layer's attribute names: its fields first, then its objects' other keys, in the order met. */
  const names = (): string[] => {
    const out = new Set<string>((layers.get(layer)?.fields ?? []).map((f) => f.name));
    for (const e of doc.byLayer(layer)) for (const k of Object.keys(e.attrs)) out.add(k);
    return [...out];
  };
  /** The setting as chosen, or null without a start. */
  const chosen = (): LayerTime | null => (start ? { start, ...(end && { end }), ...(key && { key }), ...(cumulative && { cumulative: true }) } : null);
  const load = () => {
    const t = layers.get(layer)?.time;
    const all = names();
    // A temporal layer's own setting; otherwise the first name that looks like a date's.
    start = t?.start ?? all.find((n) => /tarih|baslangic|başlangıç|start|date/i.test(n)) ?? all[0] ?? '';
    end = t?.end ?? NONE;
    key = t?.key ?? NONE;
    cumulative = !!t?.cumulative;
  };

  const layerBox = h('div', { class: 'time-layer__layer' });
  const fieldsBox = h('div', { class: 'io-row time-layer__fields' });
  const cumulativeBox = h('input', { type: 'checkbox' }) as HTMLInputElement;
  const summary = h('div', { class: 'io-summary' });
  const remove = h('button', { class: 'btn', type: 'button' }, 'Zamanı kaldır');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');

  function build(): void {
    const all = names();
    // A name the setting has that no object or field has any more is kept, to be seen.
    const choices = (extra: string[]) => [...new Set([...extra.filter(Boolean), ...all])].map((n) => ({ value: n, label: n }));
    replaceChildren(layerBox, field('Katman', select('Katman', candidates.map((l) => ({ value: l.id, label: layers.path(l.id) })), layer, (v) => ((layer = v), load(), build())), undefined, 'grow'));
    replaceChildren(
      fieldsBox,
      field('Başlangıç alanı', select('Başlangıç alanı', [...(start ? [] : [{ value: '', label: 'Seçin' }]), ...choices([start])], start, (v) => ((start = v), build()))),
      field('Bitiş alanı', select('Bitiş alanı', [{ value: NONE, label: 'Yok (anlık)' }, ...choices([end])], end, (v) => ((end = v), build()))),
      field('Kimlik alanı', select('Kimlik alanı', [{ value: NONE, label: 'Yok' }, ...choices([key])], key, (v) => ((key = v), build()))),
    );
    cumulativeBox.checked = cumulative;
    preview();
  }

  function preview(): void {
    const t = chosen();
    const node = layers.get(layer);
    remove.disabled = !node?.time;
    if (!t) {
      save.disabled = true;
      return replaceChildren(summary, summaryLine('info', names().length ? 'Nesnelerin başlangıç tarihini tutan alanı seçin.' : 'Katmanın nesnelerinde öznitelik yok; önce tarihleri bir alana yazın.'));
    }
    const problem = layerTimeProblem(t);
    if (problem) {
      save.disabled = true;
      return replaceChildren(summary, summaryLine('warn', `Ayar: ${problem}.`));
    }
    save.disabled = false;
    const list = doc.byLayer(layer);
    const { summary: s } = layerTimes(t.end != null, !!t.cumulative, timeValues(t, list.map((e) => e.attrs)));
    const range = s.extent ? `; kapsam ${showTime(s.extent[0], 'day')} – ${showTime(s.extent[1], 'day')}` : '';
    const lines = [summaryLine(s.timed ? 'ok' : 'warn', `${list.length} nesne: ${s.timed} zamanlı, ${s.timeless} zamansız${range}.`)];
    if (s.unreadable) lines.push(summaryLine('warn', `${s.unreadable} değer tarih olarak okunamadı (05.03.2024, 2024-03-05 ya da 2024-03-05T14:30 gibi yazılmalı); o nesnelerin o ucu boş sayılır.`));
    if (t.end != null && t.cumulative) lines.push(summaryLine('info', 'Birikimli: nesne başlangıcından sonra hep görünür, bitişi göz ardı edilir.'));
    replaceChildren(summary, ...lines);
  }

  const dialog = new Dialog({
    title: TIME_LAYER_TITLE,
    width: 640,
    className: 'dialog--io dialog--time-layer',
    content: [
      h('div', { class: 'io-row' }, layerBox),
      fieldsBox,
      h('div', { class: 'io-row' }, field('Gösterim', h('label', { class: 'io-check' }, cumulativeBox, 'Birikimli (başlangıçtan sonra hep görünür)'))),
      summary,
    ],
    footer: [remove, h('div', { class: 'dialog__spacer' }), cancel, save],
  });
  cumulativeBox.addEventListener('change', () => ((cumulative = cumulativeBox.checked), preview()));
  const write = (time: LayerTime | null) => {
    const result = layersTime.execute({ doc }, { layer, ...(time && { time }) });
    if (result.status !== 'completed') return void ('error' in result && log.warn(result.error.message));
    const name = layers.get(layer)?.name ?? layer;
    if (!result.output.changed) log.info(`“${name}” katmanının zaman ayarı zaten böyle.`);
    else log.success(time ? `“${name}” katmanı zamansal: ${time.start}${time.end ? ` – ${time.end}` : time.cumulative ? ' (birikimli)' : ' (anlık)'}.` : `“${name}” katmanının zamanı kaldırıldı.`);
    ctx.time.refresh();
    dialog.close();
  };
  save.addEventListener('click', () => {
    const t = chosen();
    if (t) write(t);
  });
  remove.addEventListener('click', () => write(null));
  cancel.addEventListener('click', () => dialog.close());
  load();
  build();
}
