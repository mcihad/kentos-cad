import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { registerCoreCommands } from '../../app/commands';
import type { AppContext } from '../../app/context';
import { PREFERENCE_DEFAULTS, type PreferencesData } from '../../app/state';
import { CommandRegistry } from '../../core/commands';
import { Signal } from '../../core/signal';
import { CadDocument } from '../../model/document';
import { LayerStore } from '../../model/layers';
import type { MenuItem } from '../widgets/PopupMenu';
import { layerSnapItems, snapState, toggledSnap } from './layerSnap';

/**
 * A layer's own snapping in the Katmanlar panel (docs/adr/0163 §4): the magnet's states and click, and Kenet ▸ over the
 * real commands and preferences. The desktop's are in apps/desktop/src/layer_snap.rs.
 */

function deep(): unknown {
  const stub: unknown = new Proxy(function () {}, {
    get: (_t, key) => (key === 'value' ? false : key === Symbol.toPrimitive ? () => '' : stub),
    apply: () => stub,
  });
  return stub;
}

function setup() {
  const layers = new LayerStore(
    [
      { id: 'g', name: 'Yapılar', type: 'group', children: [{ id: 'a', name: 'Bina' }, { id: 'b', name: 'Duvar' }] },
      { id: 'c', name: 'Yol' },
    ],
    'a',
  );
  const doc = new CadDocument({ name: 'Deneme', layers, origin: { x: 0, y: 0 } });
  const commands = new CommandRegistry();
  const prefs = Object.fromEntries(Object.entries(PREFERENCE_DEFAULTS).map(([k, v]) => [k, new Signal(v)])) as { [K in keyof PreferencesData]: Signal<PreferencesData[K]> };
  const context: Record<string, unknown> = new Proxy({ commands, doc, prefs, keymap: { chordFor: () => undefined } } as Record<string, unknown>, {
    get: (t, key: string) => (key in t ? t[key] : deep()),
  });
  const ctx = context as unknown as AppContext;
  registerCoreCommands(ctx, { openShortcuts() {}, openAbout() {}, openAppSettings() {}, openProjectSettings() {}, openNewProject() {}, focusCommandLine() {}, searchCommands() {}, keyTips() {}, openStart() {} });
  const node = (id: string) => doc.layers.get(id)!;
  const item = (items: MenuItem[], label: string) => items.find((i) => i.label === label)!;
  return { ctx, layers: doc.layers, node, item };
}

beforeEach(() => vi.stubGlobal('document', { fullscreenElement: null, fullscreenEnabled: true, addEventListener() {} }));
afterEach(() => vi.unstubAllGlobals());

describe('Katmanın keneti', () => {
  it('shows a layer’s state on its magnet and a group’s from its layers', () => {
    const { layers, node } = setup();
    expect(snapState(layers, node('a'))).toBe('none');
    layers.setSnap('a', { off: true });
    expect(snapState(layers, node('a'))).toBe('off');
    expect(snapState(layers, node('g')), 'one off, one general').toBe('kinds');
    layers.setSnap('b', { off: true });
    expect(snapState(layers, node('g')), 'all off').toBe('off');
    layers.setSnap('b', { kinds: ['node'] });
    expect(snapState(layers, node('b'))).toBe('kinds');
  });

  it('turns off with a click, and back to the general kinds; a group’s writes to all its layers', () => {
    const { layers, node } = setup();
    layers.setSnap('g', toggledSnap(layers, node('g')));
    expect([node('a').snap, node('b').snap, node('g').snap, node('c').snap]).toEqual([{ off: true }, { off: true }, undefined, undefined]);
    layers.setSnap('g', toggledSnap(layers, node('g')));
    expect([node('a').snap, node('b').snap]).toEqual([undefined, undefined]);
    layers.setSnap('c', { kinds: ['endpoint'] });
    expect(toggledSnap(layers, node('c')), 'own kinds turn off too').toEqual({ off: true });
  });

  it('ticks a kind from what the layer takes now: the general kinds, or none when off', () => {
    const { ctx, layers, node, item } = setup();
    const menu = () => layerSnapItems(ctx, node('a'));
    expect(item(menu(), 'Genel türler').checked).toBe(true);
    // The general kinds as the settings start them: all but nearest and the four additions.
    expect(menu().filter((i) => i.checked && i.label !== 'Genel türler').map((i) => i.label)).toEqual(['Uç nokta', 'Orta nokta', 'Kesişim', 'Merkez', 'Dik', 'Teğet', 'Nokta']);
    item(menu(), 'Orta nokta').run!();
    expect(node('a').snap).toEqual({ kinds: ['endpoint', 'intersection', 'center', 'perpendicular', 'tangent', 'node'] });
    item(menu(), 'Kapalı').run!();
    expect(node('a').snap).toEqual({ off: true });
    item(menu(), 'Karelaj').run!();
    expect(node('a').snap, 'off: the ticked kind alone').toEqual({ kinds: ['grid'] });
    item(menu(), 'Karelaj').run!();
    expect(node('a').snap, 'the last kind unticked: off').toEqual({ off: true });
    item(menu(), 'Genel türler').run!();
    expect(node('a').snap).toBeUndefined();
    expect(layers.get('g')?.snap).toBeUndefined();
  });
});
