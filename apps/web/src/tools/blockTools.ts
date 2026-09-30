import { Signal } from '../core/signal';
import type { ViewTransform } from '../viewport/Camera';
import type { Vec2 } from '../model/geometry';
import { turnOf } from '../model/blocks';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { SelectionFirstTool } from './modifyTools';
import { strokePaths } from './preview';

/**
 * The block tools (docs/adr/0144 §6).
 *
 * Blok ekle places a block of the drawing at clicked or typed points, as
 * many as wanted, each an insert through `cad.entities.create` on the active
 * layer in the current colour: its own object and undo step, “Ekle”. The
 * block is the one last placed (or chosen), else the drawing's first; Blok
 * (B) takes the next. Ölçek (Ö) and Dönüş (D, degrees counter-clockwise) are
 * typed, Aynala (A) mirrors in the block's x axis; the ghost is the block as
 * it would be placed. The block, scale, turn and mirror are remembered for as
 * long as the app lives. The desktop's `kentos_interaction::block_insert`
 * does the same, step for step.
 */

const NO_BLOCK = 'Çizimde blok yok; önce Blok oluştur ile bir blok tanımlayın.';

export class BlockInsertTool extends PointInputTool {
  readonly id = 'blockInsert';
  protected readonly label = 'Blok ekle';
  /** The block placed last, or chosen (the Bloklar panel marks it): its id. */
  static readonly chosen = new Signal<string | null>(null);
  static get block(): string | null {
    return BlockInsertTool.chosen.value;
  }
  static set block(id: string | null) {
    BlockInsertTool.chosen.set(id);
  }
  static scale = 1;
  /** Degrees, counter-clockwise from east. */
  static rotation = 0;
  static mirror = false;
  private ask: 'scale' | 'rotation' | null = null;

  override activate(): void {
    const blocks = this.ctx.doc.blocks.value;
    if (!blocks.length) {
      this.ctx.log.warn(NO_BLOCK);
      this.ctx.tools.exit();
      return;
    }
    if (!blocks.some((b) => b.id === BlockInsertTool.block)) BlockInsertTool.block = blocks[0].id;
    super.activate();
  }

  private get name(): string {
    return this.ctx.doc.block(BlockInsertTool.block ?? '')?.name ?? '';
  }

  protected promptFor(): string {
    const opts = `[Blok (B) / Ölçek (Ö): ${BlockInsertTool.scale.toFixed(4)} / Dönüş (D): ${BlockInsertTool.rotation.toFixed(4)}° / Aynala (A): ${BlockInsertTool.mirror ? 'açık' : 'kapalı'}]`;
    if (this.ask === 'scale') return `ölçeği yazın ${opts}`;
    if (this.ask === 'rotation') return `dönüş açısını derece olarak yazın ${opts}`;
    return `“${this.name}” için yerleştirme noktasına tıklayın ${opts}`;
  }

  protected override option(key: string): boolean {
    if (key === 'B') {
      // The drawing's next block, round to the first.
      const blocks = this.ctx.doc.blocks.value;
      const at = blocks.findIndex((b) => b.id === BlockInsertTool.block);
      BlockInsertTool.block = blocks[(at + 1) % blocks.length]?.id ?? null;
    } else if (key === 'Ö' || key === 'O') this.ask = 'scale';
    else if (key === 'D') this.ask = 'rotation';
    else if (key === 'A') BlockInsertTool.mirror = !BlockInsertTool.mirror;
    else return false;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    if (this.ask && n !== null && !/[,;@<]/.test(text)) {
      if (this.ask === 'scale' && !(n > 0)) this.ctx.log.warn('Ölçek sıfırdan büyük olmalı.');
      else {
        if (this.ask === 'scale') BlockInsertTool.scale = n;
        else BlockInsertTool.rotation = n;
        this.ask = null;
      }
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    return super.input(text);
  }

  protected onPoint(p: Vec2): void {
    const block = BlockInsertTool.block;
    if (this.ask || !block) return;
    this.writeObjects([
      {
        kind: 'insert',
        block,
        p,
        scale: BlockInsertTool.scale,
        rotation: turnOf(BlockInsertTool.rotation),
        ...(BlockInsertTool.mirror && { mirror: true }),
      },
    ]);
  }

  /** The block as it would be placed at the cursor; nothing while a value is asked for. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const block = BlockInsertTool.block;
    if (!this.hover || this.ask || !block) return;
    const paths = this.ctx.view.blockOutlines(block, this.hover, BlockInsertTool.scale, turnOf(BlockInsertTool.rotation), BlockInsertTool.mirror);
    strokePaths(g, view, paths, { color: this.ctx.view.palette.accent, dash: [4, 3] });
    this.drawTracking(g, view);
  }
}

/**
 * Blok oluştur: the objects are picked first (click, window; a selection made
 * before the tool is taken as it is), then the base point, clicked or typed;
 * the window that names the block opens then (`ctx.blocks.define`,
 * ui/blocks/BlockDefineDialog.ts) and writes it through `cad.blocks.define`.
 * Objects on a locked layer are taken too: the definition copies them. The
 * desktop's `kentos_interaction::block_define` does the same.
 */
export class BlockDefineTool extends SelectionFirstTool {
  readonly id = 'blockDefine';
  protected readonly label = 'Blok oluştur';

  protected begin(): void {}

  protected stagePrompt(): string {
    return 'taban noktasına tıklayın ya da Y,X yazın';
  }

  protected point(p: Vec2): void {
    const uids = this.selectedUids();
    this.ctx.tools.exit();
    this.ctx.blocks.define(p, uids);
  }
}
