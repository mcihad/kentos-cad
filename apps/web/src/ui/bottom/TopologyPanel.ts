import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { tableSpacer, VirtualRows } from '../widgets/VirtualRows';
import { clickPick } from './PointTable';
import { countText, fixLabel, measureText, missingLayers, noFindings, notChecked, objectsText, ruleText, shownFindings, TOPOLOGY_COLUMNS, TOPOLOGY_TEXTS as T, type TopologyFilter } from './topologyPlan';
import { applyTopologyFix, isStale, projectRules, runTopologyCheck, showFinding, toggleExceptions, topologyNames, topologyState } from './topologyRun';

/**
 * Topoloji, the bottom panel's tab of the topology rules' findings (docs/adr/0202 §5): Denetle and Kurallar…, a rule
 * and Açık / İstisna / Hepsi to choose the rows, Düzelt ▾ and İstisna yap; the rows (Sıra, Katman, Kural, Sorun,
 * Nesneler, Ölçü). A click on a row selects its objects, zooms to it and shows the finding over the drawing; Ctrl and
 * Shift add rows without moving. The rules, the check and the fixes are topologyRun.ts; the desktop's tab is
 * apps/desktop/src/topology/.
 */

/** The icon of each fix in Düzelt ▾ (the operations they are). */
const FIX_ICONS: Record<string, string> = {
  subtractFirst: 'areaSubtract',
  subtractSecond: 'geoDifference',
  mergeNeighbour: 'areaUnion',
  deletePart: 'erase',
  deleteDuplicate: 'erase',
  snapEnd: 'extend',
  removeVertex: 'erase',
  deleteObject: 'erase',
  repair: 'geoRepair',
  addVertex: 'vertex',
  clipOutside: 'geoClip',
  snapToEnd: 'snapEndpoint',
};

export class TopologyPanel extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly rulePick: Dropdown;
  private readonly filterButtons = new Map<TopologyFilter, HTMLButtonElement>();
  private readonly checkBtn: HTMLButtonElement;
  private readonly fixBtn: HTMLButtonElement;
  private readonly markBtn: HTMLButtonElement;
  private readonly markText = h('span');
  private readonly count = h('span', { class: 'ptable__count num' });
  private readonly banner = h('div', { class: 'topo__banner', role: 'status' });
  private readonly scroller: HTMLElement;
  private readonly body = h('tbody');
  private readonly empty = h('div', { class: 'empty empty--inline ptable__empty' });
  private readonly rows: VirtualRows;
  /** The findings shown, by their places in the check. */
  private shown: number[] = [];
  /** The last click without Shift, in the order shown. */
  private anchor: number | null = null;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const button = (iconName: string, text: string | HTMLElement, hint: () => { title: string; description: string }, run: () => void) => {
      const b = h('button', { class: 'btn btn--small ptable__btn', type: 'button' }, icon(iconName, 14), typeof text === 'string' ? h('span', null, text) : text);
      this.d.add(listen(b, 'click', run));
      this.d.add(tooltip(b, hint, 'top'));
      return b;
    };
    this.checkBtn = button('topologyCheck', T.check, () => ({ title: T.check, description: T.checkHint }), () => {
      runTopologyCheck(ctx);
    });
    const rulesBtn = button('topologyRules', T.rules, () => ({ title: T.rules, description: T.rulesHint }), () => ctx.commands.execute('topology.rules'));
    this.rulePick = new Dropdown({ ariaLabel: T.rulePick, width: 230, className: 'topo__rule', items: () => this.ruleItems() });
    const filters = h(
      'div',
      { class: 'seg topo__filter', role: 'radiogroup', 'aria-label': T.showPick },
      (['open', 'exception', 'all'] as const).map((f) => {
        const b = h('button', { class: 'seg__opt', type: 'button', role: 'radio', 'aria-checked': 'false' }, f === 'open' ? T.open : f === 'exception' ? T.exceptions : T.all);
        this.d.add(
          listen(b, 'click', () => {
            topologyState.filter.set(f);
            topologyState.selected.set([]);
          }),
        );
        this.filterButtons.set(f, b);
        return b;
      }),
    );
    this.fixBtn = button('topologyFix', T.fix, () => ({ title: T.fix, description: T.fixHint }), () => this.openFixes());
    this.fixBtn.append(icon('chevronDown', 12));
    this.markBtn = button('topologyException', this.markText, () => (this.marking() ? { title: T.mark, description: T.markHint } : { title: T.unmark, description: T.unmarkHint }), () => {
      toggleExceptions(ctx, topologyState.selected.value, this.marking());
    });

    // Sıra and Ölçü are numbers: their headers sit on the right, as their cells.
    const head = h('tr', null, TOPOLOGY_COLUMNS.map((c, i) => h('th', { scope: 'col', class: `topo__th topo__th--${i}${i === 0 || i === 5 ? ' num' : ''}` }, h('span', { class: 'topo__head' }, c))));
    this.scroller = h('div', { class: 'ptable__scroll' }, h('table', { class: 'topo__table' }, h('thead', null, head), this.body), this.empty);
    this.el = h(
      'div',
      { class: 'ptable topo' },
      h(
        'div',
        { class: 'ptable__bar' },
        h('div', { class: 'ptable__group' }, this.checkBtn, rulesBtn, this.rulePick.el, filters),
        h('div', { class: 'ptable__group ptable__group--end' }, this.count, this.fixBtn, this.markBtn),
      ),
      this.banner,
      this.scroller,
    );
    this.rows = new VirtualRows({ parent: this.body, scroller: this.scroller, row: (i) => this.row(i), spacer: tableSpacer(TOPOLOGY_COLUMNS.length) });
    this.d.add(() => this.rows.dispose());

    this.d.add(
      listen<MouseEvent>(this.body, 'click', (e) => {
        const tr = (e.target as HTMLElement).closest<HTMLElement>('tr[data-at]');
        if (!tr) return;
        this.press(Number(tr.dataset.at), { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
      }),
    );
    this.d.add(watchAll([topologyState.checked, topologyState.selected, topologyState.filter, topologyState.rule, ctx.doc.settings.topology, ctx.format.changed], () => this.refresh()));
    this.d.add(ctx.doc.events.on('changed', () => this.refresh()));
    this.d.add(ctx.doc.layers.version.subscribe(() => this.refresh()));
    queueMicrotask(() => this.refresh());
  }

  /** A row pressed (its place in the order shown): alone, it is chosen and shown; Ctrl turns one over, Shift takes the run. */
  press(at: number, how: { ctrl: boolean; shift: boolean } = { ctrl: false, shift: false }): void {
    const c = topologyState.checked.value;
    if (!c || at < 0 || at >= this.shown.length) return;
    if (how.ctrl || how.shift) {
      const chosen = new Set(topologyState.selected.value);
      const picked = clickPick(chosen, this.shown, at, this.anchor, how);
      if (!how.shift) this.anchor = at;
      topologyState.selected.set([...picked]);
      return;
    }
    this.anchor = at;
    topologyState.selected.set([this.shown[at]]);
    showFinding(this.ctx, c, this.shown[at], true);
  }

  /** Whether İstisna yap marks (some chosen row is open) or unmarks. */
  private marking(): boolean {
    const c = topologyState.checked.value;
    const chosen = topologyState.selected.value;
    return !c || !chosen.length || chosen.some((i) => !c.findings[i]?.exception);
  }

  /** Bütün kurallar, then each rule with its count of findings. */
  private ruleItems(): MenuItem[] {
    const c = topologyState.checked.value;
    const rules = c?.rules ?? projectRules(this.ctx).rules;
    const kinds = topologyNames().kinds;
    const name = (id: string) => this.ctx.doc.layers.get(id)?.name ?? id;
    const pick = (r: number | null) => () => {
      topologyState.rule.set(r);
      topologyState.selected.set([]);
    };
    return [
      { label: T.allRules, radio: true, checked: topologyState.rule.value === null, run: pick(null) },
      ...rules.map((r, i) => ({
        label: `${name(r.layer)}: ${ruleText(r, kinds, name)}`,
        hint: c ? String(c.findings.filter((f) => f.rule === i).length) : undefined,
        radio: true,
        checked: topologyState.rule.value === i,
        run: pick(i),
      })),
    ];
  }

  /** Düzelt ▾: the fixes the one chosen finding offers. */
  private openFixes(): void {
    const c = topologyState.checked.value;
    const chosen = topologyState.selected.value;
    if (!c || chosen.length !== 1) return;
    const at = chosen[0];
    const f = c.findings[at];
    const ctx = this.ctx;
    const items: MenuItem[] = f.fixes.length
      ? f.fixes.map((x) => ({ label: fixLabel(f, x, c.ids, (m) => ctx.format.length(m)), icon: FIX_ICONS[x.key] ?? 'topologyFix', run: () => applyTopologyFix(ctx, at, x.key) }))
      : [{ label: T.fixNone, disabled: true }];
    PopupMenu.open(items, this.fixBtn.getBoundingClientRect(), { owner: this.fixBtn });
  }

  private refresh(): void {
    if (!this.el.isConnected) return;
    const { ctx } = this;
    const c = topologyState.checked.value;
    const { rules } = projectRules(ctx);
    const rule = topologyState.rule.value;
    const filter = topologyState.filter.value;
    this.shown = c ? shownFindings(c.findings, filter, rule !== null && rule < c.rules.length ? rule : null) : [];
    const chosen = topologyState.selected.value;
    for (const [f, b] of this.filterButtons) b.setAttribute('aria-checked', String(f === filter));
    const kinds = topologyNames().kinds;
    const name = (id: string) => ctx.doc.layers.get(id)?.name ?? id;
    const r = rule !== null ? c?.rules[rule] : undefined;
    this.rulePick.set(r ? `${name(r.layer)}: ${ruleText(r, kinds, name)}` : T.allRules);
    this.checkBtn.disabled = !rules.length;
    this.fixBtn.disabled = !c || chosen.length !== 1 || isStale(ctx, c);
    this.markBtn.disabled = !c || !chosen.length || isStale(ctx, c);
    this.markText.textContent = this.marking() ? T.mark : T.unmark;
    const open = c ? c.findings.filter((f) => !f.exception).length : 0;
    this.count.textContent = c && c.findings.length ? countText(this.shown.length, open, c.findings.length - open) : '';
    const notes = [c && isStale(ctx, c) ? T.stale : null, c?.missing ? missingLayers(c.missing) : null].filter((x): x is string => !!x);
    replaceChildren(this.banner, notes.map((n) => h('div', { class: 'topo__note' }, icon('warning', 14), h('span', null, n))));
    this.banner.hidden = !notes.length;
    this.empty.hidden = this.shown.length > 0;
    if (!this.shown.length) {
      replaceChildren(
        this.empty,
        !rules.length ? T.noRules : !c ? notChecked(rules.length) : !c.findings.length ? noFindings(c.rules.length, c.ids.length) : T.noneShown,
      );
    }
    this.rows.set(this.shown.length);
  }

  private row(i: number): HTMLElement {
    const { ctx } = this;
    const c = topologyState.checked.value;
    const at = this.shown[i];
    const f = c?.findings[at];
    if (!c || !f) return h('tr');
    const kinds = topologyNames().kinds;
    const name = (id: string) => ctx.doc.layers.get(id)?.name ?? id;
    const rule = c.rules[f.rule];
    const on = topologyState.selected.value.includes(at);
    const objects = objectsText(f, c.ids);
    const measure = measureText(f, { area: (m) => ctx.format.area(m), length: (m) => ctx.format.length(m), angle: (a) => ctx.format.angle(a) });
    const text = rule ? ruleText(rule, kinds, name) : '';
    return h(
      'tr',
      { class: `${on ? 'is-selected' : ''}${f.exception ? ' topo__row--exception' : ''}` || null, 'data-at': String(i), 'aria-selected': String(on) },
      h('td', { class: 'num' }, String(i + 1)),
      h('td', { title: rule ? name(rule.layer) : '' }, rule ? name(rule.layer) : ''),
      h('td', { title: text }, text),
      h('td', { class: 'topo__problem', title: f.label }, f.exception ? icon('topologyException', 12) : null, h('span', null, f.label)),
      h('td', { title: objects }, objects),
      h('td', { class: 'num', title: measure }, measure),
    );
  }
}
