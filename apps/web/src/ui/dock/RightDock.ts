import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h } from '../dom';
import { LayersPanel } from '../layers/LayersPanel';
import { ProcessingPanel } from '../processing/ProcessingPanel';
import { PropertiesPanel } from '../properties/PropertiesPanel';
import { splitter } from '../widgets/Splitter';
import { dockTabs } from './dockTabs';
import { filterOf } from '../../app/workspaces';

/**
 * Right dock: the upper slot holds the layer tree or the processing
 * toolbox (tabs), attributes sit below, split by a draggable divider.
 */
export class RightDock extends Component {
  readonly el: HTMLElement;
  readonly processing: ProcessingPanel;
  private readonly layers: LayersPanel;
  private readonly props: PropertiesPanel;

  constructor(ctx: AppContext) {
    super();
    const { ui } = ctx;
    this.layers = new LayersPanel(ctx);
    this.processing = new ProcessingPanel(ctx);
    this.props = new PropertiesPanel(ctx);
    this.layers.setTabs(dockTabs(ctx, 'layers'));
    this.processing.setTabs(dockTabs(ctx, 'processing'));
    this.layers.el.classList.add('dock__top');
    this.processing.el.classList.add('dock__top');

    let startFrac = 0;
    let height = 1;
    const split = splitter({
      orientation: 'horizontal',
      label: 'Üst panel ile öznitelikler arası',
      onStart: () => {
        startFrac = ui.layersFraction.value;
        height = this.el.clientHeight || 1;
      },
      onDrag: (dy) => ui.layersFraction.set(Math.min(0.85, Math.max(0.15, startFrac + dy / height))),
      onReset: () => ui.layersFraction.set(0.5),
    });
    this.d.add(split.dispose);

    this.el = h('aside', { class: 'dock', 'aria-label': 'Katmanlar, işlemler ve öznitelikler' }, this.layers.el, this.processing.el, split.el, this.props.el);
    this.d.add(ui.layersFraction.subscribe((f) => this.el.style.setProperty('--layers-frac', String(f)), true));
    const sync = () => {
      // A work mode without processing (CAD) keeps the layers in the slot, whatever tab was last open.
      const processing = filterOf(ctx).menu('processing');
      for (const b of this.el.querySelectorAll<HTMLElement>('[data-dock-tab="processing"]')) b.hidden = !processing;
      const tab = processing ? ui.dockTab.value : 'layers';
      this.layers.el.hidden = tab !== 'layers';
      this.processing.el.hidden = tab !== 'processing';
      const top = tab === 'layers' ? this.layers : this.processing;
      this.el.dataset.layout = top.collapsed.value ? 'props' : this.props.collapsed.value ? 'top' : 'split';
    };
    this.d.add(watchAll([ui.dockTab, this.layers.collapsed, this.processing.collapsed, this.props.collapsed, ctx.doc.settings.workspace], sync));
    sync();
  }

  override dispose(): void {
    this.layers.dispose();
    this.processing.dispose();
    this.props.dispose();
    super.dispose();
  }
}
