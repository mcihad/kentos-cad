import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { registerCoreCommands } from '../../app/commands';
import type { AppContext } from '../../app/context';
import { PREFERENCE_DEFAULTS, type PreferencesData } from '../../app/state';
import { CommandRegistry } from '../../core/commands';
import { Signal } from '../../core/signal';
import { CadDocument } from '../../model/document';
import { LayerStore } from '../../model/layers';
import { screenScale, snapInRange } from '../../viewport/snapRange';
import type { MenuItem } from '../widgets/PopupMenu';
import { GRID_SPACINGS, snapMenu } from './snapMenu';

/**
 * The Kenet cell's menu and the snap kinds' commands (docs/adr/0163 §5–§6): the commands are the real ones,
 * registered by `registerCoreCommands` over the real preferences; the desktop's are in apps/desktop/src/snap_menu.rs.
 */

/** Anything else the commands touch while they are registered: reads give more of the same, calls give it back. */
function deep(): unknown {
  const stub: unknown = new Proxy(function () {}, {
    get: (_t, key) => (key === 'value' ? false : key === Symbol.toPrimitive ? () => '' : stub),
    apply: () => stub,
  });
  return stub;
}

function setup() {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: { x: 0, y: 0 } });
  const commands = new CommandRegistry();
  const prefs = Object.fromEntries(Object.entries(PREFERENCE_DEFAULTS).map(([k, v]) => [k, new Signal(v)])) as { [K in keyof PreferencesData]: Signal<PreferencesData[K]> };
  const keymap = { chordFor: () => undefined };
  const context: Record<string, unknown> = new Proxy({ commands, doc, prefs, keymap } as Record<string, unknown>, { get: (t, key: string) => (key in t ? t[key] : deep()) });
  const ctx = context as unknown as AppContext;
  registerCoreCommands(ctx, { openShortcuts() {}, openAbout() {}, openAppSettings() {}, openProjectSettings() {}, openNewProject() {}, focusCommandLine() {}, searchCommands() {}, keyTips() {}, openStart() {} });
  return { ctx, prefs, commands };
}

const labels = (items: MenuItem[]) => items.filter((i) => !i.kind || i.kind === 'item').map((i) => i.label);

beforeEach(() => {
  // The one command that listens to the page (Tam ekran) asks for `document` when it is registered.
  vi.stubGlobal('document', { fullscreenElement: null, fullscreenEnabled: true, addEventListener() {} });
});
afterEach(() => vi.unstubAllGlobals());

describe('Kenet hücresinin menüsü', () => {
  it('lists the twelve kinds by their short names, ticked as the preferences are, the four additions off', () => {
    const { ctx } = setup();
    const items = snapMenu(ctx);
    expect(items[0]).toMatchObject({ kind: 'header', label: 'Kenet türleri' });
    expect(labels(items).slice(0, 12)).toEqual([
      'Uç nokta',
      'Orta nokta',
      'Kesişim',
      'Merkez',
      'Dik',
      'Teğet',
      'Nokta',
      'En yakın',
      'Ağırlık merkezi',
      'Uzantı',
      'Paralel',
      'Karelaj',
    ]);
    const ticked = items.filter((i) => i.checked).map((i) => i.label);
    expect(ticked).toEqual(['Uç nokta', 'Orta nokta', 'Kesişim', 'Merkez', 'Dik', 'Teğet', 'Nokta', 'Çizilmekte olan nesneye']);
    // Each kind with its marker's icon beside the tick.
    expect(items.slice(1, 13).map((i) => i.icon)).toEqual([
      'snapEndpoint',
      'snapMidpoint',
      'snapIntersection',
      'snapCenter',
      'snapPerpendicular',
      'snapTangent',
      'snapNode',
      'snapNearest',
      'snapCentroid',
      'snapExtension',
      'snapParallel',
      'snapGrid',
    ]);
  });

  it('turns a kind and Çizilmekte olan nesneye on and off through their commands', () => {
    const { ctx, prefs, commands } = setup();
    const item = (label: string) => snapMenu(ctx).find((i) => i.label === label)!;
    item('Karelaj').run!();
    expect(prefs.snapGrid.value).toBe(true);
    expect(item('Karelaj').checked).toBe(true);
    item('Çizilmekte olan nesneye').run!();
    expect(prefs.snapSelf.value).toBe(false);
    // The commands by their ids and Turkish names, as the command line takes them.
    commands.execute('draft.snap.centroid');
    expect(prefs.snapCentroid.value).toBe(true);
    expect(commands.byAlias('KARELAJ')?.id).toBe('draft.snap.grid');
    expect(commands.byAlias('UZANTI')?.id).toBe('draft.snap.extension');
    expect(commands.byAlias('AGIRLIKMERKEZI')?.id).toBe('draft.snap.centroid');
    expect(commands.byAlias('PARALELKENET')?.id).toBe('draft.snap.parallel');
    // Named and described as the settings are.
    expect(commands.get('draft.snap.grid')).toMatchObject({ title: 'Kenet: Karelaj', short: 'Karelaj' });
    expect(commands.get('draft.snap.grid')?.description).toContain('karelaj aralığındaki');
  });

  it('offers Karelaj’s usual spacings: one sets both and turns Karelaj on', () => {
    const { ctx, prefs } = setup();
    const grid = () => snapMenu(ctx).find((i) => i.label === 'Karelaj aralığı')!;
    expect(grid().hint).toBe('1 m × 1 m');
    const spacings = grid().items as MenuItem[];
    expect(labels(spacings)).toEqual([...GRID_SPACINGS.map((v) => `${v} m`), 'Farklı aralık…']);
    expect(spacings.filter((i) => i.checked).map((i) => i.label)).toEqual(['1 m']);
    spacings.find((i) => i.label === '0.25 m')!.run!();
    expect([prefs.snapGridEast.value, prefs.snapGridNorth.value, prefs.snapGrid.value]).toEqual([0.25, 0.25, true]);
    expect(grid().hint).toBe('0.25 m × 0.25 m');
    // Different spacings east and north: none of the usual ones is chosen.
    prefs.snapGridNorth.set(0.5);
    expect((grid().items as MenuItem[]).filter((i) => i.checked)).toEqual([]);
  });
});

describe('Kenedin ölçek aralığı', () => {
  it('reads the scale as the status bar shows it and keeps to the range, 0 no limit', () => {
    // 0.125 m a pixel (the traces' view): 1:472.
    expect(screenScale(8)).toBe(472);
    expect(snapInRange(472, 0, 0)).toBe(true);
    expect(snapInRange(472, 500, 0)).toBe(false);
    expect(snapInRange(472, 472, 472)).toBe(true);
    expect(snapInRange(472, 0, 471)).toBe(false);
    expect(snapInRange(100_000, 0, 5000)).toBe(false);
    expect(snapInRange(100_000, 5000, 0)).toBe(true);
  });
});
