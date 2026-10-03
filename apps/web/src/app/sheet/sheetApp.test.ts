import { describe, expect, it } from 'vitest';
import { testEngine } from '../../product/sheet/engineTesting';
import { copyItems, withChildren } from '../../product/sheet/ops';
import { BookStore, MemoryKeyValue } from '../../product/sheet/store';
import type { AppContext } from '../context';
import { registerSheetCommands } from './commands';
import { sheetToolCommands } from './toolCommands';
import { kpaftaOf, renewIds } from './exporting';
import { opensModel } from './install';
import { keyAllowedInSheet } from './keys';
import { keyChange, projectKeyOf } from './projectKey';
import { sheetRibbonTab, sheetTabCommands } from './ribbonTab';
import { SheetService } from './service';
import { fakeApp } from './sheetAppTesting';
import { newSheet, sheetFromTemplate } from './templateActions';

/**
 * The sheet layouts in the app (docs/sheet/design.md §10, §11, §11a): the key
 * a project's book is kept under and how it follows the drawing; the Pafta
 * tab built from the engine's profile of the mode and every command it
 * names; a sheet made from the mode's default template, edited, undone and
 * redone through the engine; a refusal said with the engine's code; the
 * book written to the device and read back by the engine; `.kpafta` there
 * and back; which keys stay the app's while a sheet is in front.
 */

const engine = testEngine();
const loaded = () => Promise.resolve(engine);
/** The template's questions answered with its own values (the window is the browser's). */
const noQuestions = async () => [];

function setup() {
  const { ctx, app } = fakeApp();
  const kv = new MemoryKeyValue();
  const sheets = new SheetService(app, kv, loaded);
  const asked: string[] = [];
  registerSheetCommands(app, sheets, { gallery: () => asked.push('gallery'), stage: () => null, window: (n) => asked.push(n), ask: async () => 'Yeni ad' });
  return { ctx, sheets, kv, asked };
}

describe('project key', () => {
  it('names a cloud project, then a drawing’s lasting id, then its file, else the session', () => {
    const base = { cloud: null, projectId: null, fileName: null, session: 's1' };
    expect(projectKeyOf({ ...base, cloud: { tenantId: 't', projectId: 'p', name: 'Ada' }, projectId: 'x', fileName: 'a.kcad' })).toMatchObject({ id: 'bulut/t/p', lasting: true });
    expect(projectKeyOf({ ...base, projectId: 'x', fileName: 'a.kcad' })).toMatchObject({ id: 'proje/x', lasting: true });
    expect(projectKeyOf({ ...base, fileName: 'a.kcad' })).toMatchObject({ id: 'dosya/a.kcad', lasting: true });
    expect(projectKeyOf(base)).toMatchObject({ id: 'oturum/s1', lasting: false });
  });

  it('loads another drawing’s book, moves a session’s along, copies a lasting name’s', () => {
    const session = projectKeyOf({ cloud: null, projectId: null, fileName: null, session: 's' });
    const file = projectKeyOf({ cloud: null, projectId: null, fileName: 'a.kcad', session: 's' });
    const cloud = projectKeyOf({ cloud: { tenantId: 't', projectId: 'p', name: 'P' }, projectId: null, fileName: null, session: 's' });
    expect(keyChange(null, file, false)).toBe('load');
    expect(keyChange(file, file, false)).toBe('keep');
    expect(keyChange(file, file, true)).toBe('load');
    expect(keyChange(session, file, false)).toBe('move');
    expect(keyChange(file, cloud, false)).toBe('copy');
    expect(keyChange(file, cloud, true)).toBe('load');
  });
});

describe('Pafta tab from the mode’s profile', () => {
  it('has its own panels around the profile’s groups, the mode’s tools under Ekle', () => {
    const tab = sheetRibbonTab(engine.profileFor('cad', { georeferenced: true, attributeLayers: true, plotScale: 1000 }));
    expect(tab).toMatchObject({ id: 'sheet', label: 'Pafta', contextual: 'sheet' });
    expect(tab.panels.map((p) => p.label)).toEqual(['Pafta', 'Araçlar', 'Ekle', 'Harita', 'Düzen', 'Görünüm', 'Çıktı']);
    const ekle = tab.panels.find((p) => p.label === 'Ekle')!;
    const names = ekle.items.map((i) => (i.kind === 'command' ? i.id : i.kind === 'menu' ? `${i.menu.label}▾` : ''));
    expect(names.slice(0, 3)).toEqual(['sheet.add.map', 'sheet.add.text', 'sheet.add.legend']);
    expect(names).toContain('Şekil▾');
    expect(names).toContain('Pafta çerçevesi▾');
    // CBS's tools are not CAD's.
    expect(names).not.toContain('sheet.add.attributeTable');
    for (const p of tab.panels) expect(p.items.filter((i) => i.kind !== 'builtin' && i.size !== 'small').length, p.label).toBeLessThanOrEqual(4);
    // A mode without a coordinate system hides what needs one (§11a): no north arrow, no grid.
    const plain = sheetRibbonTab(engine.profileFor('cad', { georeferenced: false, attributeLayers: false }));
    expect(sheetTabCommands(plain)).not.toContain('sheet.add.northArrow');
    expect(sheetTabCommands(plain)).not.toContain('sheet.grid');
  });

  it('registers every command the tab names (the mode’s tools with their ready looks), each with an icon and a title', async () => {
    const { ctx, sheets } = setup();
    await sheets.ensureEngine();
    sheetToolCommands(ctx as unknown as AppContext, sheets, sheets.tools.value);
    for (const id of sheetTabCommands(sheetRibbonTab(sheets.profile.value))) {
      const c = ctx.commands.get(id);
      expect(c, id).toBeTruthy();
      expect(c!.icon, id).toBeTruthy();
      expect(c!.title.length, id).toBeGreaterThan(1);
    }
    // A tool's command takes it in hand once a sheet is in front.
    expect(ctx.commands.get('sheet.add.map')!.whyDisabled!()).toContain('Önce bir pafta açın');
    await newSheet(sheets, noQuestions);
    ctx.commands.execute('sheet.add.map');
    expect(sheets.state.tool.value).toMatchObject({ kind: 'add', tool: 'map', item: 'map' });
  });
});

describe('a sheet through the engine', () => {
  it('makes one from the mode’s default template, opens it, moves an item and takes it back', async () => {
    const { ctx, sheets } = setup();
    expect(sheets.state.engine.value.state).toBe('idle');
    const id = await newSheet(sheets, noQuestions);
    expect(id).toBeTruthy();
    expect(sheets.state.engine.value.state).toBe('ready');
    expect(sheets.state.open.value).toBe(id);
    expect(sheets.profile.value.id).toBe('cad');
    // CAD's default template, with the drawing area's centre for its map.
    const sheet = sheets.book()!.book.sheets[0];
    expect(sheet.origin?.templateId).toBe(sheets.profile.value.defaultTemplate);
    const map = sheet.items.find((i) => i.kind.type === 'map')!;
    expect(map.kind.type === 'map' && map.kind.view.type === 'fixed' && map.kind.view.center).toEqual({ x: 486_780, y: 4_420_080 });
    // A nudge is the engine's step.
    sheets.state.select([map.id]);
    expect(ctx.commands.execute('sheet.nudge', { dx: 1, dy: 0, step: 'shift' })).toBe(true);
    const moved = sheets.book()!.book.sheets[0].items.find((i) => i.id === map.id)!;
    expect(moved.frame.left).toBe(map.frame.left + engine.info.nudge[1]);
    expect(ctx.commands.isEnabled('sheet.undo')).toBe(true);
    ctx.commands.execute('sheet.undo');
    expect(sheets.book()!.book.sheets[0].items.find((i) => i.id === map.id)!.frame.left).toBe(map.frame.left);
    ctx.commands.execute('sheet.redo');
    expect(sheets.book()!.book.sheets[0].items.find((i) => i.id === map.id)!.frame.left).toBe(moved.frame.left);
    expect(ctx.log.entries.value.some((e) => e.text.includes('Geri alındı (pafta)'))).toBe(true);
  });

  it('says a refusal with the engine’s words and code, and leaves the book as it was', async () => {
    const { ctx, sheets } = setup();
    await newSheet(sheets, noQuestions);
    const before = sheets.book();
    expect(sheets.apply([{ op: 'renameItem', id: 'yok', name: 'x' }], 'Ad ver')).toBe(false);
    const last = ctx.log.entries.value.at(-1)!;
    expect(last.level).toBe('error');
    expect(last.text).toMatch(/^Ad ver uygulanmadı: .+ \([a-z_]+\)$/);
    expect(sheets.book()).toBe(before);
  });

  it('groups, duplicates and deletes the choice, each one step', async () => {
    const { ctx, sheets } = setup();
    await sheetFromTemplate(sheets, engine.systemTemplates().find((t) => t.meta.id === 'sys:genel-a4-dikey')!, null);
    const items = sheets.book()!.book.sheets[0].items.filter((i) => !i.locked);
    sheets.state.select(items.slice(0, 2).map((i) => i.id));
    expect(ctx.commands.execute('sheet.group')).toBe(true);
    const group = [...sheets.state.selection.value][0];
    expect(sheets.book()!.book.sheets[0].items.find((i) => i.id === group)?.kind.type).toBe('group');
    const count = sheets.book()!.book.sheets[0].items.length;
    ctx.commands.execute('sheet.duplicateItems');
    expect(sheets.book()!.book.sheets[0].items.length).toBe(count + 3);
    ctx.commands.execute('sheet.deleteItems');
    expect(sheets.book()!.book.sheets[0].items.length).toBe(count);
    ctx.commands.execute('sheet.undo');
    expect(sheets.book()!.book.sheets[0].items.length).toBe(count + 3);
  });

  it('makes a master page of chosen items (one step) that the sheet then draws under its own, and lets it go again', async () => {
    const { sheets } = setup();
    await sheetFromTemplate(sheets, engine.systemTemplates().find((t) => t.meta.id === 'sys:ifraz-paftasi')!, null);
    const sheet = sheets.book()!.book.sheets[0];
    const chosen = sheet.items.filter((i) => i.kind.type === 'titleBlock').map((i) => i.id);
    const id = 'ana';
    const items = copyItems(sheet, chosen, () => sheets.newId(), 0);
    expect(sheets.apply([{ op: 'addMaster', master: { id, name: 'Ana sayfa 1', page: sheet.page, items, guides: [], variants: [] } }, { op: 'removeItems', ids: withChildren(sheet, chosen) }, { op: 'setSheetMaster', sheet: sheet.id, master: id }], 'Ana sayfa yap')).toBe(true);
    const list = sheets.plan(sheets.book()!, sheet.id)!;
    expect(list.masterItems.sort()).toEqual(items.map((i) => i.id).sort());
    expect(sheets.state.book.value.sheets[0].items.filter((i) => i.master).length).toBe(items.length);
    // Ana sayfadan ayır: the master's items come back onto the sheet.
    const master = sheets.book()!.book.masters[0];
    expect(sheets.apply([{ op: 'detachMaster', sheet: sheet.id, itemIds: master.items.map(() => sheets.newId()) }], 'Ana sayfadan ayır')).toBe(true);
    expect(sheets.book()!.book.sheets[0].master).toBeUndefined();
    sheets.undo();
    sheets.undo();
    expect(engine.bookDigest(sheets.book()!)).not.toBe('');
    expect(sheets.book()!.book.sheets[0].items.length).toBe(sheet.items.length);
  });

  it('writes the book to this device and reads it back through the engine when the project opens again', async () => {
    const { sheets, kv } = setup();
    const stop = sheets.watchProject();
    await newSheet(sheets, noQuestions);
    sheets.flush();
    await new Promise((r) => setTimeout(r, 0));
    const key = sheets.key.value!.id;
    const stored = await new BookStore(kv).load(key);
    expect(stored.status).toBe('ok');
    stop();
    // Another session of the app on the same device: the tab shows the name before the engine is fetched.
    const again = fakeApp();
    const other = new SheetService(again.app, kv, loaded);
    (other.projectBooks as unknown as { session: string }).session = key.slice('oturum/'.length);
    const stop2 = other.watchProject();
    await new Promise((r) => setTimeout(r, 0));
    expect(other.state.waiting.value.map((w) => w.name)).toEqual([sheets.book()!.book.sheets[0].name]);
    expect(other.state.engine.value.state).toBe('idle');
    other.openSheet(other.state.waiting.value[0].id);
    await new Promise((r) => setTimeout(r, 0));
    expect(other.state.open.value).toBe(sheets.book()!.book.sheets[0].id);
    expect(engine.bookDigest(other.book()!)).toBe(engine.bookDigest(sheets.book()!));
    stop2();
  });

  it('carries the book in a .kpafta file (the core’s codec) and reads it back with new ids beside a project’s own', async () => {
    const { sheets } = setup();
    await newSheet(sheets, noQuestions);
    const text = await kpaftaOf(sheets, sheets.book()!.book);
    expect(JSON.parse(text)).toMatchObject({ format: 'kentos.sheet.file', version: 1 });
    const read = engine.decodeKpafta(text);
    expect(engine.bookDigest(engine.readBook(JSON.stringify(read.book)))).toBe(engine.bookDigest(sheets.book()!));
    let n = 0;
    const renewed = renewIds(read.book, () => `yeni-${n++}`);
    expect(renewed.sheets[0].id).toMatch(/^yeni-/);
    expect(renewed.sheets[0].items.every((i) => i.id.startsWith('yeni-'))).toBe(true);
    expect(() => engine.decodeKpafta('{"format":"başka"}')).toThrow();
  });
});

describe('keys while a sheet is in front', () => {
  it('lets the app’s own keys through and holds the drawing’s', () => {
    for (const id of ['file.save', 'help.shortcuts', 'view.commandSearch', 'tools.options', 'sheet.undo', 'cloud.open']) expect(keyAllowedInSheet(id), id).toBe(true);
    for (const id of ['tool.line', 'edit.undo', 'draft.snap', 'view.zoomExtents', 'view.rightPanel', 'view.bottomPanel']) expect(keyAllowedInSheet(id), id).toBe(false);
  });

  it('brings Model forward for a drawing command run from the ribbon or the command line (not for Geri al: the sheet takes it)', () => {
    for (const id of ['tool.line', 'edit.copy', 'draft.snap', 'view.zoomExtents']) expect(opensModel(id), id).toBe(true);
    for (const id of ['tool.cancel', 'tool.confirm', 'edit.undo', 'edit.redo', 'file.save', 'sheet.zoomPage', 'view.ribbonCollapse']) expect(opensModel(id), id).toBe(false);
  });
});
