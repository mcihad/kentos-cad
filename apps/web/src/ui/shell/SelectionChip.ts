import type { AppContext } from '../../app/context';
import { formatChord } from '../../core/keymap';
import { watchAll } from '../../core/signal';
import { ENTITY_KIND_LABEL } from '../../model/entities';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { ENTITY_KIND_ICON } from '../kindIcons';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';

/** The chip's distance right of and below the click, px. */
const OFFSET = 10;

/**
 * Sıradakini seç's chip (docs/adr/0187 §1): beside a click that found several objects, “1/3 ▾”. Shift+Boşluk takes the
 * next (`edit.cycleSelection`); a click opens their list, kind and layer, the chosen one marked, the one under the
 * pointer highlighted in the drawing. It follows the clicked place as the view moves and goes with the cycle (the
 * selection changing otherwise, a command starting, Esc). The desktop's is `selection_chip.rs`.
 */
export class SelectionChip extends Component {
  readonly el: HTMLButtonElement;
  private readonly ctx: AppContext;
  private readonly count: HTMLElement;

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.count = h('span', { class: 'sel-chip__count num' });
    this.el = h('button', { class: 'sel-chip', type: 'button', hidden: true, 'aria-haspopup': 'menu' }, this.count, icon('chevronDown', 12)) as HTMLButtonElement;
    host.append(this.el);
    const { selection, view, tools } = ctx;
    this.d.add(watchAll([selection.cycle, tools.activeId], () => this.sync()));
    this.d.add(view.camera.changed.subscribe(() => this.place()));
    this.el.addEventListener('pointerdown', (e) => e.stopPropagation());
    this.el.addEventListener('click', (e) => {
      e.stopPropagation();
      this.open();
    });
    this.sync();
  }

  private sync(): void {
    const c = this.ctx.selection.cycle.value;
    const shown = !!c && this.ctx.tools.activeId.value === 'select';
    this.el.hidden = !shown;
    if (!c || !shown) return;
    this.count.textContent = `${c.index + 1}/${c.candidates.length}`;
    const chord = this.ctx.keymap.chordFor('edit.cycleSelection');
    this.el.title = `Burada üst üste ${c.candidates.length} nesne var. ${chord ? `${formatChord(chord)}: sıradakini seç. ` : ''}Tıklayın: listeden seçin.`;
    this.place();
  }

  /** Right of and below the clicked place, where the view shows it now. */
  private place(): void {
    const c = this.ctx.selection.cycle.value;
    if (!c || this.el.hidden) return;
    const at = this.ctx.view.camera.worldToScreen(c.at);
    this.el.style.transform = `translate(${Math.round(at.x + OFFSET)}px, ${Math.round(at.y + OFFSET)}px)`;
  }

  private open(): void {
    const { selection, doc } = this.ctx;
    const c = selection.cycle.value;
    if (!c) return;
    const items = c.candidates.map((id, i): MenuItem => {
      const e = doc.get(id);
      const layer = e ? doc.layers.get(e.layerId)?.name : undefined;
      return {
        label: e ? `${ENTITY_KIND_LABEL[e.kind]}${layer ? ` · ${layer}` : ''}` : `Nesne ${id}`,
        icon: e ? ENTITY_KIND_ICON[e.kind] : 'more',
        radio: true,
        checked: i === c.index,
        highlight: () => selection.hover.set(id),
        run: () => selection.cycleTo(i),
      };
    });
    PopupMenu.open([{ kind: 'header', label: 'Burada üst üste binenler' }, ...items], this.el.getBoundingClientRect(), {
      placement: 'below',
      owner: this.el,
      onClose: () => selection.hover.set(null),
    });
  }
}
