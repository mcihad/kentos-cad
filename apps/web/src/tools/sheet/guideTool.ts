import type { Axis } from '../../contracts/generated/sheet/Axis';
import type { Guide } from '../../contracts/generated/sheet/Guide';
import { toUm, type PaperPoint, type PaperToolHost } from './tool';

/**
 * Ruler guides (docs/sheet/design.md §5, §11): dragged out of a ruler onto
 * the paper (the top ruler gives a level one, the left an upright one),
 * moved along the paper, dropped back on a ruler to be removed. A guide
 * lands on a tenth of a millimetre; the engine snaps items to it. One undo
 * step each: add, move, remove.
 */

/** How near (CSS px) a press must be to a guide to take it. */
const GRAB_PX = 4;

export class GuideDrag {
  private readonly host: PaperToolHost;
  private readonly axis: Axis;
  /** The guide moved; null for a new one from a ruler. */
  private readonly guide: Guide | null;
  private at: number;
  private remove = false;

  private constructor(host: PaperToolHost, axis: Axis, guide: Guide | null, at: number) {
    this.host = host;
    this.axis = axis;
    this.guide = guide;
    this.at = at;
  }

  /** A new guide dragged out of a ruler: the top ruler's is level (`y`), the left ruler's upright (`x`). */
  static fromRuler(host: PaperToolHost, axis: Axis): GuideDrag {
    return new GuideDrag(host, axis, null, 0);
  }

  /** The sheet's guide under a press on the paper, taken to be moved; null when there is none (or it is locked). */
  static at(host: PaperToolHost, p: PaperPoint): GuideDrag | null {
    const id = host.sheet();
    const guides = host.book().book.sheets.find((s) => s.id === id)?.guides ?? [];
    const tol = (GRAB_PX / host.scale()) * 1000;
    const g = guides.find((x) => !x.locked && Math.abs((x.axis === 'x' ? toUm(p.x) : toUm(p.y)) - x.at) <= tol);
    return g ? new GuideDrag(host, g.axis, g, g.at) : null;
  }

  /** The pointer at `p` on the paper; `overRuler` when it is back over a ruler (the guide would be dropped). */
  move(p: PaperPoint, overRuler: boolean): void {
    this.at = Math.round((this.axis === 'x' ? p.x : p.y) * 10) * 100;
    this.remove = overRuler;
    this.host.overlay({ guide: { axis: this.axis, at: this.at, remove: overRuler } });
  }

  up(p: PaperPoint, overRuler: boolean): void {
    this.move(p, overRuler);
    this.host.overlay(null);
    const owner = { kind: 'sheet' as const, id: this.host.sheet() };
    if (!this.guide) {
      if (!this.remove) this.host.apply([{ op: 'addGuide', owner, guide: { id: this.host.newId(), axis: this.axis, at: this.at, locked: false } }], 'Kılavuz ekle');
      return;
    }
    if (this.remove) this.host.apply([{ op: 'removeGuide', owner, id: this.guide.id }], 'Kılavuzu kaldır');
    else if (this.at !== this.guide.at) this.host.apply([{ op: 'moveGuide', owner, id: this.guide.id, at: this.at }], 'Kılavuzu taşı');
  }

  cancel(): void {
    this.host.overlay(null);
  }
}
