import '../../styles/io.css';
import '../../styles/labels.css';
import type { AppContext } from '../../app/context';
import type { LabelClass } from '../../contracts/generated/LabelClass';
import type { LabelObstacle } from '../../contracts/generated/LabelObstacle';
import type { LabelStyle } from '../../contracts/generated/LabelStyle';
import type { LayerLabels } from '../../contracts/generated/LayerLabels';
import type { LayersLabels } from '../../contracts/generated/LayersLabels';
import { attributeFields, entityObjects } from '../../model/expression/builderObjects';
import { DEFAULT_LABELS } from '../../model/labelDefaults';
import { labelStyleProblem, layerLabelsProblem } from '../../model/labelRules';
import { labelExpression } from '../../model/labelTexts';
import { layersLabels } from '../../product/layersLabels';
import { builderButton } from '../expression/builderApi';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { field, select, summaryLine } from '../io/common';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { LABEL_TABS, tabRows, type LabelTab, type StylePatch } from './labelForm';

/** The window's title, which a trace names it by. */
export const LABELS_TITLE = 'Etiketler';

type Mode = 'single' | 'rules' | 'off';

/** The style a layer's single label starts from: its own, else its objects' most common kind's default. */
function startingStyle(ctx: AppContext, layer: string): LabelStyle {
  const own = ctx.doc.layers.get(layer)?.style.label;
  if (own) return structuredClone(own);
  const counts = new Map<string, number>();
  for (const e of ctx.doc.byLayer(layer).slice(0, 2000)) counts.set(e.kind, (counts.get(e.kind) ?? 0) + 1);
  const kind = [...counts].sort((a, b) => b[1] - a[1])[0]?.[0] as keyof typeof DEFAULT_LABELS | undefined;
  const fallback = kind && DEFAULT_LABELS[kind];
  return structuredClone(fallback ?? { placement: 'center', size: 10 });
}

/** A new class's name: “Sınıf n”, the first free. */
function freeName(classes: readonly LabelClass[]): string {
  for (let n = classes.length + 1; ; n++) if (!classes.some((c) => c.name === `Sınıf ${n}`)) return `Sınıf ${n}`;
}

/**
 * Etiketler (docs/adr/0212 §4; the desktop's `labelling.rs`): a layer's labelling. Katman (the active one, or the one
 * the layer tree's menu named); Etiketleme: Tek etiket, Kurallı (an ordered list of classes, each its name, its
 * condition and its own style) or Yok; the style in five tabs (Metin, Yerleşim, Biçim, Sığdırma, Öncelik); Engel: the
 * layer's objects as obstacles to every label (their weight, an area's inside or only its outline). Uygula writes
 * through `cad.layers.labels` (one undo step “Etiketler”) and leaves the window open; Kaydet writes and closes.
 */
export function openLabels(ctx: AppContext, layerId?: string): void {
  const { doc, log } = ctx;
  const layers = doc.layers;
  const candidates = layers.leaves().filter((l) => !l.service);
  if (!candidates.length) return void log.warn('Çizimde nesne tutan katman yok.');
  let layer = candidates.find((l) => l.id === (layerId ?? layers.active.value))?.id ?? candidates[0].id;
  let mode: Mode = 'single';
  let single: LabelStyle = { placement: 'center', size: 10 };
  let classes: LabelClass[] = [];
  let selected = 0;
  let obstacle: LabelObstacle | null = null;
  let tab: LabelTab = 'text';
  const geometry = { evaluateExpression: (...a: Parameters<typeof ctx.view.evaluateExpression>) => ctx.view.evaluateExpression(...a) };

  const load = () => {
    const node = layers.get(layer);
    const labels = node?.style.labels;
    mode = labels?.mode ?? 'single';
    single = startingStyle(ctx, layer);
    classes = structuredClone(labels?.classes ?? []);
    if (!classes.length) classes = [{ name: 'Sınıf 1', style: structuredClone(single) }];
    selected = 0;
    obstacle = labels?.obstacle ? structuredClone(labels.obstacle) : null;
  };

  /** The style the form edits: the single label's or the selected class's. */
  const current = (): LabelStyle => (mode === 'rules' ? classes[selected].style : single);

  /** What `cad.layers.labels` is given. */
  const input = (): LayersLabels => {
    const ob = obstacle ? { obstacle } : {};
    if (mode === 'rules') return { layer, labels: { mode: 'rules', classes, ...ob } };
    if (mode === 'off') return { layer, labels: { mode: 'off', ...ob } };
    return { layer, label: single, labels: obstacle ? ({ mode: 'single', ...ob } as LayerLabels) : null };
  };

  const fieldsOf = () => {
    const counts = new Map<string, number>();
    for (const f of layers.get(layer)?.fields ?? []) counts.set(f.name, 0);
    for (const e of doc.byLayer(layer).slice(0, 2000)) for (const k of Object.keys(e.attrs)) counts.set(k, (counts.get(k) ?? 0) + 1);
    return attributeFields([...counts].map(([name, count]) => ({ name, count })));
  };
  const expressionButton = (get: () => string, put: (v: string) => void) =>
    builderButton({
      get,
      set: put,
      fields: fieldsOf,
      objects: () => entityObjects(doc.byLayer(layer).slice(0, 5000), (id) => layers.get(id)?.name ?? id, { geometry }),
      context: `${LABELS_TITLE} · İfade`,
      fail: (m) => log.error(m),
    });

  const head = h('div', { class: 'lbl-head' });
  const classesBox = h('div', { class: 'lbl-classes' });
  const tabsBox = h('div', { class: 'lbl-tabs', role: 'tablist', 'aria-label': 'Etiketin stili' });
  const formBox = h('div', { class: 'lbl-form', role: 'tabpanel' });
  const obstacleBox = h('div', { class: 'lbl-obstacle' });
  const summary = h('div', { class: 'io-summary' });
  const apply = h('button', { class: 'btn', type: 'button' }, 'Uygula');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');
  const body = h('div', { class: 'lbl-body' }, classesBox, h('div', { class: 'lbl-style' }, tabsBox, formBox));

  /** A change of the edited style: the patch merged, an undefined field taken away. */
  const set = (patch: StylePatch) => {
    const s = current() as unknown as Record<string, unknown>;
    for (const [k, v] of Object.entries(patch)) {
      if (v === undefined) delete s[k];
      else s[k] = v;
    }
    buildForm();
    check();
  };

  function buildHead(): void {
    replaceChildren(
      head,
      field('Katman', select('Katman', candidates.map((l) => ({ value: l.id, label: layers.path(l.id) })), layer, (v) => ((layer = v), load(), build())), undefined, 'grow'),
      field(
        'Etiketleme',
        segmented({
          label: 'Etiketleme',
          options: [
            { value: 'single', label: 'Tek etiket', hint: 'Katmanın bütün nesneleri tek bir stille etiketlenir.' },
            { value: 'rules', label: 'Kurallı', hint: 'Sıralı sınıflar: koşulu tutan her sınıf nesneyi kendi stiliyle etiketler.' },
            { value: 'off', label: 'Yok', hint: 'Katman etiketlenmez; nesneleri yine de öbür etiketlere engel olabilir.' },
          ],
          value: mode,
          onChange: (v) => ((mode = v), build()),
        }),
      ),
    );
  }

  function buildClasses(): void {
    classesBox.hidden = mode !== 'rules';
    if (mode !== 'rules') return void replaceChildren(classesBox);
    const list = h(
      'div',
      { class: 'lbl-classlist', role: 'listbox', 'aria-label': 'Sınıflar' },
      classes.map((c, i) => {
        const b = h('button', { class: 'lbl-class', type: 'button', role: 'option', 'aria-selected': String(i === selected), title: c.when ? `${c.name}\n${c.when}` : c.name }, h('span', { class: 'lbl-class__name' }, c.name), c.when ? h('span', { class: 'lbl-class__when' }, c.when) : null);
        b.addEventListener('click', () => ((selected = i), build()));
        return b;
      }),
    );
    const tool = (name: Parameters<typeof icon>[0], title: string, disabled: boolean, run: () => void) => {
      const b = h('button', { class: 'ibtn', type: 'button', title, 'aria-label': title, disabled }, icon(name, 16));
      b.addEventListener('click', run);
      return b;
    };
    const c = classes[selected];
    const name = h('input', { class: 'field', value: c.name, spellcheck: 'false', 'aria-label': 'Sınıfın adı' }) as HTMLInputElement;
    name.addEventListener('change', () => ((c.name = name.value.trim()), buildClasses(), check()));
    const when = h('input', { class: 'field', value: c.when ?? '', spellcheck: 'false', placeholder: 'her nesne', 'aria-label': 'Sınıfın koşulu' }) as HTMLInputElement;
    const setWhen = (v: string) => {
      if (v.trim()) c.when = v.trim();
      else delete c.when;
      buildClasses();
      check();
    };
    when.addEventListener('change', () => setWhen(when.value));
    replaceChildren(
      classesBox,
      h('div', { class: 'lbl-classes__title' }, 'Sınıflar'),
      list,
      h(
        'div',
        { class: 'lbl-classes__tools' },
        tool('plus', 'Sınıf ekle', classes.length >= 64, () => {
          classes.push({ name: freeName(classes), style: structuredClone(single) });
          selected = classes.length - 1;
          build();
        }),
        tool('copy', 'Sınıfın kopyası', classes.length >= 64, () => {
          classes.splice(selected + 1, 0, { ...structuredClone(c), name: freeName(classes) });
          selected++;
          build();
        }),
        tool('chevronUp', 'Yukarı', selected === 0, () => {
          [classes[selected - 1], classes[selected]] = [classes[selected], classes[selected - 1]];
          selected--;
          build();
        }),
        tool('chevronDown', 'Aşağı', selected === classes.length - 1, () => {
          [classes[selected + 1], classes[selected]] = [classes[selected], classes[selected + 1]];
          selected++;
          build();
        }),
        tool('trash', 'Sınıfı sil', classes.length <= 1, () => {
          classes.splice(selected, 1);
          selected = Math.min(selected, classes.length - 1);
          build();
        }),
      ),
      h('label', { class: 'lbl-row' }, h('span', { class: 'lbl-row__name' }, 'Ad'), h('span', { class: 'lbl-row__control' }, name)),
      h(
        'label',
        { class: 'lbl-row' },
        h('span', { class: 'lbl-row__name' }, 'Koşul'),
        h('span', { class: 'lbl-row__control lbl-exprbox' }, when, expressionButton(() => when.value, (v) => ((when.value = v), setWhen(v)))),
      ),
    );
  }

  function buildTabs(): void {
    replaceChildren(
      tabsBox,
      LABEL_TABS.map(([id, label]) => {
        const b = h('button', { class: 'tab', type: 'button', role: 'tab', 'aria-selected': String(id === tab) }, label);
        b.addEventListener('click', () => ((tab = id), buildTabs(), buildForm()));
        return b;
      }),
    );
  }

  function buildForm(): void {
    if (mode === 'off') return void replaceChildren(formBox, h('p', { class: 'lbl-off' }, 'Katman etiketlenmez. Nesnelerinin öbür etiketlere engel olması için aşağıdaki Engel’i açın.'));
    replaceChildren(formBox, tabRows(tab, { style: current(), set, expression: expressionButton }));
  }

  function buildObstacle(): void {
    const on = h('input', { type: 'checkbox', checked: !!obstacle, 'aria-label': 'Nesneler etiketlere engel' }) as HTMLInputElement;
    on.addEventListener('change', () => ((obstacle = on.checked ? { weight: 5 } : null), buildObstacle(), check()));
    const parts: HTMLElement[] = [h('label', { class: 'lbl-check' }, on, h('span', null, 'Nesneleri öbür etiketlere engel'))];
    if (obstacle) {
      const ob = obstacle;
      const weight = h('input', { class: 'field lbl-num num', value: String(ob.weight), inputmode: 'numeric', 'aria-label': 'Engelin ağırlığı' }) as HTMLInputElement;
      weight.addEventListener('change', () => {
        const w = Math.round(Number(weight.value));
        if (Number.isFinite(w) && w >= 1 && w <= 10) ob.weight = w;
        else weight.value = String(ob.weight);
        check();
      });
      parts.push(
        h('span', { class: 'lbl-obstacle__weight' }, h('span', null, 'Ağırlık'), weight, h('span', { class: 'lbl-unit' }, '1–10')),
        segmented({
          label: 'Engelin türü',
          options: [
            { value: 'interior', label: 'Alanın içi', hint: 'Alanın içine etiket konmaz.' },
            { value: 'boundary', label: 'Yalnız sınırı', hint: 'Etiket alanın içine konabilir, sınırını kesmez.' },
          ],
          value: ob.kind ?? 'interior',
          onChange: (v) => {
            if (v === 'boundary') ob.kind = 'boundary';
            else delete ob.kind;
            buildObstacle();
            check();
          },
        }),
      );
    }
    replaceChildren(obstacleBox, h('span', { class: 'lbl-obstacle__title' }, 'Engel'), ...parts, obstacle ? h('span', { class: 'lbl-hint' }, 'Önceliği ağırlıktan küçük etiket engeli örtemez; büyüğü örtebilir ama başka yeri yeğler.') : null);
  }

  /** The labelling checked as the contract does (its rules, its expressions); Kaydet and Uygula follow it. */
  function check(): void {
    const given = input();
    const words: string[] = [];
    if (given.label) {
      const p = labelStyleProblem(given.label);
      if (p) words.push(`Etiketin stili: ${p}.`);
      if (given.label.text !== undefined) {
        const e = labelExpression(given.label.text).error;
        if (e) words.push(`Etiketin metni: ${e}`);
      }
    }
    if (given.labels) {
      const p = layerLabelsProblem(given.labels);
      if (p) words.push(`Etiketleme: ${p}.`);
      for (const c of given.labels.classes ?? []) {
        const w = c.when !== undefined ? labelExpression(c.when).error : null;
        if (w) words.push(`“${c.name}” sınıfının koşulu: ${w}`);
        const t = c.style.text !== undefined ? labelExpression(c.style.text).error : null;
        if (t) words.push(`“${c.name}” sınıfının metni: ${t}`);
      }
    }
    save.disabled = apply.disabled = words.length > 0;
    if (words.length) return replaceChildren(summary, ...words.slice(0, 3).map((w) => summaryLine('warn', w)));
    const n = doc.byLayer(layer).length;
    const what = mode === 'rules' ? `${classes.length} sınıf` : mode === 'off' ? 'etiketsiz' : 'tek etiket';
    replaceChildren(summary, summaryLine('ok', `${layers.get(layer)?.name ?? layer}: ${n} nesne, ${what}${obstacle ? `, engel (ağırlık ${obstacle.weight})` : ''}.`));
  }

  function build(): void {
    buildHead();
    buildClasses();
    buildTabs();
    buildForm();
    buildObstacle();
    check();
  }

  const dialog = new Dialog({
    title: LABELS_TITLE,
    width: 860,
    className: 'dialog--io dialog--labels',
    content: [head, body, obstacleBox, summary],
    footer: [h('div', { class: 'dialog__spacer' }), apply, cancel, save],
  });

  /** Writes the labelling; whether it went (or nothing changed). */
  const write = (): boolean => {
    check();
    if (save.disabled) return false;
    const r = layersLabels.execute({ doc }, input());
    if (r.status !== 'completed') {
      if ('error' in r) log.warn(r.error.message);
      return false;
    }
    const name = layers.get(layer)?.name ?? layer;
    if (r.output.changed) log.success(`“${name}” katmanının etiketlemesi yazıldı.`);
    else log.info(`“${name}” katmanının etiketlemesi zaten böyle.`);
    return true;
  };
  apply.addEventListener('click', () => void write());
  save.addEventListener('click', () => write() && dialog.close());
  cancel.addEventListener('click', () => dialog.close());
  load();
  build();
}
