import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import { listen } from '../../core/disposable';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { commandButton } from '../widgets/CommandButton';
import { fitBar } from '../widgets/fit';
import { PopupMenu } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { SHELL_TEXTS, TOOLBAR_FOLD, TOOLBAR_GROUPS, TOOLBAR_WIDTHS as WIDTHS, VIEW_MORE } from '../shell/shellPlan';
import { colorField, layerField, lineTypeField, propertiesField, scaleField, weightField } from './fields';

const T = SHELL_TEXTS.toolbar;

/**
 * Toolbar = groups of command buttons + current-property fields
 * (shellPlan.ts TOOLBAR_GROUPS).
 *
 * It fits its width in steps (DESIGN.md §7.3; shellPlan.ts TOOLBAR_FOLD), so
 * nothing is cut off at the shell's 1100 px, on any type scale; the first
 * step that fits is kept:
 *   1. the layer, colour, type and weight fields narrow (their values
 *      shorten with an ellipsis);
 *   2. Yakınlaştır, Uzaklaştır and Kaydır also go under the Görünüm
 *      group's ⋯;
 *   3. Renk, Tip and Kalınlık fold into one “Özellikler” field whose menu
 *      holds the three lists; the layer field gets its width back;
 *   4. the layer field narrows again (the largest type scales).
 */
export class Toolbar extends Component {
  readonly el: HTMLElement;

  constructor(ctx: AppContext) {
    super();
    const btn = (id: string) => commandButton(ctx, id, this.d);
    const group = (label: string, ...children: HTMLElement[]) => h('div', { class: 'toolbar__group', role: 'group', 'aria-label': label }, ...children);

    const folded = VIEW_MORE.map(btn);
    const more = h('button', { class: 'tbtn toolbar__more', type: 'button', 'aria-label': T.more, 'aria-haspopup': 'menu', hidden: true }, icon('more', 18));
    this.d.add(listen(more, 'click', () => PopupMenu.open(VIEW_MORE.map((id) => commandItem(ctx, id)), more.getBoundingClientRect(), { owner: more, minWidth: 220 })));
    this.d.add(tooltip(more, () => ({ title: T.more, description: T.moreTip }), 'bottom'));

    const layer = layerField(ctx, this.d, { width: WIDTHS.layer[0] });
    const props = [colorField(ctx, this.d, { width: WIDTHS.color[0] }), lineTypeField(ctx, this.d, { width: WIDTHS.lineType[0] }), weightField(ctx, this.d, { width: WIDTHS.weight[0] })];
    const fold = propertiesField(ctx, this.d);
    fold.hidden = true;
    const scale = scaleField(ctx, this.d);

    // The groups in their order; the ⋯ closes the group holding the foldable Görünüm commands, the spacer comes before the scale.
    this.el = h(
      'div',
      { class: 'toolbar', role: 'toolbar', 'aria-label': T.label },
      TOOLBAR_GROUPS.flatMap((g) => {
        if (g.fields) return [group(g.label, layer, ...props, fold)];
        if (g.scale) return [h('div', { class: 'toolbar__spacer' }), group(g.label, scale)];
        const ids = g.commands ?? [];
        const buttons = ids.map((id) => folded[VIEW_MORE.indexOf(id)] ?? btn(id));
        return [group(g.label, ...buttons, ...(ids.some((id) => VIEW_MORE.includes(id)) ? [more] : []))];
      }),
    );

    const width = (el: HTMLElement, w: readonly [number, number], narrow: boolean) => (el.style.width = `calc(${narrow ? w[1] : w[0]}px * var(--ui-scale))`);
    const fit = fitBar(
      this.el,
      TOOLBAR_FOLD.length - 1,
      (level) => {
        const f = TOOLBAR_FOLD[level];
        width(layer, WIDTHS.layer, f.layerNarrow);
        props.forEach((p, i) => width(p, [WIDTHS.color, WIDTHS.lineType, WIDTHS.weight][i], f.propsNarrow));
        folded.forEach((b) => (b.hidden = f.viewMore));
        more.hidden = !f.viewMore;
        props.forEach((p) => (p.hidden = f.propsFolded));
        fold.hidden = !f.propsFolded;
        this.el.dataset.fit = String(level);
      },
      () => this.el.scrollWidth <= this.el.clientWidth,
    );
    this.d.add(fit.dispose);
    // Another typeface changes the fields' and buttons' widths without resizing the bar.
    this.d.add(ctx.prefs.uiFont.subscribe(() => fit.refit()));
    this.d.add(listen(document.fonts, 'loadingdone', () => fit.refit()));
  }
}
