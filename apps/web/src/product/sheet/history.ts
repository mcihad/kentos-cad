import { Signal, type ReadonlySignal } from '../../core/signal';
import type { Op } from '../../contracts/generated/sheet/Op';
import { bookText, type BookText, type SheetEngine } from './engine';

/**
 * The sheet mode's own undo (docs/sheet/design.md §4, §10): apart from the
 * drawing's. Every edit is a list of the engine's operations applied as one
 * step (`applyOps`); the engine gives back the operations that take it back
 * and the step's Turkish name (“Taşı: Harita”), both kept. Undo applies the
 * kept inverse, and what the engine gives back for that is the redo; so each
 * direction is always the engine's own inverse of the last, and a book
 * undone is the book it was (the engine proves the identity on its
 * fixtures). A refusal of the engine leaves the book and both stacks as
 * they were and comes up as its `SheetEngineError`.
 */

export interface HistoryStep {
  /** The step's name, as the menus and the message cell say it. */
  readonly label: string;
  /** Applied to take the step back. */
  readonly undo: readonly Op[];
  /** Applied to make it again. */
  readonly redo: readonly Op[];
}

/** Steps kept back (each holds only operations: pictures' bytes are the asset store's, design §3.4). */
export const HISTORY_LIMIT = 200;

export class SheetHistory {
  private readonly engine: SheetEngine;
  private readonly current: Signal<BookText>;
  private readonly undoable = new Signal(false);
  private readonly redoable = new Signal(false);
  private past: HistoryStep[] = [];
  private future: HistoryStep[] = [];

  constructor(engine: SheetEngine, book: BookText) {
    this.engine = engine;
    this.current = new Signal(book, () => false);
  }

  /** The book as it is now; set on every edit, undo, redo and load. */
  get book(): ReadonlySignal<BookText> {
    return this.current;
  }

  get canUndo(): ReadonlySignal<boolean> {
    return this.undoable;
  }

  get canRedo(): ReadonlySignal<boolean> {
    return this.redoable;
  }

  /** The name of the step Geri al would take back; null when there is none. */
  get undoLabel(): string | null {
    return this.past.at(-1)?.label ?? null;
  }

  get redoLabel(): string | null {
    return this.future.at(-1)?.label ?? null;
  }

  /**
   * Applies operations as one step and keeps it. `label` names it when the
   * caller says it better than the engine (“Şablondan pafta”); otherwise the
   * engine's name. Returns the step's name; throws the engine's refusal.
   */
  apply(ops: readonly Op[], label?: string): string {
    if (!ops.length) return label ?? '';
    const a = this.engine.applyOps(this.current.value, ops);
    const step: HistoryStep = { label: label ?? a.label, undo: a.inverse, redo: ops };
    this.past.push(step);
    if (this.past.length > HISTORY_LIMIT) this.past.splice(0, this.past.length - HISTORY_LIMIT);
    this.future = [];
    this.set(bookText(a.book));
    return step.label;
  }

  /** Takes the last step back; its name, or null when there was none. */
  undo(): string | null {
    const step = this.past.at(-1);
    if (!step) return null;
    const a = this.engine.applyOps(this.current.value, step.undo);
    this.past.pop();
    this.future.push({ label: step.label, undo: step.undo, redo: a.inverse });
    this.set(bookText(a.book));
    return step.label;
  }

  /** Makes the last undone step again; its name, or null when there was none. */
  redo(): string | null {
    const step = this.future.at(-1);
    if (!step) return null;
    const a = this.engine.applyOps(this.current.value, step.redo);
    this.future.pop();
    this.past.push({ label: step.label, undo: a.inverse, redo: step.redo });
    this.set(bookText(a.book));
    return step.label;
  }

  /** Another book (a project opened, a file read): the steps of the last one cannot apply to it. */
  reset(book: BookText): void {
    this.past = [];
    this.future = [];
    this.set(book);
  }

  private set(book: BookText): void {
    this.undoable.set(this.past.length > 0);
    this.redoable.set(this.future.length > 0);
    this.current.set(book);
  }
}
