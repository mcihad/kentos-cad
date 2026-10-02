import type { AppContext } from '../../app/context';
import type { LayerNode } from '../../model/layers';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { field, summaryLine } from '../io/common';
import { layerSwatch } from '../layers/swatch';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { BATCH_STEP, planBatch, planDedupe, runBatch, type BatchOp } from './pointBatch';
import { PREFIX } from './pointEdit';

/** The window shows the editor's words without its prefix. */
const plain = (text: string) => (text.startsWith(PREFIX) ? text.slice(PREFIX.length) : text);
const quoted = (name: string | undefined) => (name?.trim() ? `“${name.trim()}”` : '(adsız)');
/** How many of the targets' names the window lists under their count. */
const LISTED = 6;

/** What the window takes from the table. */
export interface BatchWindowOptions {
  /** Bağlı çizgiler izler, as the table has it (Çift noktaları ayıkla's kept points). */
  follow: boolean;
  /** Çiftleri göster: the table shows the groups (by their members' ids). */
  showGroups: (groups: number[][]) => void;
  /** Written: the table forgets the groups it showed. */
  done: () => void;
}

/** Çift noktaları ayıkla's choices as its window lists them. */
const BY = [
  { value: 'place', label: 'Aynı yer' },
  { value: 'name', label: 'Aynı ad' },
] as const;
const KEEP = [
  { value: 'first', label: 'İlki' },
  { value: 'last', label: 'Sonuncusu' },
  { value: 'average', label: 'Ortalaması' },
] as const;

/**
 * Nokta editörü's batch windows (docs/adr/0153 §5), loaded on first use: the operation's values over its target rows
 * (named at the top), what it would change (how many points, the first change), Uygula writing it as one undo step
 * named after it (./pointBatch.ts). A refusal is said and shown, and the window stays for another value. Çift
 * noktaları ayıkla's window counts the groups as its values change; Çiftleri göster shows them in the table. The
 * desktop's are `apps/desktop/src/points/batch_view.rs`.
 */
export function openPointBatchDialog(ctx: AppContext, kind: BatchOp['kind'], ids: readonly number[], header: string, opts: BatchWindowOptions): void {
  const { doc } = ctx;
  let mode: 'add' | 'remove' = 'add';
  let layer = doc.layers.active.value;
  let by: 'name' | 'place' = 'place';
  let keep: 'first' | 'last' | 'average' = 'first';
  let refused: string | null = null;
  const points = ids.flatMap((id) => {
    const e = doc.get(id);
    return e?.kind === 'point' ? [e] : [];
  });
  // Sıralı numara ver starts from the first row's name when it ends with a number.
  const firstName = points[0]?.label?.trim() ?? '';
  const text = h('input', {
    class: `field${kind === 'dedupe' ? ' num' : ''}`,
    spellcheck: 'false',
    value: kind === 'number' ? (/[0-9]$/.test(firstName) ? firstName : '1') : kind === 'dedupe' ? '0.001' : '',
    placeholder: kind === 'rename' ? 'ör. P.' : kind === 'number' ? 'ör. 1, P100, 101/1' : 'metre',
    'aria-label': kind === 'rename' ? 'Önek' : kind === 'number' ? 'Başlangıç adı' : 'Tolerans',
  });
  // The layer list as the toolbar's: groups as headers, each layer's count; a locked layer takes nothing.
  const layerPick = new Dropdown({
    ariaLabel: 'Katman',
    className: 'dropdown--layer',
    width: 260,
    items: () => {
      const { layers } = doc;
      const counts = doc.countByLayer();
      const out: MenuItem[] = [];
      const walk = (nodes: readonly LayerNode[]) => {
        for (const n of nodes) {
          if (n.type === 'group') {
            out.push({ kind: 'header', label: layers.path(n.id) });
            walk(n.children);
          } else
            out.push({
              label: n.name,
              swatch: layerSwatch(n, ctx.view.palette),
              radio: true,
              checked: n.id === layer,
              hint: String(counts.get(n.id) ?? 0),
              disabled: layers.isLocked(n.id),
              run: () => ((layer = n.id), (refused = null), refresh()),
            });
        }
      };
      walk(layers.tree);
      return out;
    },
  });
  const modeBox = h('div');
  const byBox = h('div');
  const keepBox = h('div');
  const summary = h('div', { class: 'io-summary' });
  const apply = h('button', { class: 'btn btn--primary', type: 'button' }, kind === 'dedupe' ? 'Ayıkla' : 'Uygula');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const show = h('button', { class: 'btn', type: 'button' }, 'Çiftleri göster');
  const listed = points.slice(0, LISTED).map((e) => e.label?.trim() || '(adsız)');
  const targets = h(
    'div',
    { class: 'io-file' },
    icon('pointEditor', 18),
    h('div', { class: 'io-file__text' }, h('span', { class: 'io-file__name' }, header), h('span', { class: 'io-file__meta' }, listed.join(', ') + (points.length > LISTED ? ' …' : ''))),
  );

  const op = (): BatchOp =>
    kind === 'rename'
      ? { kind, mode, prefix: text.value }
      : kind === 'number'
        ? { kind, start: text.value }
        : kind === 'layer'
          ? { kind, layer }
          : { kind, by, tolerance: text.value, keep };
  const groups = (): number[][] => {
    const o = op();
    return o.kind === 'dedupe' ? planDedupe(doc, ids, o).groups : [];
  };

  function refresh(): void {
    if (kind === 'rename')
      replaceChildren(
        modeBox,
        segmented({
          label: 'İşlem',
          options: [
            { value: 'add', label: 'Önek ekle' },
            { value: 'remove', label: 'Önek kaldır' },
          ],
          value: mode,
          onChange: (v) => ((mode = v), (refused = null), refresh()),
        }),
      );
    if (kind === 'layer') {
      const l = doc.layers.get(layer);
      if (l) layerPick.set(h('span', { class: 'swatch', style: `--swatch:${layerSwatch(l, ctx.view.palette)}` }), h('span', { class: 'dropdown__text' }, l.name));
    }
    const o = op();
    if (o.kind === 'dedupe') {
      replaceChildren(byBox, segmented({ label: 'Ölçüt', options: [...BY], value: by, onChange: (v) => ((by = v), (refused = null), refresh()) }));
      replaceChildren(keepBox, segmented({ label: 'Tutulan', options: [...KEEP], value: keep, onChange: (v) => ((keep = v), (refused = null), refresh()) }));
      text.disabled = by === 'name';
      const plan = planDedupe(doc, ids, o);
      const lines = [plan.error ? summaryLine('warn', plain(plan.error)) : summaryLine('info', plan.summary ?? '')];
      if (refused) lines.push(summaryLine('warn', plain(refused)));
      replaceChildren(summary, ...lines);
      apply.disabled = show.disabled = plan.error !== null || plan.groups.length === 0;
      return;
    }
    const plan = planBatch(doc, ids, o);
    const lines: HTMLElement[] = [];
    if (plan.error) lines.push(summaryLine(text.value.trim() ? 'warn' : 'info', plain(plan.error)));
    else if (!plan.changes.length) lines.push(summaryLine('info', kind === 'layer' ? 'Taşınacak nokta yok.' : 'Adı değişen nokta yok.'));
    else if (kind === 'layer') lines.push(summaryLine('info', `${plan.changes.length} nokta “${doc.layers.get(layer)?.name ?? layer}” katmanına taşınacak.`));
    else {
      const c = plan.changes[0];
      lines.push(summaryLine('info', `${plan.changes.length} noktanın adı değişecek; ilki ${quoted(c.e.label)} → “${c.name}”.`));
    }
    if (refused) lines.push(summaryLine('warn', plain(refused)));
    replaceChildren(summary, ...lines);
    apply.disabled = plan.error !== null || plan.changes.length === 0;
  }

  const run = () => {
    if (apply.disabled) return;
    const out = runBatch(doc, ids, op(), opts.follow);
    if (out.step) {
      const [done, ...rest] = out.said;
      if (done) ctx.log.success(done);
      for (const line of rest) ctx.log.warn(line);
      dialog.close();
      opts.done();
      return;
    }
    for (const line of out.said) ctx.log.warn(line);
    refused = out.said[0] ?? null;
    refresh();
  };

  const values =
    kind === 'rename'
      ? [h('div', { class: 'io-row' }, field('İşlem', modeBox), field('Önek', text, undefined, 'grow'))]
      : kind === 'number'
        ? [h('div', { class: 'io-row' }, field('Başlangıç adı', text, 'İlk satır bu adı, her sonraki bir fazlasını alır (Artır).', 'grow'))]
        : kind === 'layer'
          ? [h('div', { class: 'io-row' }, field('Katman', layerPick.el, undefined, 'grow'))]
          : [
              h('div', { class: 'io-row' }, field('Ölçüt', byBox), field('Tolerans (m)', text, 'Aynı yerde: ilk noktasına bu kadar yakın olan gruba katılır.', 'grow')),
              h(
                'div',
                { class: 'io-row' },
                field(
                  'Tutulan',
                  keepBox,
                  opts.follow
                    ? 'Ortalamada tutulan grubun ilk noktasıdır; Bağlı çizgiler izler açık: çizgileri de taşınır.'
                    : 'Ortalamada tutulan grubun ilk noktasıdır; Bağlı çizgiler izler kapalı: çizgiler yerinde kalır.',
                  'grow',
                ),
              ),
            ];
  const dialog = new Dialog({
    title: BATCH_STEP[kind],
    width: kind === 'dedupe' ? 540 : 480,
    className: 'dialog--io dialog--point-batch',
    content: [targets, ...values, summary],
    footer: [h('div', { class: 'dialog__spacer' }), ...(kind === 'dedupe' ? [show] : []), cancel, apply],
  });
  show.addEventListener('click', () => {
    if (show.disabled) return;
    opts.showGroups(groups());
    dialog.close();
  });
  text.addEventListener('input', () => ((refused = null), refresh()));
  text.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      run();
    }
  });
  apply.addEventListener('click', run);
  cancel.addEventListener('click', () => dialog.close());
  refresh();
  if (kind === 'layer' || kind === 'dedupe') (kind === 'layer' ? layerPick.el : (byBox.querySelector('[aria-checked="true"]') as HTMLElement | null))?.focus();
  else {
    text.focus();
    text.select();
  }
}
