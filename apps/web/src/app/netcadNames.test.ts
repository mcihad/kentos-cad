import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CommandRegistry } from '../core/commands';
import { foldTurkish } from '../core/text';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { TOOL_CATALOG } from '../tools/catalog';
import { ToolManager } from '../tools/ToolManager';
import type { AppContext } from './context';
import { registerCoreCommands } from './commands';

/**
 * Netcad's command names (docs/adr/0141): typed in the command line, each starts the command it names.
 * The command line looks a typed word up with `CommandRegistry.byAlias` (ui/bottom/CommandLine.ts
 * `submit`); the registry here is the real one, filled by `registerCoreCommands` and the tool catalog
 * exactly as the app fills it. Aliases go through the same check as any: none may be claimed by two.
 */

/** Anything the commands touch while they are registered: reads give more of the same, calls give it back. */
function deep(): unknown {
  const stub: unknown = new Proxy(function () {}, {
    get: (_t, key) => (key === 'value' ? false : key === Symbol.toPrimitive ? () => '' : stub),
    apply: () => stub,
  });
  return stub;
}

function registry(): CommandRegistry {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: { x: 0, y: 0 } });
  const commands = new CommandRegistry();
  const context: Record<string, unknown> = new Proxy({ commands, doc } as Record<string, unknown>, { get: (t, key: string) => (key in t ? t[key] : deep()) });
  const ctx = context as unknown as AppContext;
  const tools = new ToolManager(ctx);
  for (const d of TOOL_CATALOG) tools.register(d);
  context.tools = tools;
  registerCoreCommands(ctx, { openShortcuts() {}, openAbout() {}, openAppSettings() {}, openProjectSettings() {}, openNewProject() {}, focusCommandLine() {}, searchCommands() {}, keyTips() {}, openStart() {} });
  return commands;
}

beforeEach(() => {
  // The one command that listens to the page (Tam ekran) asks for `document` when it is registered.
  vi.stubGlobal('document', { fullscreenElement: null, fullscreenEnabled: true, addEventListener() {} });
});
afterEach(() => vi.unstubAllGlobals());

describe('Netcad adları', () => {
  // [typed name, the command it starts]
  const names: [string, string][] = [
    ['ALANSOR', 'tool.area'],
    ['CETVEL', 'tool.measure'],
    ['XYZSOR', 'crs.query'],
    ['LIMITBUL', 'view.zoomExtents'],
    ['PENCEREBUYUT', 'tool.zoomWindow'],
    ['KUTU', 'tool.rectangle'],
    ['AYRISTIR', 'tool.explode'],
    ['KOSEYUVARLAT', 'tool.fillet'],
    ['COKLUDOGRU', 'tool.polyline'],
    ['BICIMBOYA', 'tool.matchProperties'],
    ['OBJEBOL', 'tool.split'],
    ['PRIZMA', 'tool.stationOffset'],
    ['ONCEKIPENCERE', 'view.previous'],
    ['ZP', 'view.previous'],
    ['ZN', 'view.next'],
    ['KAPSAM', 'view.extentCheck'],
    ['CITLESEC', 'tool.selectFence'],
    ['DAIRESEC', 'tool.selectCircle'],
    ['ICEREN', 'tool.selectContaining'],
  ];

  it.each(names)('%s starts %s', (typed, id) => {
    const commands = registry();
    expect(commands.byAlias(typed)?.id).toBe(id);
  });

  it('is found however it is typed: lower case, with the Turkish letters', () => {
    const commands = registry();
    expect(commands.byAlias('prizma')?.id).toBe('tool.stationOffset');
    expect(commands.byAlias('Çitlesec')?.id).toBe('tool.selectFence');
    expect(commands.byAlias('köşeyuvarlat')?.id).toBe('tool.fillet');
    expect(commands.byAlias('ÇOKLUDOĞRU')?.id).toBe('tool.polyline');
    expect(commands.byAlias('ıcerEn')?.id).toBe('tool.selectContaining');
  });

  it('every command they start is there, and is not the stand-in of an unbuilt tool', () => {
    const commands = registry();
    for (const [, id] of names) {
      const c = commands.get(id);
      expect(c, id).toBeDefined();
      expect(c!.pending, id).toBeFalsy();
    }
  });

  it('no one of them is claimed by a second command', () => {
    const commands = registry();
    const claimants = (typed: string) => commands.all().filter((c) => (c.aliases ?? []).some((a) => foldTurkish(a) === foldTurkish(typed))).map((c) => c.id);
    for (const [typed, id] of names) expect(claimants(typed), typed).toEqual([id]);
  });
});
