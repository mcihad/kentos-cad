import type { ExprField, ExprSection } from '../../model/expression/builder';
import { exprFlow, exprFlowEdit, type Flow, type FlowEdit, type FlowNode, type FlowTree } from '../../model/expression/flow';
import { FlowInspector } from './FlowInspector';
import { FlowView } from './FlowView';

/**
 * The builder's Akış view (DESIGN.md §7.16, docs/adr/0101): the flow of the
 * expression being written, its selected node's inspector and its own undo.
 * The text stays the builder's: a change here goes to the core, which
 * answers with the new text (canonical); the builder writes it into its
 * editor, and its check, preview and Tamam work on it as ever. Nodes put
 * down but not connected are kept here, with their places, while the
 * dialog is open.
 */
export interface FlowModeOptions {
  readonly fields: () => readonly ExprField[];
  readonly catalog: () => readonly ExprSection[];
  /** The builder's expression. */
  readonly text: () => string;
  /** Writes an expression the flow changed into the builder's editor. */
  readonly setText: (text: string) => void;
  /** An expression's value on the previewed object, as the preview writes it (undefined: none). */
  readonly value: (text: string) => string | undefined;
  /** The help of a node's key (null: the builder's own). */
  readonly showHelp: (key: string | null) => void;
  /** A change the core refused, in its words. */
  readonly fail: (message: string) => void;
}

/** How many changes back Ctrl+Z goes. */
const UNDO = 100;

export class FlowMode {
  readonly view: FlowView;
  readonly inspector: FlowInspector;
  private readonly opts: FlowModeOptions;
  /** The trees not connected to the result (tree 0 is the builder's text). */
  private apart: FlowTree[] = [];
  private flow: Flow | null = null;
  private selected: string | null = null;
  private readonly undoStack: FlowTree[][] = [];
  private readonly redoStack: FlowTree[][] = [];

  constructor(opts: FlowModeOptions) {
    this.opts = opts;
    this.inspector = new FlowInspector({ fields: opts.fields, catalog: opts.catalog, edit: (c) => this.edit(c) });
    this.view = new FlowView({
      select: (id) => this.select(id),
      connect: (from, to, port) => this.edit({ op: 'connect', from, to, port }),
      disconnect: (to, port) => this.edit({ op: 'disconnect', to, port }),
      move: (tree, at) => this.edit({ op: 'move', tree, at }),
      remove: (id) => this.edit({ op: 'remove', node: id }),
      addPort: (id) => this.edit({ op: 'addPort', node: id }),
      removePort: (id, port) => this.edit({ op: 'removePort', node: id, port }),
      drop: (key, at, to) => this.put({ op: 'add', key, at }, to),
      open: (id) => {
        this.select(id);
        this.inspector.focusValue();
      },
      undo: () => this.undo(),
      redo: () => this.redo(),
    });
  }

  dispose(): void {
    this.view.dispose();
  }

  private trees(): FlowTree[] {
    return [{ text: this.opts.text() }, ...this.apart];
  }

  /** Reads the text (it changed, or the view is shown) and draws the flow. */
  refresh(): void {
    this.flow = exprFlow(this.trees(), this.opts.fields());
    if (this.selected && !this.node(this.selected)) this.selected = null;
    this.view.render(this.flow, this.selected, this.values());
    this.inspector.show(this.selected ? (this.node(this.selected) ?? null) : null);
    if (this.selected) this.view.reveal(this.selected);
  }

  /** The previewed object changed: the nodes' values. */
  preview(): void {
    if (this.flow) this.view.setValues(this.values());
  }

  /** A palette entry (a double click in the tree, an operator button): see `put`. */
  add(key: string): void {
    this.put({ op: 'add', key, at: this.free() });
  }

  /** An expression as the language writes it (a field's value from the help). */
  addText(text: string): void {
    this.put({ op: 'addText', text, at: this.free() });
  }

  private node(id: string): FlowNode | undefined {
    return this.flow?.nodes.find((n) => n.id === id);
  }

  private values(): Map<string, string> {
    const out = new Map<string, string>();
    const f = this.flow;
    if (!f || f.error) return out;
    for (const n of f.nodes) {
      if (!n.whole || !n.text) continue;
      const v = this.opts.value(n.text);
      if (v !== undefined) out.set(n.id, v);
    }
    return out;
  }

  private select(id: string | null): void {
    this.selected = id;
    const n = id ? this.node(id) : undefined;
    this.inspector.show(n ?? null);
    this.opts.showHelp(n?.key ?? null);
    if (this.flow) this.view.render(this.flow, this.selected, this.values());
  }

  /**
   * Where a new node goes when nothing says: left of the selected node's
   * empty input, else under everything, below the middle of the view.
   */
  private free(): [number, number] {
    const f = this.flow!;
    const target = this.target();
    if (target) {
      const n = this.node(target.node)!;
      return [n.x - f.column, n.y + n.ports[target.port].y - f.head / 2];
    }
    const c = this.view.centre();
    const width = f.nodes[0]?.w ?? 140;
    return [Math.round(c.x - width / 2), Math.round(f.bounds[3] + 24)];
  }

  /** The input a new node goes into: the selected node's first empty one, else the result's when empty. */
  private target(): { node: string; port: number } | null {
    const sel = this.selected ? this.node(this.selected) : undefined;
    const k = sel ? sel.ports.findIndex((p) => !p.from) : -1;
    if (sel && k >= 0) return { node: sel.id, port: k };
    if (!this.opts.text().trim()) return { node: 'r', port: 0 };
    return null;
  }

  /** Puts a node down and connects it into `to` (or where `target` says). */
  private put(add: FlowEdit, to?: { readonly node: string; readonly port: number }): void {
    const into = to ?? this.target();
    const fields = this.opts.fields();
    const before = this.trees();
    try {
      const added = exprFlowEdit(before, add, fields);
      let trees = added.trees;
      let focus = added.focus;
      if (into && focus) {
        const joined = exprFlowEdit(trees, { op: 'connect', from: focus, to: into.node, port: into.port }, fields);
        trees = joined.trees;
        focus = joined.focus;
      }
      this.commit(before, trees, focus);
    } catch (err) {
      this.opts.fail((err as Error).message);
    }
  }

  private edit(change: FlowEdit): void {
    const before = this.trees();
    try {
      const r = exprFlowEdit(before, change, this.opts.fields());
      this.commit(before, r.trees, change.op === 'remove' ? undefined : r.focus);
    } catch (err) {
      this.opts.fail((err as Error).message);
    }
  }

  private commit(before: FlowTree[], after: readonly FlowTree[], focus: string | undefined): void {
    this.undoStack.push(before);
    if (this.undoStack.length > UNDO) this.undoStack.shift();
    this.redoStack.length = 0;
    this.restore(after, focus ?? null);
  }

  private restore(trees: readonly FlowTree[], focus: string | null): void {
    this.apart = trees.slice(1).map((t) => ({ ...t }));
    this.selected = focus;
    const text = trees[0]?.text ?? '';
    // The builder's editor takes the new text (its check and preview follow) and asks for `refresh`.
    if (text !== this.opts.text()) this.opts.setText(text);
    else this.refresh();
    this.opts.showHelp(focus ? (this.node(focus)?.key ?? null) : null);
  }

  private undo(): void {
    const t = this.undoStack.pop();
    if (!t) return;
    this.redoStack.push(this.trees());
    this.restore(t, null);
  }

  private redo(): void {
    const t = this.redoStack.pop();
    if (!t) return;
    this.undoStack.push(this.trees());
    this.restore(t, null);
  }
}
