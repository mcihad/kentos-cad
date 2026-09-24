import type { Disposable } from './disposable';
import { Emitter } from './emitter';
import type { ReadonlySignal } from './signal';
import { foldTurkish } from './text';

/**
 * Everything the user can trigger — from a menu, toolbar, shortcut or the
 * command line — is a Command. UI never calls features directly.
 */
export interface Command {
  id: string;
  title: string;
  category?: string;
  icon?: string;
  description?: string;
  /** Names accepted by the command line, e.g. ["L", "LINE", "CIZGI"]. */
  aliases?: readonly string[];
  run(args?: unknown): void;
  isEnabled?(): boolean;
  isChecked?(): boolean;
  /** Signals whose change may alter enabled/checked state. */
  watch?: readonly ReadonlySignal<unknown>[];
}

interface CommandEvents {
  registered: Command;
  executed: { command: Command; args?: unknown };
  missing: { id: string };
}

const foldAlias = foldTurkish;

export class CommandRegistry {
  readonly events = new Emitter<CommandEvents>();
  private commands = new Map<string, Command>();
  private aliases = new Map<string, string>();

  register(cmd: Command): Disposable {
    if (this.commands.has(cmd.id)) console.warn(`Command "${cmd.id}" re-registered`);
    this.commands.set(cmd.id, cmd);
    for (const a of cmd.aliases ?? []) this.aliases.set(foldAlias(a), cmd.id);
    this.events.emit('registered', cmd);
    return () => {
      this.commands.delete(cmd.id);
      for (const a of cmd.aliases ?? []) this.aliases.delete(foldAlias(a));
    };
  }

  registerAll(cmds: readonly Command[]): Disposable {
    const ds = cmds.map((c) => this.register(c));
    return () => ds.forEach((d) => d());
  }

  get(id: string): Command | undefined {
    return this.commands.get(id);
  }

  all(): Command[] {
    return [...this.commands.values()];
  }

  isEnabled(id: string): boolean {
    const c = this.commands.get(id);
    return !!c && (c.isEnabled?.() ?? true);
  }

  execute(id: string, args?: unknown): boolean {
    const cmd = this.commands.get(id);
    if (!cmd) {
      this.events.emit('missing', { id });
      return false;
    }
    if (cmd.isEnabled && !cmd.isEnabled()) return false;
    cmd.run(args);
    this.events.emit('executed', { command: cmd, args });
    return true;
  }

  byAlias(text: string): Command | undefined {
    const id = this.aliases.get(foldAlias(text));
    return id ? this.commands.get(id) : undefined;
  }

  /** Commands whose alias or title starts with / contains the query. */
  search(query: string, limit = 8): Command[] {
    const q = foldAlias(query);
    if (!q) return [];
    const scored: { cmd: Command; score: number }[] = [];
    for (const cmd of this.commands.values()) {
      let score = Infinity;
      for (const a of cmd.aliases ?? []) {
        const fa = foldAlias(a);
        if (fa === q) score = Math.min(score, 0);
        else if (fa.startsWith(q)) score = Math.min(score, 1 + fa.length / 100);
      }
      const ft = foldAlias(cmd.title);
      if (ft.startsWith(q)) score = Math.min(score, 2);
      else if (ft.includes(q)) score = Math.min(score, 3);
      if (score < Infinity) scored.push({ cmd, score });
    }
    return scored
      .sort((a, b) => a.score - b.score)
      .slice(0, limit)
      .map((s) => s.cmd);
  }
}
