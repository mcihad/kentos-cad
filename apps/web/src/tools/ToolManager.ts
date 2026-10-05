import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import { clearLocks, followLocks as followLocksHere, lockPickStep, lockReference, lockToward, NO_LOCK_EDGE, NO_LOCK_REFERENCE, NO_LOCKS, NO_REFERENCE_TOOL, pickedEdge, takesPoints, type LockPick } from './locks';
import { pointFromText } from './tracking';
import { drawLocks } from './lockGuides';
import type { CanvasPalette } from '../render/color';
import type { Camera } from '../viewport/Camera';
import type { Tool, ToolDescriptor, ToolGroup, ToolPointer } from './Tool';
import type { TemplateRun } from './templateStamp';

export class ToolManager {
  readonly activeId = new Signal('select');
  readonly prompt = new Signal('');
  /** Nesneye paralel or dik waiting for its edge (docs/adr/0166 §3): the next press picks it instead of reaching the tool. */
  readonly lockPick = new Signal<LockPick | null>(null);
  /** The press that picked the edge, or set the reference: its release does not reach the tool. */
  private pickPressed = false;
  /** Referans noktası's or Yapım kipi's point (docs/adr/0166 §5): the locks and relative input are measured from it. */
  readonly reference = new Signal<Vec2 | null>(null);
  /** The tool's own last point when the reference was set: once it moves (a corner was placed), a one-shot reference is done. */
  private referenceFrom: Vec2 | null = null;
  /** Referans noktası asked: the next press or typed point is the reference. */
  readonly referenceWait = new Signal(false);
  /** Yapım kipi: every press and typed point renews the reference. */
  readonly construction = new Signal(false);
  private registry = new Map<string, ToolDescriptor>();
  private current: Tool | null = null;
  private promptSub: Disposable | null = null;
  private lastRepeatable: string | null = null;
  /** The object template the last remembered command drew with (docs/adr/0176 §3): Son komutu yinele starts it again. */
  private lastTemplate: string | null = null;
  /** The object templates drawn with in this session, the newest first (at most five; the Şablonlar panel's first group). */
  readonly recentTemplates = new Signal<readonly string[]>([]);
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

  /**
   * Starts a tool by its id, dropping whatever ran; with `template`, it draws with that object template
   * (app/objectTemplates.ts, docs/adr/0176 §3): the template's colour and weight are the current ones until the tool
   * ends, and every object it writes takes the template's stamp.
   */
  activate(id: string, template: TemplateRun | null = null): void {
    const d = this.registry.get(id);
    if (!d) return;
    if (this.hold) return this.hold();
    this.dropNested();
    this.current?.deactivate?.();
    this.promptSub?.();
    // A new command starts with nothing locked (docs/adr/0166 §1).
    this.ctx.settings.locks.set(NO_LOCKS);
    this.lockPick.set(null);
    this.dropReference();
    // A template's run ends with its tool; the next one starts with its own.
    this.releaseTemplate();
    if (template) {
      this.ctx.settings.template.set(template);
      this.ctx.settings.color.set(template.color);
      this.ctx.settings.lineWeight.set(template.lineWeight);
      this.recentTemplates.set([template.id, ...this.recentTemplates.value.filter((id) => id !== template.id)].slice(0, 5));
    }
    this.current = d.create(this.ctx);
    if (id !== 'select' && id !== 'pan') {
      this.lastRepeatable = id;
      this.lastTemplate = template?.id ?? null;
    }
    this.promptSub = this.current.prompt.subscribe(() => this.showPrompt(), true);
    // The command's name first, then what the tool says as it starts (the erase tool deletes a
    // selection at once): the history reads in order, as on the desktop (docs/adr/0029).
    if (id !== 'select') this.ctx.log.command(template ? `${template.name} · ${d.label}` : d.label);
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
    this.dropReference();
    this.releaseTemplate();
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
    // Then a reference asked for, Yapım kipi, and the reference with the locks (docs/adr/0166 §5).
    if (this.referenceWait.value) {
      this.referenceWait.set(false);
      return this.showPrompt();
    }
    if (this.construction.value) return void this.setConstruction(false);
    if (this.reference.value && lockReference(this.ctx)) {
      this.reference.set(null);
      if (!clearLocks(this.ctx)) this.ctx.log.info('Kilitler kaldırıldı.');
      return this.showPrompt();
    }
    if (lockReference(this.ctx) && clearLocks(this.ctx)) return;
    if (this.current?.cancel?.()) return;
    if (this.parents.length) return this.unnest(null);
    if (this.activeId.value === 'select') {
      this.ctx.selection.clear();
      return;
    }
    this.activate('select');
  }

  /** The prompt: the running tool's, or the awaited lock edge's, the reference's or Yapım kipi's step under the tool's name. */
  private showPrompt(): void {
    const pick = this.lockPick.value;
    const own = this.current?.prompt.value ?? '';
    const step = pick
      ? lockPickStep(pick)
      : this.referenceWait.value
        ? 'referans noktasını belirtin [Vazgeç (Esc)]'
        : this.construction.value
          ? 'yapım noktasını belirtin; köşe olmaz [Vazgeç (Esc)]'
          : null;
    // An object template's name before the tool's (“Parsel sınırı · Kapalı alan: …”, docs/adr/0176 §3); a tool run over it speaks for itself.
    const template = this.parents.length ? null : this.ctx.settings.template.value;
    const titled = (text: string) => (template && text ? `${template.name} · ${text}` : text);
    if (!step) return this.prompt.set(titled(own));
    // The tool's name stays before the step, as the tool writes it (“Çoklu çizgi: …”; promptOptions.ts reads it so).
    const colon = own.indexOf(':');
    const bracket = own.indexOf('[');
    const name = colon > 0 && (bracket < 0 || colon < bracket) ? own.slice(0, colon) : '';
    this.prompt.set(titled(name ? `${name}: ${step}` : step));
  }

  /** Ends an object template's run, if one is: the colour and weight it found come back (docs/adr/0176 §3). */
  private releaseTemplate(): void {
    const run = this.ctx.settings.template.value;
    if (!run) return;
    this.ctx.settings.template.set(null);
    this.ctx.settings.color.set(run.before.color);
    this.ctx.settings.lineWeight.set(run.before.lineWeight);
  }

  /** No reference, none asked, Yapım kipi off: a new command's start. */
  private dropReference(): void {
    this.reference.set(null);
    this.referenceFrom = null;
    this.referenceWait.set(false);
    this.construction.set(false);
  }

  /** The running tool's own last point. */
  private ownFrom(): Vec2 | null {
    return this.current?.snapFrom?.() ?? null;
  }

  /** Referans noktası (docs/adr/0166 §5): the next press or typed point is the reference, not a corner. */
  askReference(): boolean {
    if (!takesPoints(this.ctx)) return void this.ctx.log.warn(NO_REFERENCE_TOOL), false;
    this.referenceWait.set(true);
    this.showPrompt();
    return true;
  }

  /** Yapım kipi on or off: while on, every press and typed point renews the reference; off, the next point is a corner. */
  setConstruction(on: boolean): boolean {
    if (on && !takesPoints(this.ctx)) return void this.ctx.log.warn(NO_REFERENCE_TOOL), false;
    this.construction.set(on);
    this.referenceWait.set(false);
    // The reference stays for the next corner, then goes.
    this.referenceFrom = this.ownFrom();
    this.ctx.log.info(on ? 'Yapım kipi açık: tıklanan ve yazılan noktalar köşe olmaz, referansı yeniler.' : 'Yapım kipi kapalı: sonraki nokta köşedir.');
    this.showPrompt();
    return true;
  }

  /** Whether the next press or typed point is a reference rather than a corner. */
  private takesReference(): boolean {
    return this.referenceWait.value || this.construction.value;
  }

  private setReference(p: Vec2): void {
    this.referenceFrom = this.ownFrom();
    this.referenceWait.set(false);
    this.reference.set(p);
    this.ctx.log.info(`Referans noktası: ${this.ctx.format.point(p)}`);
    this.showPrompt();
    followLocksHere(this.ctx);
  }

  /** Typed where a reference is taken: the point (relative to the reference so far) is the new reference. Whether it was taken. */
  typedReference(text: string): boolean {
    if (!this.takesReference()) return false;
    const p = pointFromText(this.ctx, text, lockReference(this.ctx), this.ctx.view.cursorWorld.value);
    if (!p) return false;
    this.setReference(p);
    return true;
  }

  /** After a point was placed: a one-shot reference is done once the tool's own last point moved (docs/adr/0166 §5). */
  followReference(): void {
    if (!this.reference.value || this.takesReference()) return;
    const own = this.ownFrom();
    const same = own === null || this.referenceFrom === null ? own === this.referenceFrom : own.x === this.referenceFrom.x && own.y === this.referenceFrom.y;
    if (!same) this.reference.set(null);
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
    // Referans noktası and Yapım kipi: the press is the reference, not a corner (docs/adr/0166 §5).
    if (!pick && this.takesReference() && takesPoints(this.ctx)) {
      this.pickPressed = true;
      this.setReference(p.world);
      return true;
    }
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
    // An object template is started again as itself (docs/adr/0176 §3).
    if (this.lastTemplate !== null) return void this.ctx.commands.execute('template.draw', this.lastTemplate);
    if (this.lastRepeatable) this.activate(this.lastRepeatable);
  }

  get lastToolLabel(): string | null {
    const template = this.lastTemplate === null ? undefined : this.ctx.styles.library.get(this.lastTemplate);
    if (template) return template.name;
    return this.lastRepeatable ? (this.registry.get(this.lastRepeatable)?.label ?? null) : null;
  }
}
