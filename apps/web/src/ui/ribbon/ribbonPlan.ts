import { QUICK_ACCESS, type SplitEntry } from '../../app/ribbon';

/**
 * The ribbon's rules apart from the DOM (Ribbon.ts, controls.ts;
 * docs/specs/ribbon.md): the quick access bar's menu and how a command is
 * added or taken off, what a right click offers on each kind of item, and
 * a split button's list. The key tips' letters and keys are keytips.ts's;
 * the kept bar and split choices are app/ribbon.ts's (quickAccessOf,
 * splitCurrent). fixtures/shell/v1/ribbon.json holds them for the desktop
 * (format in fixtures/shell/README.md).
 */

export const RIBBON_TEXTS = {
  quickAccess: 'Hızlı erişim',
  customize: 'Hızlı erişimi özelleştir',
  customizeTip: 'Şeritteki bir düğmeye sağ tıklayarak da ekleyebilirsiniz.',
  fixedHint: 'sabit',
  add: 'Hızlı erişime ekle',
  remove: 'Hızlı erişimden kaldır',
  fixed: 'Hızlı erişimde (sabit)',
  fold: 'Şeridi daralt',
  pin: 'Şeridi sabitle',
  foldTip: 'Daraltılmış şerit bir sekmeye tıklayınca çizimin üstünde açılır.',
  others: (title: string) => `${title}: diğer seçenekler`,
  method: (title: string, label: string) => `${title}: ${label}`,
  panelMore: (panel: string) => `${panel}: diğer araçlar`,
  cannotStart: (title: string, label: string) => `“${title}: ${label}” şu an başlatılamadı.`,
} as const;

/** Offered in the quick access bar's menu besides what the user added from the ribbon, in this order. */
export const QUICK_ACCESS_OFFERS: readonly string[] = ['file.new', 'file.open', 'file.saveAs', 'edit.paste', 'view.zoomExtents', 'tool.zoomWindow', 'tool.pan', 'tools.options', 'view.theme.toggle'];

/** A row of a ribbon menu. */
export type RibbonRow =
  | { kind: 'header'; label: string }
  | { kind: 'separator' }
  /** A command's own row, as the menus draw it (its title, shortcut and check; off while the command is). */
  | { kind: 'command'; command: string }
  /**
   * A quick access row about a command: `label` its own words, or the
   * command's title when absent; `set`, what choosing it makes of the
   * command's place on the bar (true adds, false takes off; none: nothing).
   */
  | { kind: 'quick'; command: string; label?: string; icon?: string; checked?: boolean; disabled?: boolean; hint?: string; set?: boolean };

/** The kept list after a command is put on the bar (at its end) or taken off. */
export function withQuickAccess(kept: readonly string[], id: string, on: boolean): string[] {
  const rest = kept.filter((x) => x !== id);
  return on ? [...rest, id] : rest;
}

/**
 * The bar's ▾ menu: its title, then every command on the bar and every
 * offer (once each, the bar's first, then the offers' order; only commands
 * this app has), checked while on the bar; the fixed three are off and say
 * so. Then the ribbon's fold.
 */
export function quickAccessMenu(bar: readonly string[], exists: (id: string) => boolean): RibbonRow[] {
  const offers = [...new Set([...bar, ...QUICK_ACCESS_OFFERS])].filter(exists);
  return [
    { kind: 'header', label: RIBBON_TEXTS.quickAccess },
    ...offers.map((id): RibbonRow => {
      const fixed = QUICK_ACCESS.includes(id);
      const on = bar.includes(id);
      return { kind: 'quick', command: id, checked: on, ...(fixed ? { disabled: true, hint: RIBBON_TEXTS.fixedHint } : { set: !on }) };
    }),
    { kind: 'separator' },
    { kind: 'command', command: 'view.ribbonCollapse' },
  ];
}

/** A right click on a command's button (on a panel, on the bar, the top of a split button or its arrow): the bar, then the fold. */
export function commandMenu(id: string, bar: readonly string[]): RibbonRow[] {
  const first: RibbonRow = QUICK_ACCESS.includes(id)
    ? { kind: 'quick', command: id, label: RIBBON_TEXTS.fixed, icon: 'pin', disabled: true }
    : bar.includes(id)
      ? { kind: 'quick', command: id, label: RIBBON_TEXTS.remove, icon: 'close', set: false }
      : { kind: 'quick', command: id, label: RIBBON_TEXTS.add, icon: 'pin', set: true };
  return [first, { kind: 'separator' }, { kind: 'command', command: 'view.ribbonCollapse' }];
}

/** A right click anywhere else on the ribbon (a tab, a menu button, a panel's title, the bar's ▾, empty space): the fold. A text field keeps its own menu. */
export function ribbonMenu(): RibbonRow[] {
  return [{ kind: 'command', command: 'view.ribbonCollapse' }];
}

/**
 * A split button's list: a tool's methods under the tool's name, or a
 * family's tools without a title; each row the entry's words, its
 * description as the hint and a method's own icon (Ölçülendirme's kinds;
 * else its command's). Choosing one keeps it on top (`ribbonSplits`) and
 * runs it.
 */
export function splitMenu(entries: readonly SplitEntry[]): { header: string | null; rows: { command: string; option?: string; label: string; hint?: string; icon?: string }[] } {
  const methods = entries.every((e) => e.command === entries[0].command);
  return {
    header: methods ? entries[0].title : null,
    rows: entries.map((e) => ({ command: e.command, ...(e.option ? { option: e.option } : {}), label: e.label, ...(e.description ? { hint: e.description } : {}), ...(e.icon ? { icon: e.icon } : {}) })),
  };
}

/**
 * The top of a split button: its name (a method does not rename it), what it says it does (`Daire: 3 nokta`) and the
 * chosen method's own icon, when it has one (else its command's).
 */
export function splitFace(e: SplitEntry): { label: string; aria: string; icon?: string } {
  return { label: e.title.replace(/…$/, ''), aria: e.option || e.label !== e.title ? RIBBON_TEXTS.method(e.title, e.label) : e.title, ...(e.icon ? { icon: e.icon } : {}) };
}
