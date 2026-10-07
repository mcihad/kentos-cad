import type { AppContext } from '../../app/context';
import { drawingJson, pickDrawing, takeInto } from '../../app/drawingExchange';
import { blockKey, exchangeFold, type Json, type Picks, type Same } from '../../model/exchange';
import { h, replaceChildren } from '../dom';
import { summaryLine } from './common';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const TAKE_TITLE = 'Başka çizimden al';

type Section = 'layers' | 'blocks' | 'textStyles' | 'dimensionStyles' | 'library' | 'layerStates' | 'settings';

/** Each section's heading in the list, in its order. */
export const TAKE_HEADINGS: Record<Section, string> = {
  layers: 'Katmanlar',
  blocks: 'Bloklar',
  textStyles: 'Yazı stilleri',
  dimensionStyles: 'Ölçü stilleri',
  library: 'Kitaplık',
  layerStates: 'Katman durumları',
  settings: 'Proje ayarları',
};

const SECTIONS = Object.keys(TAKE_HEADINGS) as Section[];
const KIND_WORDS: Record<string, string> = { symbol: 'Sembol', asset: 'Görüntü', template: 'Şablon' };

interface Row {
  section: Section;
  /** What the picks name it by: a path, a name, an id. */
  key: string;
  words: string;
  /** This drawing has one of the same name (or id). */
  same: boolean;
}

/** What another drawing offers, row by row, each marked when this drawing has the same. */
export function takeRows(ours: Json, theirs: Json): Row[] {
  const rows: Row[] = [];
  const walk = (nodes: readonly Json[], above: string[], out: [Json, string[]][]) => {
    for (const n of nodes) {
      const here = [...above, n.name as string];
      out.push([n, here]);
      walk(n.children ?? [], here, out);
    }
    return out;
  };
  const ourPaths = new Set(walk(ours.layers ?? [], [], []).map(([, p]) => p.map(exchangeFold).join('\u0000')));
  for (const [n, p] of walk(theirs.layers ?? [], [], []))
    if (n.type === 'layer') rows.push({ section: 'layers', key: p.join(' / '), words: p.join(' / '), same: ourPaths.has(p.map(exchangeFold).join('\u0000')) });
  const ourBlocks = new Set((ours.blocks ?? []).map((b: Json) => blockKey(b.name)));
  for (const b of theirs.blocks ?? []) rows.push({ section: 'blocks', key: b.name, words: b.name, same: ourBlocks.has(blockKey(b.name)) });
  for (const section of ['textStyles', 'dimensionStyles', 'layerStates'] as const) {
    const mine = new Set((ours.settings[section] ?? []).map((s: Json) => exchangeFold(s.name)));
    for (const s of theirs.settings[section] ?? []) rows.push({ section, key: s.name, words: s.name, same: mine.has(exchangeFold(s.name)) });
  }
  const ourItems = new Set((ours.styles?.items ?? []).map((it: Json) => it.id));
  for (const it of theirs.styles?.items ?? [])
    rows.push({ section: 'library', key: it.id, words: `${KIND_WORDS[it.kind] ?? it.kind}: ${it.name ?? it.id}`, same: ourItems.has(it.id) });
  rows.push({ section: 'settings', key: 'settings', words: 'Birimler ve ondalıklar, açı birimi, çizim ölçeği, çizim yazı tipi, ölçme ayarları', same: false });
  return rows;
}

/** The checked rows as Başka çizimden al's picks. */
export function picksOf(rows: readonly Row[], checked: ReadonlySet<string>): Picks {
  const of = (s: Section) => rows.filter((r) => r.section === s && checked.has(`${s}:${r.key}`)).map((r) => r.key);
  return {
    layers: of('layers'),
    blocks: of('blocks'),
    textStyles: of('textStyles'),
    dimensionStyles: of('dimensionStyles'),
    library: of('library'),
    layerStates: of('layerStates'),
    settings: checked.has('settings:settings'),
  };
}

/**
 * Başka çizimden al (docs/adr/0193 §2; the desktop's `exchange.rs`): a drawing file is chosen, then what it offers is
 * listed by section, each row checked (the project settings not), a row this drawing has the same of marked. Aynı adlı
 * olanlar: Atla or Değiştir. Al takes the checked ones and closes.
 */
export async function openTakeFrom(ctx: AppContext): Promise<void> {
  const picked = await pickDrawing(ctx, TAKE_TITLE);
  if (!picked) return;
  const rows = takeRows(drawingJson(ctx), picked.json);
  const checked = new Set(rows.filter((r) => r.section !== 'settings').map((r) => `${r.section}:${r.key}`));
  let same: Same = 'skip';
  const summary = h('div', { class: 'io-summary' });
  const list = h('div', { class: 'io-table-wrap purge-rows' });
  const take = h('button', { class: 'btn btn--primary', type: 'button' }, 'Al') as HTMLButtonElement;
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç') as HTMLButtonElement;

  function rowOf(r: Row): HTMLElement {
    const key = `${r.section}:${r.key}`;
    const box = h('input', { type: 'checkbox', checked: checked.has(key), 'aria-label': r.words }) as HTMLInputElement;
    box.addEventListener('change', () => {
      if (box.checked) checked.add(key);
      else checked.delete(key);
      refresh();
    });
    return h('label', { class: 'purge-row' }, box, h('span', { class: 'purge-row__words' }, r.words), r.same ? h('span', { class: 'purge-row__note' }, 'bu çizimde var') : null);
  }

  function refresh(): void {
    const n = checked.size;
    replaceChildren(
      summary,
      summaryLine('info', `“${picked!.name}” çiziminden işaretlenenler alınır. Katmanlar ve bloklar tek adımda geri alınır; stiller, kitaplık, katman durumları ve proje ayarları ayardır. Koordinat sistemi alınmaz.`),
      summaryLine(n ? 'ok' : 'info', n ? `${n} öğe işaretli.` : 'Alınacakları işaretleyin.'),
    );
    take.disabled = !n;
  }

  replaceChildren(
    list,
    ...SECTIONS.flatMap((s) => {
      const of = rows.filter((r) => r.section === s);
      return of.length ? [h('div', { class: 'purge-heading' }, `${TAKE_HEADINGS[s]} (${of.length})`), ...of.map(rowOf)] : [];
    }),
  );
  // Built again when chosen, as the segmented control shows the value it was made with.
  const sameField = h('div', { class: 'take-same' });
  const buildSame = (): void =>
    replaceChildren(
      sameField,
      h('span', {}, 'Aynı adlı olanlar'),
      segmented<Same>({
        label: 'Aynı adlı olanlar',
        options: [
          { value: 'skip', label: 'Atla' },
          { value: 'replace', label: 'Değiştir' },
        ],
        value: same,
        onChange: (v) => ((same = v), buildSame()),
      }),
    );
  buildSame();
  const dialog = new Dialog({
    title: TAKE_TITLE,
    width: 600,
    className: 'dialog--io dialog--purge dialog--take',
    content: [summary, list, sameField],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, take],
  });
  take.addEventListener('click', () => {
    if (takeInto(ctx, picked.json, picksOf(rows, checked), same, picked.name)) dialog.close();
  });
  cancel.addEventListener('click', () => dialog.close());
  refresh();
}
