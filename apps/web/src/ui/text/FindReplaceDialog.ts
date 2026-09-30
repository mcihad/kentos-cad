import type { AppContext } from '../../app/context';
import { h, replaceChildren } from '../dom';
import { field, summaryLine } from '../io/common';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { VirtualRows } from '../widgets/VirtualRows';
import { findMatches, replaceMatches, type FindQuery, type Match } from './findReplace';

/** The window's title, which a trace names it by. */
export const FIND_TITLE = 'Bul ve değiştir';

/**
 * Bul ve değiştir (docs/adr/0145 §6): Bul and Değiştir, Joker (*), Büyük küçük harf eşleşsin, Tam sözcük and Yalnız
 * seçimde; the matches as rows (layer, the text, what it becomes), each with its box, found again as one types; a row's
 * words zoom to its text and select it. Seçilenleri değiştir writes the checked rows, Hepsini değiştir every row, in
 * one step “Bul ve değiştir”; a text on a locked layer, or one that would become empty, is listed but not written,
 * and that is said. The window stays open for the next search. The desktop's (apps/desktop/src/find_replace.rs) is
 * the same.
 */
export function openFindReplaceDialog(ctx: AppContext): void {
  const q: FindQuery = { find: '', replace: '', wildcard: false, caseless: true, wholeWord: false, selectionOnly: ctx.selection.ids.value.size > 0 };
  let matches: Match[] = [];
  /** The rows left unchecked, by object id: every other row is checked. */
  const unchecked = new Set<number>();

  const find = h('input', { class: 'field', 'aria-label': 'Bul', spellcheck: 'false', placeholder: 'Aranacak yazı; * herhangi bir dizi (Joker açıkken)' });
  const replace = h('input', { class: 'field', 'aria-label': 'Değiştir', spellcheck: 'false', placeholder: 'Yeni yazı; boş bırakılırsa silinir' });
  const box = (words: string, on: boolean, set: (v: boolean) => void) => {
    const input = h('input', { type: 'checkbox', checked: on });
    input.addEventListener('change', () => (set(input.checked), refresh()));
    return { input, label: h('label', { class: 'io-check' }, input, words) };
  };
  const wildcard = box('Joker (*)', q.wildcard, (v) => (q.wildcard = v));
  const matchCase = box('Büyük küçük harf eşleşsin', !q.caseless, (v) => (q.caseless = !v));
  const whole = box('Tam sözcük', q.wholeWord, (v) => (q.wholeWord = v));
  const only = box('Yalnız seçimde', q.selectionOnly, (v) => (q.selectionOnly = v));
  const summary = h('div', { class: 'io-summary find-summary' });
  const tbody = h('tbody');
  const scroller = h('div', { class: 'io-table-wrap find-rows' }, h('table', { class: 'io-table' }, h('thead', null, h('tr', null, h('th', { class: 'find-check' }), h('th', null, 'Katman'), h('th', null, 'Metin'), h('th', null, 'Yeni metin'))), tbody));
  const rows = new VirtualRows({
    parent: tbody,
    scroller,
    spacer: () => h('tr', null, h('td', { colspan: '4' })),
    row: (i) => rowOf(matches[i]),
  });
  const chosen = h('button', { class: 'btn', type: 'button' }, 'Seçilenleri değiştir');
  const all = h('button', { class: 'btn btn--primary', type: 'button' }, 'Hepsini değiştir');
  const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');

  function rowOf(m: Match): HTMLElement {
    const check = h('input', { type: 'checkbox', checked: !m.blocked && !unchecked.has(m.id), disabled: !!m.blocked, 'aria-label': `“${m.old}” değişsin` });
    check.addEventListener('change', () => {
      if (check.checked) unchecked.delete(m.id);
      else unchecked.add(m.id);
      buttons();
    });
    const why = m.blocked === 'locked' ? 'Kilitli katmanda: değiştirilmez.' : m.blocked === 'empty' ? 'Boş yazı olmaz: değiştirilmez.' : null;
    const tr = h(
      'tr',
      { class: `find-row${m.blocked ? ' find-row--blocked' : ''}`, title: why ?? 'Yazıya gitmek için tıklayın' },
      h('td', { class: 'find-check' }, check),
      h('td', null, m.blocked === 'locked' ? icon('lock', 12) : null, m.layer),
      h('td', null, m.old),
      h('td', { class: 'find-new' }, m.blocked === 'empty' ? '(boş)' : m.new),
    );
    // The row's words: to the text on the drawing, selected.
    tr.addEventListener('click', (e) => {
      if ((e.target as HTMLElement).closest('input')) return;
      ctx.selection.set([m.id]);
      ctx.view.zoomToSelection();
    });
    return tr;
  }

  function buttons(): void {
    const writable = matches.filter((m) => !m.blocked);
    all.disabled = writable.length === 0;
    chosen.disabled = !writable.some((m) => !unchecked.has(m.id));
  }

  function refresh(): void {
    q.find = find.value;
    q.replace = replace.value;
    whole.input.disabled = q.wildcard;
    only.input.disabled = ctx.selection.ids.value.size === 0 && !q.selectionOnly;
    matches = findMatches(ctx, q);
    const locked = matches.filter((m) => m.blocked === 'locked').length;
    const empty = matches.filter((m) => m.blocked === 'empty').length;
    const lines = [
      q.find
        ? summaryLine(matches.length ? 'info' : 'warn', matches.length ? `${matches.length} eşleşme.` : `Eşleşen yazı yok${q.selectionOnly ? ' (yalnız seçimde arandı)' : ''}.`)
        : summaryLine('info', 'Aranacak yazıyı yazın. Joker açıkken * herhangi bir dizidir ve kalıp bütün yazıya uyar; değiştirmedeki * sırasıyla onların yerine geçer.'),
    ];
    if (locked) lines.push(summaryLine('warn', `${locked} yazı kilitli katmanda: listelenir, değiştirilmez.`));
    if (empty) lines.push(summaryLine('warn', `${empty} yazı boş kalacağı için değiştirilmez.`));
    replaceChildren(summary, ...lines);
    rows.set(matches.length);
    buttons();
  }

  const write = (only: boolean) => {
    const picked = matches.filter((m) => !m.blocked && (!only || !unchecked.has(m.id)));
    const blocked = matches.filter((m) => m.blocked).length;
    const done = replaceMatches(ctx, picked);
    if (done) ctx.log.success(`${done} yazı değiştirildi.`);
    if (blocked && !only) ctx.log.warn(`${blocked} yazı değiştirilmedi: kilitli katmanda ya da boş kalacaktı.`);
    unchecked.clear();
    refresh();
  };

  const dialog = new Dialog({
    title: FIND_TITLE,
    width: 680,
    className: 'dialog--io dialog--find',
    content: [
      h('div', { class: 'io-row' }, field('Bul', find, undefined, 'grow'), field('Değiştir', replace, undefined, 'grow')),
      h('div', { class: 'io-row find-options' }, wildcard.label, matchCase.label, whole.label, only.label),
      summary,
      scroller,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), close, chosen, all],
    onClose: () => rows.dispose(),
  });
  // One call to the core for every text: found again at every keystroke.
  find.addEventListener('input', refresh);
  replace.addEventListener('input', refresh);
  chosen.addEventListener('click', () => write(true));
  all.addEventListener('click', () => write(false));
  close.addEventListener('click', () => dialog.close());
  refresh();
  find.focus();
}
