import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import { clearLocks, lockPickStep, lockReference, lockToward, NO_LOCK_EDGE, NO_LOCK_REFERENCE, NO_LOCKS, pickedEdge, type LockPick } from './locks';
import { drawLocks } from './lockGuides';
import type { CanvasPalette } from '../render/color';
import type { Camera } from '../viewport/Camera';
import type { Tool, ToolDescriptor, ToolGroup, ToolPointer } from './Tool';

export class ToolManager {
  readonly activeId = new Signal('select');
  readonly prompt = new Signal('');
  /** Nesneye paralel or dik waiting for its edge (docs/adr/0166 §3): the next press picks it instead of reaching the tool. */
  readonly lockPick = new Signal<LockPick | null>(null);
  /** The press that picked the edge: its release does not reach the tool. */
  private pickPressed = false;
  private registry = new Map<string, ToolDescriptor>();
  private current: Tool | null = null;
  private promptSub: Disposable | null = null;
  private lastRepeatable: string | null = null;
  /** Tools suspended under a transparent one (point calculator), innermost last. */
  private parents: Tool[] = [];
  private readonly ctx: AppContext;
  /**
   * While set, no tool starts (a large import writing into the drawing lets
   * only the view move, app/hold.ts): the one running stays, and the hold
   * says why.
   */
  hold: (() => void) | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  register(d: ToolDescriptor): void {
    this.registry.set(d.id, d);
  }

  get(id: string): ToolDescriptor | undefined {
    return this.registry.get(id);
  }

  list(): ToolDescriptor[] {
    return [...this.registry.values()];
  }

  byGroup(): Map<ToolGroup, ToolDescriptor[]> {
    const m = new Map<ToolGroup, ToolDescriptor[]>();
    for (const d of this.registry.values()) m.set(d.group, [...(m.get(d.group) ?? []), d]);
    return m;
  }

  get active(): Tool {
    if (!this.current) this.activate('select');
    return this.current!;
  }

  get activeDescriptor(): ToolDescriptor | undefined {
    return this.registry.get(this.activeId.value);
  }

  activate(id: string): void {
    const d = this.registry.get(id);
    if (!d) return;
    if (this.hold) return this.hold();
    this.dropNested();
    this.current?.deactivate?.();
    this.promptSub?.();
    // A new command starts with nothing locked (docs/adr/0166 §1).
    this.ctx.settings.locks.set(NO_LOCKS);
    this.lockPick.set(null);
    this.current = d.create(this.ctx);
    if (id !== 'select' && id !== 'pan') this.lastRepeatable = id;
    this.promptSub = this.current.prompt.subscribe(() => this.showPrompt(), true);
    // The command's name first, then what the tool says as it starts (the erase tool deletes a
    // selection at once): the history reads in order, as on the desktop (docs/adr/0029).
    if (id !== 'select') this.ctx.log.command(d.label);
    this.current.activate?.();
    this.activeId.set(id);
    this.ctx.view.requestRender();
  }

  /**
   * Runs a tool that is not in the catalog (e.g. paste with its clipboard
   * contents). It is not remembered for "repeat last".
   */
  run(tool: Tool, label: string): void {
    if (this.hold) return this.hold();
    this.dropNested();
    this.current?.deactivate?.();
    this.promptSub?.();
    this.ctx.settings.locks.set(NO_LOCKS);
    this.lockPick.set(null);
    this.current = tool;
    this.promptSub = tool.prompt.subscribe(() => this.showPrompt(), true);
    this.ctx.log.command(label);
    tool.activate?.();
    this.activeId.set(tool.id);
    this.ctx.view.requestRender();
  }

  /**
   * Runs `child` on top of the active tool without ending it (a transparent
   * command, like AutoCAD's 'CAL). The parent keeps its state; `unnest`
   * brings it back and can hand it the child's result as a clicked point.
   */
  nest(child: Tool, label: string): void {
    if (!this.current) return;
    if (this.hold) return this.hold();
    this.parents.push(this.current);
    this.promptSub?.();
    this.current = child;
    this.promptSub = child.prompt.subscribe(() => this.showPrompt(), true);
    this.ctx.log.command(label);
    child.activate?.();
    this.ctx.view.requestOverlay();
  }

  /** Ends the transparent tool; `point` goes to the resumed tool as if clicked. */
  unnest(point: Vec2 | null): void {
    const parent = this.parents.pop();
    if (!parent) return;
    this.current?.deactivate?.();
    this.promptSub?.();
    this.current = parent;
    this.promptSub = parent.prompt.subscribe(() => this.showPrompt(), true);
    if (point && !parent.acceptPoint?.(point)) this.ctx.log.warn('Çalışan araç şu adımda nokta beklemiyor; hesaplanan nokta kullanılmadı.');
    this.ctx.view.requestOverlay();
  }

  /** Whether a transparent tool (point calculator) is running over another. */
  get nested(): boolean {
    return this.parents.length > 0;
  }

  private dropNested(): void {
    if (!this.parents.length) return;
    this.current?.deactivate?.();
    this.current = this.parents[0];
    this.parents = [];
  }

  /** Esc: leave the running tool and return to selection; an awaited lock edge, then the locks go first (docs/adr/0166 §1, §3). */
  exit(): void {
    if (this.lockPick.value) return this.endPick();
    if (lockReference(this.ctx) && clearLocks(this.ctx)) return;
    if (this.current?.cancel?.()) return;
    if (this.parents.length) return this.unnest(null);
    if (this.activeId.value === 'select') {
      this.ctx.selection.clear();
      return;
    }
    this.activate('select');
  }

  /** The prompt: the running tool's, or the awaited lock edge's step under the tool's name. */
  private showPrompt(): void {
    const pick = this.lockPick.value;
    const own = this.current?.prompt.value ?? '';
    if (!pick) return this.prompt.set(own);
    // The tool's name stays before the step, as the tool writes it (“Çoklu çizgi: …”; promptOptions.ts reads it so).
    const colon = own.indexOf(':');
    const bracket = own.indexOf('[');
    const name = colon > 0 && (bracket < 0 || colon < bracket) ? own.slice(0, colon) : '';
    this.prompt.set(name ? `${name}: ${lockPickStep(pick)}` : lockPickStep(pick));
  }

  private endPick(): void {
    this.lockPick.set(null);
    this.showPrompt();
    this.ctx.view.requestOverlay();
  }

  /** Nesneye paralel or dik (docs/adr/0166 §3): the next press on the drawing picks the edge the direction is taken from. */
  pickLockEdge(pick: LockPick): boolean {
    if (!lockReference(this.ctx)) return void this.ctx.log.warn(NO_LOCK_REFERENCE), false;
    this.lockPick.set(pick);
    this.showPrompt();
    return true;
  }

  /** A press on the drawing while an edge is awaited: the edge under it gives the direction lock, or the wait goes on and says why. Whether the press was the pick's. */
  pickPress(p: ToolPointer): boolean {
    const pick = this.lockPick.value;
    if (!pick) return false;
    this.pickPressed = true;
    const found = pickedEdge(this.ctx, p);
    if (!found) return this.ctx.log.warn(NO_LOCK_EDGE), true;
    this.endPick();
    if (lockToward(this.ctx, { kind: pick, u: found.u })) this.ctx.settings.locks.set({ ...this.ctx.settings.locks.value, edge: found.edge });
    return true;
  }

  /** The release of the press that picked a lock's edge: the pick's, not the tool's. */
  pickRelease(): boolean {
    const was = this.pickPressed;
    this.pickPressed = false;
    return was;
  }

  /** The digitizing locks' guides and tag over the drawing (docs/adr/0166 §6); the viewport calls it after the tool's preview. */
  drawLocks(g: CanvasRenderingContext2D, cam: Camera, pal: CanvasPalette, cursor: Vec2 | null): void {
    drawLocks(this.ctx, g, cam, pal, cursor);
  }

  repeatLast(): void {
    if (this.lastRepeatable) this.activate(this.lastRepeatable);
  }

  get lastToolLabel(): string | null {
    return this.lastRepeatable ? (this.registry.get(this.lastRepeatable)?.label ?? null) : null;
  }
}
