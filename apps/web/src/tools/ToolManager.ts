import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import type { Tool, ToolDescriptor, ToolGroup } from './Tool';

export class ToolManager {
  readonly activeId = new Signal('select');
  readonly prompt = new Signal('');
  private registry = new Map<string, ToolDescriptor>();
  private current: Tool | null = null;
  private promptSub: Disposable | null = null;
  private lastRepeatable: string | null = null;
  /** Tools suspended under a transparent one (point calculator), innermost last. */
  private parents: Tool[] = [];
  private readonly ctx: AppContext;

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
    this.dropNested();
    this.current?.deactivate?.();
    this.promptSub?.();
    this.current = d.create(this.ctx);
    if (id !== 'select' && id !== 'pan') this.lastRepeatable = id;
    this.promptSub = this.current.prompt.subscribe((p) => this.prompt.set(p), true);
    this.current.activate?.();
    this.activeId.set(id);
    if (id !== 'select') this.ctx.log.command(d.label);
    this.ctx.view.requestRender();
  }

  /**
   * Runs a tool that is not in the catalog (e.g. paste with its clipboard
   * contents). It is not remembered for "repeat last".
   */
  run(tool: Tool, label: string): void {
    this.dropNested();
    this.current?.deactivate?.();
    this.promptSub?.();
    this.current = tool;
    this.promptSub = tool.prompt.subscribe((p) => this.prompt.set(p), true);
    tool.activate?.();
    this.activeId.set(tool.id);
    this.ctx.log.command(label);
    this.ctx.view.requestRender();
  }

  /**
   * Runs `child` on top of the active tool without ending it (a transparent
   * command, like AutoCAD's 'CAL). The parent keeps its state; `unnest`
   * brings it back and can hand it the child's result as a clicked point.
   */
  nest(child: Tool, label: string): void {
    if (!this.current) return;
    this.parents.push(this.current);
    this.promptSub?.();
    this.current = child;
    this.promptSub = child.prompt.subscribe((p) => this.prompt.set(p), true);
    child.activate?.();
    this.ctx.log.command(label);
    this.ctx.view.requestOverlay();
  }

  /** Ends the transparent tool; `point` goes to the resumed tool as if clicked. */
  unnest(point: Vec2 | null): void {
    const parent = this.parents.pop();
    if (!parent) return;
    this.current?.deactivate?.();
    this.promptSub?.();
    this.current = parent;
    this.promptSub = parent.prompt.subscribe((p) => this.prompt.set(p), true);
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

  /** Esc: leave the running tool and return to selection. */
  exit(): void {
    if (this.current?.cancel?.()) return;
    if (this.parents.length) return this.unnest(null);
    if (this.activeId.value === 'select') {
      this.ctx.selection.clear();
      return;
    }
    this.activate('select');
  }

  repeatLast(): void {
    if (this.lastRepeatable) this.activate(this.lastRepeatable);
  }

  get lastToolLabel(): string | null {
    return this.lastRepeatable ? (this.registry.get(this.lastRepeatable)?.label ?? null) : null;
  }
}
