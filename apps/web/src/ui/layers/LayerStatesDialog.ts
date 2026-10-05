import type { AppContext } from '../../app/context';
import {
  applyLayerState,
  nextStateName,
  partsOf,
  partsText,
  removeLayerState,
  renameLayerState,
  saveLayerState,
  stateMatches,
  statesLocked,
  updateLayerState,
} from '../../app/layerStates';
import { DisposableStore } from '../../core/disposable';
import { h, replaceChildren } from '../dom';
import { field, summaryLine } from '../io/common';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const STATES_TITLE = 'Katman durumları';

/**
 * Katman durumları (docs/adr/0177 §4; the desktop's `layer_states.rs`): the project's saved states, each with what it
 * keeps and a mark when the layers are in it now. A state is chosen by a click (its name and parts come into the fields
 * below), applied with Uygula or a double click, saved again from the layers as they are with Güncelle, removed with
 * Sil. Yeni durum kaydet saves the layers as a new state under the name in Ad, keeping their visibility and, when
 * checked, their locks and styles; Yeniden adlandır gives the chosen state that name. The window stays open; Kapat
 * closes it.
 */
export function openLayerStates(ctx: AppContext): void {
  const { doc } = ctx;
  const d = new DisposableStore();
  let chosen: string | null = null;

  const name = h('input', { class: 'field', type: 'text', 'aria-label': 'Ad', spellcheck: 'false' }) as HTMLInputElement;
  const locks = h('input', { type: 'checkbox' }) as HTMLInputElement;
  const styles = h('input', { type: 'checkbox', checked: true }) as HTMLInputElement;
  const list = h('div', { class: 'states-list', role: 'listbox', 'aria-label': 'Kayıtlı durumlar' });
  const rights = h('div', { class: 'io-summary' });
  const button = (words: string, primary = false) => h('button', { class: primary ? 'btn btn--primary' : 'btn', type: 'button' }, words) as HTMLButtonElement;
  const apply = button('Uygula');
  const update = button('Güncelle');
  const remove = button('Sil');
  const save = button('Yeni durum kaydet', true);
  const rename = button('Yeniden adlandır');
  const close = button('Kapat');

  const states = () => doc.settings.layerStates.value;
  const parts = () => ({ locks: locks.checked, styles: styles.checked });

  function choose(id: string | null): void {
    chosen = id;
    const state = states().find((s) => s.id === id);
    if (state) {
      name.value = state.name;
      const p = partsOf(state);
      locks.checked = p.locks;
      styles.checked = p.styles;
    }
    refresh();
  }

  function rowOf(id: string): HTMLElement {
    const state = states().find((s) => s.id === id)!;
    const now = stateMatches(ctx, state);
    const row = h(
      'button',
      { class: 'states-row', type: 'button', role: 'option', 'aria-label': state.name, 'aria-selected': String(id === chosen), ...(now && { title: 'Katmanlar şimdi bu durumda.' }) },
      h('span', { class: 'states-row__mark' }, now ? icon('check', 14) : null),
      h('span', { class: 'states-row__name' }, state.name),
      h('span', { class: 'states-row__parts' }, partsText(partsOf(state))),
    );
    row.addEventListener('click', () => choose(id));
    row.addEventListener('dblclick', () => applyLayerState(ctx, id));
    return row;
  }

  function refresh(): void {
    if (chosen && !states().some((s) => s.id === chosen)) chosen = null;
    replaceChildren(
      list,
      ...(states().length ? states().map((s) => rowOf(s.id)) : [h('p', { class: 'states-list__empty' }, 'Kayıtlı durum yok. Aşağıya bir ad yazıp Yeni durum kaydet’e basın.')]),
    );
    const locked = statesLocked(ctx);
    replaceChildren(rights, ...(locked ? [summaryLine('warn', locked)] : []));
    rights.hidden = !locked;
    apply.disabled = !chosen;
    for (const b of [update, remove]) b.disabled = !chosen || !!locked;
    save.disabled = !!locked;
    rename.disabled = !chosen || !!locked;
  }

  const dialog = new Dialog({
    title: STATES_TITLE,
    width: 520,
    className: 'dialog--io dialog--states',
    content: [
      rights,
      field('Kayıtlı durumlar', list),
      h('div', { class: 'io-row states-actions' }, apply, update, remove),
      h('div', { class: 'io-row' }, field('Ad', name, undefined, 'grow')),
      field(
        'Kaydedilecekler',
        h('div', { class: 'io-row states-parts' }, h('label', { class: 'io-check' }, locks, 'Kilitler'), h('label', { class: 'io-check' }, styles, 'Stiller')),
        'Görünürlük her durumda kaydedilir.',
      ),
      h('div', { class: 'io-row states-actions' }, save, rename),
    ],
    footer: [h('div', { class: 'dialog__spacer' }), close],
    onClose: () => d.dispose(),
  });

  apply.addEventListener('click', () => chosen && applyLayerState(ctx, chosen));
  update.addEventListener('click', () => chosen && updateLayerState(ctx, chosen, parts()));
  remove.addEventListener('click', () => {
    if (chosen && removeLayerState(ctx, chosen)) {
      chosen = null;
      name.value = nextStateName(states());
      refresh();
    }
  });
  save.addEventListener('click', () => {
    const id = saveLayerState(ctx, name.value, parts());
    if (id) choose(id);
  });
  rename.addEventListener('click', () => chosen && renameLayerState(ctx, chosen, name.value));
  close.addEventListener('click', () => dialog.close());
  // The marks follow the layers; the list follows the project's states (another window, the cloud).
  d.add(doc.settings.layerStates.subscribe(() => refresh()));
  d.add(doc.layers.events.on('state', () => refresh()));
  d.add(doc.layers.events.on('structure', () => refresh()));
  name.value = nextStateName(states());
  refresh();
}
