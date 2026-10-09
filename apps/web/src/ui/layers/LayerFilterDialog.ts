import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import { clearFilter } from '../../app/layerFilterCommands';
import type { LayerFilter } from '../../contracts/generated/LayerFilter';
import { attributeFields, entityObjects } from '../../model/expression/builderObjects';
import { compileFilter, filterPassesIn } from '../../model/layerFilter';
import { layerFilterProblem } from '../../model/layerFilterRules';
import { layersFilter } from '../../product/layersFilter';
import { builderButton } from '../expression/builderApi';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { field, select, summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const LAYER_FILTER_TITLE = 'Katman süzgeci';

/** How long typing rests before a large layer's objects are counted again; a smaller one is counted as it is typed. */
const PREVIEW_DELAY = 150;
const COUNT_AT_ONCE = 20_000;

/**
 * Katman süzgeci (docs/adr/0211 §4; the desktop's `layer_filters.rs`): Katman (the active one, or the one the layer
 * tree's menu named), İfade (İfadeyle seç's language; ε opens the expression builder on the layer's objects), the
 * object list (Seçimden al: the layer's selected objects; Listeyi kaldır) and, as they change, how many of the layer's
 * objects pass or why the condition does not compile. Kaydet writes through `cad.layers.filter` (one step “Katman
 * süzgeci”); Süzgeci kaldır takes the filter away.
 */
export function openLayerFilter(ctx: AppContext, layerId?: string): void {
  const { doc, log, selection } = ctx;
  const layers = doc.layers;
  const candidates = layers.leaves().filter((l) => !l.service);
  if (!candidates.length) return void log.warn('Çizimde nesne tutan katman yok.');
  const first = layerId ?? layers.active.value;
  let layer = candidates.find((l) => l.id === first)?.id ?? candidates[0].id;
  let expression = '';
  let objects: string[] = [];
  const geometry = { evaluateExpression: (...a: Parameters<typeof ctx.view.evaluateExpression>) => ctx.view.evaluateExpression(...a) };

  const load = () => {
    const f = layers.get(layer)?.filter;
    expression = f?.expression ?? '';
    objects = [...(f?.objects ?? [])];
  };
  /** The layer's selected objects' persistent ids. */
  const selectedHere = (): string[] => {
    const out: string[] = [];
    for (const id of selection.ids.value) {
      const e = doc.get(id);
      const uid = e?.layerId === layer ? doc.uidOf(id) : null;
      if (uid) out.push(uid);
    }
    return out;
  };
  /** The filter as chosen; null when it keeps nothing (no condition, no list). */
  const chosen = (): LayerFilter | null => {
    const e = expression.trim();
    return e || objects.length ? { ...(e && { expression: e }), ...(objects.length && { objects }) } : null;
  };

  const layerBox = h('div', { class: 'lfilter__layer' });
  const exprInput = h('input', {
    class: 'field lfilter__expr',
    type: 'text',
    spellcheck: 'false',
    placeholder: "Nitelik = 'Arsa' ve $alan > 500",
    'aria-label': 'İfade',
  }) as HTMLInputElement;
  const builder = builderButton({
    get: () => exprInput.value,
    set: (v) => {
      exprInput.value = v;
      expression = v;
      preview();
    },
    fields: () => {
      const counts = new Map<string, number>();
      for (const f of layers.get(layer)?.fields ?? []) counts.set(f.name, 0);
      for (const e of doc.byLayer(layer).slice(0, 2000)) for (const k of Object.keys(e.attrs)) counts.set(k, (counts.get(k) ?? 0) + 1);
      return attributeFields([...counts].map(([name, count]) => ({ name, count })));
    },
    objects: () => entityObjects(doc.byLayer(layer).slice(0, 5000), (id) => layers.get(id)?.name ?? id, { geometry }),
    context: `${LAYER_FILTER_TITLE} · İfade`,
    fail: (m) => log.error(m),
  });
  const listBox = h('div', { class: 'lfilter__list' });
  const summary = h('div', { class: 'io-summary' });
  const remove = h('button', { class: 'btn', type: 'button' }, 'Süzgeci kaldır');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');

  function build(): void {
    replaceChildren(
      layerBox,
      field('Katman', select('Katman', candidates.map((l) => ({ value: l.id, label: layers.path(l.id) })), layer, (v) => ((layer = v), load(), build())), undefined, 'grow'),
    );
    exprInput.value = expression;
    buildList();
    preview();
  }

  /** The list's row: how many objects it names and its two buttons. */
  function buildList(): void {
    const here = selectedHere();
    const take = h('button', { class: 'btn btn--small', type: 'button', disabled: !here.length, title: here.length ? '' : 'Bu katmanda seçili nesne yok' }, icon('layerFilterSelection', 14), h('span', null, here.length ? `Seçimden al (${here.length})` : 'Seçimden al'));
    take.addEventListener('click', () => {
      objects = selectedHere();
      buildList();
      preview();
    });
    const drop = h('button', { class: 'btn btn--small', type: 'button', disabled: !objects.length }, icon('close', 12), h('span', null, 'Listeyi kaldır'));
    drop.addEventListener('click', () => {
      objects = [];
      buildList();
      preview();
    });
    replaceChildren(listBox, h('span', { class: 'lfilter__count' }, objects.length ? `Seçimden: ${objects.length} nesne` : 'Liste yok: ifadeye uyan bütün nesneler'), take, drop);
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  /**
   * The filter checked at once (Kaydet follows it as it is typed); how many objects pass counted at once, or after
   * typing rests (`later`): a large layer takes a while.
   */
  function preview(later = false): void {
    clearTimeout(timer);
    const node = layers.get(layer);
    remove.disabled = !node?.filter;
    exprInput.classList.remove('is-invalid');
    const f = chosen();
    if (!f) {
      save.disabled = true;
      return replaceChildren(summary, summaryLine('info', 'Bir ifade yazın ya da seçimden bir liste alın. Süzgeci kaldırmak için Süzgeci kaldır.'));
    }
    const problem = layerFilterProblem(f);
    if (problem) {
      save.disabled = true;
      return replaceChildren(summary, summaryLine('warn', `Süzgeç: ${problem}.`));
    }
    const r = compileFilter(f);
    if (!r.ok) {
      save.disabled = true;
      exprInput.classList.add('is-invalid');
      return replaceChildren(summary, summaryLine('error', `İfade: ${r.error}`));
    }
    save.disabled = false;
    const count = () => {
      const list = doc.byLayer(layer);
      const pass = filterPassesIn(doc, r.filter, list, (id) => layers.get(id)?.name ?? id, geometry);
      const passed = pass.reduce((n, p) => n + (p ? 1 : 0), 0);
      const lines = [summaryLine(passed ? 'ok' : 'warn', `${passed} / ${list.length} nesne süzgeçten geçiyor${passed ? '' : '; katmanda hiçbir nesne görünmeyecek'}.`)];
      if (objects.length) {
        const known = objects.reduce((n, u) => n + (doc.byUid(u) != null ? 1 : 0), 0);
        if (known < objects.length) lines.push(summaryLine('info', `Listedeki ${objects.length - known} nesne çizimde yok (silinmiş olabilir); listede kalır.`));
      }
      replaceChildren(summary, ...lines);
    };
    if (later && doc.byLayer(layer).length > COUNT_AT_ONCE) timer = setTimeout(count, PREVIEW_DELAY);
    else count();
  }

  const dialog = new Dialog({
    title: LAYER_FILTER_TITLE,
    width: 640,
    className: 'dialog--io dialog--layer-filter',
    content: [
      h('div', { class: 'io-row' }, layerBox),
      h('div', { class: 'io-row' }, field('İfade', h('span', { class: 'lfilter__exprbox' }, exprInput, builder), 'İfadeyle seç’in dili: uyan nesneler görünür. $sıra ve $ölçek kullanılamaz.', 'grow')),
      h('div', { class: 'io-row' }, field('Nesne listesi', listBox, undefined, 'grow')),
      summary,
    ],
    footer: [remove, h('div', { class: 'dialog__spacer' }), cancel, save],
  });
  exprInput.addEventListener('input', () => {
    expression = exprInput.value;
    preview(true);
  });
  exprInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !save.disabled) {
      e.preventDefault();
      write();
    }
  });
  const write = () => {
    preview();
    const f = chosen();
    if (!f || save.disabled) return;
    const result = layersFilter.execute({ doc }, { layer, filter: f });
    if (result.status !== 'completed') return void ('error' in result && log.warn(result.error.message));
    const name = layers.get(layer)?.name ?? layer;
    if (!result.output.changed) log.info(`“${name}” katmanının süzgeci zaten böyle.`);
    else log.success(`“${name}” katmanının süzgeci yazıldı: ${result.output.passed} / ${result.output.total} nesne görünür.`);
    dialog.close();
  };
  save.addEventListener('click', write);
  remove.addEventListener('click', () => {
    const node = layers.get(layer);
    if (node) clearFilter(ctx, node);
    dialog.close();
  });
  cancel.addEventListener('click', () => (clearTimeout(timer), dialog.close()));
  load();
  build();
  exprInput.focus();
}
