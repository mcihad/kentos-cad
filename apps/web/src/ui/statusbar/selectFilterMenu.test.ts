import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { registerCoreCommands } from '../../app/commands';
import type { AppContext } from '../../app/context';
import { menuById, resolveMenu, type SubmenuSpec } from '../../app/menus';
import { PREFERENCE_DEFAULTS, type PreferencesData } from '../../app/state';
import { CommandRegistry } from '../../core/commands';
import { Signal } from '../../core/signal';
import { CadDocument } from '../../model/document';
import type { EntityKind } from '../../model/entities';
import { LayerStore } from '../../model/layers';
import { FILTER_KINDS } from '../../tools/selectable';
import type { MenuItem } from '../widgets/PopupMenu';
import { selectFilterMenu } from './selectFilterMenu';

/**
 * Seçim süzgeci's lists (docs/adr/0187 §5, 8 Ekim): the Süzgeç cell's right-click menu and Giriş › Seçim süzgeci ▾,
 * the same checklist — the kinds by their short names and icons, every row leaving the menu open, then every kind or
 * none at once. The commands are the real ones, registered by `registerCoreCommands`; the desktop's list is
 * `select_filter_menu` in apps/desktop/src/selection_commands.rs.
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
  const settings = { selectFilter: new Signal(false), selectKinds: new Signal<ReadonlySet<EntityKind>>(new Set(FILTER_KINDS)) };
  const keymap = { chordFor: () => undefined };
  const tools = { list: () => [] };
  const context: Record<string, unknown> = new Proxy({ commands, doc, prefs, keymap, settings, tools } as Record<string, unknown>, { get: (t, key: string) => (key in t ? t[key] : deep()) });
  const ctx = context as unknown as AppContext;
  registerCoreCommands(ctx, { openShortcuts() {}, openAbout() {}, openAppSettings() {}, openProjectSettings() {}, openNewProject() {}, focusCommandLine() {}, searchCommands() {}, keyTips() {}, openStart() {} });
  return { ctx, settings };
}

const rows = (items: MenuItem[]) => items.filter((i) => !i.kind || i.kind === 'item');
const row = (items: MenuItem[], label: string) => rows(items).find((i) => i.label === label)!;
const kinds = ['Nokta', 'Çizgi', 'Çoklu çizgi', 'Kapalı alan', 'Daire', 'Yay', 'Elips', 'Eğri', 'Yardımcı çizgi', 'Işın', 'Yazı', 'Ölçü', 'Tarama', 'Blok', 'Kılavuz', 'Tablo', 'Resim'];
const icons = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'xline', 'ray', 'text', 'dimension', 'hatch', 'blockInsert', 'leader', 'table', 'imageInsert'];

beforeEach(() => {
  // The one command that listens to the page (Tam ekran) asks for `document` when it is registered.
  vi.stubGlobal('document', { fullscreenElement: null, fullscreenEnabled: true, addEventListener() {} });
});
afterEach(() => vi.unstubAllGlobals());

describe('Süzgeç hücresinin menüsü', () => {
  it('lists the seventeen kinds by their short names and icons, each ticked, then every kind or none', () => {
    const { ctx } = setup();
    const items = selectFilterMenu(ctx);
    expect(items[0]).toEqual({ kind: 'header', label: 'Seçilebilir türler' });
    expect(rows(items).map((i) => i.label)).toEqual([...kinds, 'Bütün türler', 'Hiçbir tür']);
    expect(rows(items).map((i) => i.icon)).toEqual([...icons, 'selectAll', 'deselect']);
    // The kinds are ticked (every kind at first), the bulk rows are actions; a separator between them.
    expect(rows(items).map((i) => i.checked)).toEqual([...kinds.map(() => true), undefined, undefined]);
    expect(items[kinds.length + 1]).toEqual({ kind: 'separator' });
    // Every row leaves the menu open: several kinds are ticked from one opening.
    expect(rows(items).every((i) => i.stay)).toBe(true);
  });

  it('Hiçbir tür, then two ticks: only those kinds, the filter on, the rows read again with their new ticks', () => {
    const { ctx, settings } = setup();
    row(selectFilterMenu(ctx), 'Hiçbir tür').run!();
    expect(settings.selectFilter.value).toBe(true);
    expect(settings.selectKinds.value.size).toBe(0);
    expect(rows(selectFilterMenu(ctx)).filter((i) => i.checked).map((i) => i.label)).toEqual([]);
    row(selectFilterMenu(ctx), 'Yazı').run!();
    row(selectFilterMenu(ctx), 'Ölçü').run!();
    expect([...settings.selectKinds.value]).toEqual(['text', 'dimension']);
    expect(rows(selectFilterMenu(ctx)).filter((i) => i.checked).map((i) => i.label)).toEqual(['Yazı', 'Ölçü']);
    row(selectFilterMenu(ctx), 'Bütün türler').run!();
    expect(settings.selectKinds.value.size).toBe(FILTER_KINDS.length);
    expect(FILTER_KINDS).toContain('image');
  });
});

describe('Giriş › Seçim süzgeci ▾', () => {
  it('is the same checklist under its on/off row, which says the cell’s word', () => {
    const { ctx, settings } = setup();
    const spec = menuById('edit')!.items.find((s): s is SubmenuSpec => typeof s === 'object' && 'label' in s && s.label === 'Seçim süzgeci')!;
    expect(spec.checklist).toBe(true);
    const items = resolveMenu(ctx, spec.items, { checklist: spec.checklist });
    expect(items[0]).toMatchObject({ label: 'Süzgeç', icon: 'selectFilter', checked: false, stay: true });
    // Then a separator and the cell's own list, row for row.
    expect(items[1]).toEqual({ kind: 'separator' });
    const cell = selectFilterMenu(ctx);
    expect(items.slice(2).map(({ run: _, ...rest }) => rest)).toEqual(cell.map(({ run: _, ...rest }) => rest));
    items[0].run!();
    expect(settings.selectFilter.value).toBe(true);
    expect(resolveMenu(ctx, spec.items, { checklist: true })[0].checked).toBe(true);
  });

  it('a plain drop-down is unchanged: the long title, a toggle’s tick without its icon, closing as it runs', () => {
    const { ctx } = setup();
    const [first] = resolveMenu(ctx, ['edit.selectFilter.text']);
    expect(first).toMatchObject({ label: 'Seçim süzgecinde Yazı', icon: undefined, checked: true });
    expect(first.stay).toBeUndefined();
  });
});
