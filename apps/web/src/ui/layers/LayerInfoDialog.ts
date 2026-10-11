import '../../styles/io.css';
import '../../styles/attachments.css';
import type { AppContext } from '../../app/context';
import { objectActions, objectCard } from '../../app/objectActions';
import type { InfoCard } from '../../contracts/generated/InfoCard';
import type { LayerAction } from '../../contracts/generated/LayerAction';
import type { Entity } from '../../model/entities';
import { compileTemplate } from '../../model/infoTemplates';
import { actionsProblem, cardProblem } from '../../model/layerInfoRules';
import { layersInfo } from '../../product/layersInfo';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { checkField, field, select, summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const LAYER_INFO_TITLE = 'Bilgi kartı ve eylemler';

/**
 * Bilgi kartı ve eylemler (docs/adr/0225 §4–§6; the desktop's `layer_info.rs`): a layer's card (Başlık, Alanlar, Metin,
 * Ekleri göster, Eylemleri göster) and its actions (a row each: Ad and Hedef, moved up and down, taken away), with the
 * card and the actions of an object of the layer (the one selected there, else its first) shown as they change; a
 * broken template is said in its place. Kaydet writes through `cad.layers.info` (one step “Bilgi kartı”).
 */
export function openLayerInfo(ctx: AppContext, layerId?: string): void {
  const { doc, log, selection } = ctx;
  const layers = doc.layers;
  const candidates = layers.leaves().filter((l) => !l.service);
  if (!candidates.length) return void log.warn('Çizimde nesne tutan katman yok.');
  const first = layerId ?? layers.active.value;
  let layer = candidates.find((l) => l.id === first)?.id ?? candidates[0].id;
  let title = '';
  let fields = '';
  let text = '';
  let showAttachments = true;
  let showActions = true;
  let actions: LayerAction[] = [];

  const load = () => {
    const node = layers.get(layer);
    const c = node?.card;
    title = c?.title ?? '';
    fields = (c?.fields ?? []).join(', ');
    text = c?.text ?? '';
    showAttachments = c?.attachments !== false;
    showActions = c?.actions !== false;
    actions = (node?.actions ?? []).map((a) => ({ ...a }));
  };
  /** The card as typed: what is empty is not given; a card with nothing in it is none (the command says so too). */
  const card = (): InfoCard => {
    const names = fields
      .split(',')
      .map((f) => f.trim())
      .filter(Boolean);
    return {
      ...(title.trim() && { title: title.trim() }),
      ...(names.length && { fields: names }),
      ...(text.trim() && { text: text.replace(/\r\n?/g, '\n') }),
      ...(!showAttachments && { attachments: false }),
      ...(!showActions && { actions: false }),
    };
  };
  /** The object the preview shows: the one selected on the layer, else its first. */
  const sample = (): Entity | undefined => {
    for (const id of selection.ids.value) {
      const e = doc.get(id);
      if (e?.layerId === layer) return e;
    }
    return doc.byLayer(layer)[0];
  };

  const layerBox = h('div', { class: 'lfilter__layer' });
  const titleInput = h('input', { class: 'field', type: 'text', 'aria-label': 'Başlık', placeholder: 'Parsel {Ada}/{Parsel}', spellcheck: 'false' }) as HTMLInputElement;
  const fieldsInput = h('input', { class: 'field', type: 'text', 'aria-label': 'Alanlar', placeholder: 'Malik, Nitelik', spellcheck: 'false' }) as HTMLInputElement;
  const textInput = h('textarea', { class: 'field attachment-dialog__note', rows: '3', 'aria-label': 'Metin', placeholder: 'Alan: {yuvarla($alan, 2)} m²', spellcheck: 'false' }) as HTMLTextAreaElement;
  const switches = h('div', { class: 'io-row' });
  const actionList = h('div', { class: 'layer-info__actions' });
  const addAction = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), h('span', null, 'Eylem ekle'));
  const preview = h('div', { class: 'layer-info__preview info-card' });
  const summary = h('div', { class: 'io-summary' });
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');

  function build(): void {
    replaceChildren(
      layerBox,
      field('Katman', select('Katman', candidates.map((l) => ({ value: l.id, label: layers.path(l.id) })), layer, (v) => ((layer = v), load(), build())), undefined, 'grow'),
    );
    titleInput.value = title;
    fieldsInput.value = fields;
    textInput.value = text;
    replaceChildren(
      switches,
      checkField('Ekler', 'Ekleri göster', showAttachments, (v) => ((showAttachments = v), refresh()), 'attachments'),
      checkField('Eylemler', 'Eylemleri göster', showActions, (v) => ((showActions = v), refresh()), 'actions'),
    );
    buildActions();
    refresh();
  }

  function buildActions(): void {
    replaceChildren(
      actionList,
      ...actions.map((a, i) => {
        const name = h('input', { class: 'field layer-info__name', type: 'text', value: a.name, 'aria-label': `${i + 1}. eylemin adı`, placeholder: 'Ad' }) as HTMLInputElement;
        const target = h('input', { class: 'field layer-info__target', type: 'text', value: a.target, 'aria-label': `${i + 1}. eylemin hedefi`, placeholder: 'https://… {Ada} ya da dosya yolu', spellcheck: 'false' }) as HTMLInputElement;
        name.addEventListener('input', () => ((actions[i] = { ...actions[i], name: name.value }), refresh()));
        target.addEventListener('input', () => ((actions[i] = { ...actions[i], target: target.value }), refresh()));
        const move = (d: number) => {
          const j = i + d;
          if (j < 0 || j >= actions.length) return;
          [actions[i], actions[j]] = [actions[j], actions[i]];
          buildActions();
          refresh();
        };
        const up = h('button', { class: 'ibtn', type: 'button', title: 'Yukarı', 'aria-label': 'Yukarı', disabled: i === 0 }, icon('chevronUp', 14));
        const down = h('button', { class: 'ibtn', type: 'button', title: 'Aşağı', 'aria-label': 'Aşağı', disabled: i === actions.length - 1 }, icon('chevronDown', 14));
        const drop = h('button', { class: 'ibtn', type: 'button', title: 'Kaldır', 'aria-label': 'Kaldır' }, icon('close', 14));
        up.addEventListener('click', () => move(-1));
        down.addEventListener('click', () => move(1));
        drop.addEventListener('click', () => {
          actions.splice(i, 1);
          buildActions();
          refresh();
        });
        return h('div', { class: 'layer-info__row' }, name, target, up, down, drop);
      }),
      actions.length ? null : h('div', { class: 'layer-info__empty' }, 'Eylem yok.'),
    );
  }

  /** The draft checked as the command checks it, and the sample object's card and actions shown with it. */
  function refresh(): void {
    title = titleInput.value;
    fields = fieldsInput.value;
    text = textInput.value;
    const c = card();
    const trimmed = actions.map((a) => ({ name: a.name.trim(), target: a.target.trim() })).filter((a) => a.name || a.target);
    const problems: string[] = [];
    const ap = actionsProblem(trimmed);
    if (ap) problems.push(`Eylemler: ${ap}.`);
    const cp = cardProblem(c);
    if (cp) problems.push(`Kart: ${cp}.`);
    for (const a of trimmed) {
      const t = compileTemplate(a.target);
      if (!t.ok) problems.push(`“${a.name}” eyleminin hedefi: ${t.error}`);
    }
    for (const [source, what] of [
      [c.title, 'Başlık'],
      [c.text, 'Metin'],
    ] as const) {
      if (source == null) continue;
      const t = compileTemplate(source);
      if (!t.ok) problems.push(`${what}: ${t.error}`);
    }
    save.disabled = problems.length > 0;
    const e = sample();
    if (!e) replaceChildren(preview, h('div', { class: 'layer-info__empty' }, 'Katmanda nesne yok: önizleme yok.'));
    else {
      const shown = objectCard(ctx, e, c);
      const filled = shown.actions ? objectActions(ctx, e, trimmed) : [];
      replaceChildren(
        preview,
        h('div', { class: 'info-card__title' }, shown.title),
        shown.rows.length ? h('dl', { class: 'info-card__rows' }, shown.rows.map(([k, v]) => [h('dt', null, k), h('dd', null, v || '—')])) : null,
        shown.text ? h('div', { class: 'info-card__text' }, shown.text) : null,
        shown.attachments && e.attachments?.length ? h('div', { class: 'info-card__text' }, `${e.attachments.length} ek`) : null,
        filled.length ? h('div', { class: 'info-card__links' }, filled.map((a) => h('div', { title: a.target ?? '' }, h('b', null, `${a.name}: `), a.target ?? a.error ?? ''))) : null,
      );
    }
    replaceChildren(summary, ...(problems.length ? problems.map((p) => summaryLine('warn', p)) : [summaryLine('ok', e ? 'Önizleme katmanın bir nesnesinde.' : 'Kart ve eylemler yazılmaya hazır.')]));
  }

  const dialog = new Dialog({
    title: LAYER_INFO_TITLE,
    width: 760,
    className: 'dialog--io dialog--layer-info',
    content: [
      h('div', { class: 'io-row' }, layerBox),
      h('div', { class: 'layer-info__grid' },
        h('div', { class: 'layer-info__form' },
          h('div', { class: 'io-row' }, field('Başlık', titleInput, 'Şablon: {…} arası bir ifadedir. Boşsa nesnenin türü ve etiketi.', 'grow')),
          h('div', { class: 'io-row' }, field('Alanlar', fieldsInput, 'Virgülle ayırın; boşsa bütün öznitelikler.', 'grow')),
          h('div', { class: 'io-row' }, field('Metin', textInput, '{{ ve }} düz ayraç; {$enlem}, {$boylam} nesnenin WGS 84’teki yeri.', 'grow')),
          switches,
          h('div', { class: 'io-row layer-info__head' }, h('b', null, 'Eylemler'), addAction),
          actionList,
        ),
        h('div', { class: 'layer-info__side' }, h('div', { class: 'layer-info__caption' }, 'Önizleme'), preview),
      ),
      summary,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
  });
  for (const input of [titleInput, fieldsInput, textInput]) input.addEventListener('input', refresh);
  addAction.addEventListener('click', () => {
    actions.push({ name: '', target: '' });
    buildActions();
    refresh();
    actionList.querySelector<HTMLInputElement>('.layer-info__row:last-child .layer-info__name')?.focus();
  });
  cancel.addEventListener('click', () => dialog.close());
  save.addEventListener('click', () => {
    refresh();
    if (save.disabled) return;
    const list = actions.map((a) => ({ name: a.name.trim(), target: a.target.trim() })).filter((a) => a.name || a.target);
    const c = card();
    const result = layersInfo.execute({ doc }, { layer, ...(list.length && { actions: list }), ...(Object.keys(c).length && { card: c }) });
    if (result.status !== 'completed') return void ('error' in result && log.warn(result.error.message));
    const name = layers.get(layer)?.name ?? layer;
    if (!result.output.changed) log.info(`“${name}” katmanının bilgi kartı ve eylemleri zaten böyle.`);
    else log.success(`“${name}” katmanının bilgi kartı ve eylemleri yazıldı.`);
    dialog.close();
  });
  load();
  build();
  titleInput.focus();
}
