import { describe, expect, it } from 'vitest';
import type { Template } from '../../contracts/generated/sheet/Template';
import { bookView } from '../../product/sheet/adapter';
import { bookText } from '../../product/sheet/engine';
import { templateBook, testEngine } from '../../product/sheet/engineTesting';
import { groupItem } from '../../product/sheet/ops';
import { ENGINE_TEXTS } from '../../product/sheet/state';
import { cardOf } from '../../product/sheet/templates';
import type { ItemView, SheetView } from '../../product/sheet/view';
import { actionsOf, emptyText, paperChoices, SHARE_NEEDS_SYNC, sectionNote } from './galleryPlan';
import { bindablesFor, bindingsOf, common, frameValues, notesOf, sameGroup, summaryOf } from './inspectorPlan';
import { tabsNote } from './tabsPlan';
import { dropOf, itemNodes, orderAfterDrop } from './treePlan';
import { anchorsSentence, clickPin, commonAnchors, pinState } from './widgets/anchors';

/**
 * The sheet interface's decisions apart from the DOM (docs/sheet/design.md
 * §3.2, §11, §12, §13), on the engine's own sheet (its ifraz template, two
 * items grouped by the engine): the constraint editor's pins, what the
 * inspector shows for one item and several (“—” where they differ), the
 * item tree's shape, drops and the order a drop makes, what the gallery
 * offers per template and says when a source has nothing, and the tabs'
 * note.
 */

const e = testEngine();
const made = templateBook('sys:ifraz-paftasi', 'i');
const grouped = e.applyOp(made.book, { op: 'group', ids: ['i-4', 'i-5'], group: groupItem('g', 'Kuzey ve ölçek') });
const profile = e.profileFor('cad', { georeferenced: true, attributeLayers: true, plotScale: 1000 });
const view = (open: string | null): SheetView => bookView(grouped.book, { papers: e.paperSizes(), template: () => null, note: (i) => e.itemNote('cad', i), open }).sheets[0];
const sheet = view(made.sheet);
const byId = (id: string) => sheet.items.find((i) => i.id === id)!;

describe('constraint editor', () => {
  it('takes a side alone on a click, keeps both with Shift, and never clicks an axis off', () => {
    expect(clickPin('start', 'end', false)).toBe('end');
    expect(clickPin('start', 'end', true)).toBe('both');
    expect(clickPin('both', 'start', true)).toBe('end');
    expect(clickPin('both', 'start', false)).toBe('start');
    expect(clickPin('scale', 'center', false)).toBe('center');
    expect(clickPin(null, 'end', true)).toBe('end');
    expect(clickPin('start', 'start', true)).toBe('start');
  });

  it('shows a pin on, off, or dashed when only some of the chosen items keep it', () => {
    expect(pinState(['start', 'both'], 'start')).toBe(true);
    expect(pinState(['start', 'end'], 'start')).toBe('mixed');
    expect(pinState(['center'], 'start')).toBe(false);
    expect(pinState(['both'], 'center')).toBe(false);
  });

  it('shares the anchors several items have, and says what they do', () => {
    // The map stretches both ways; the title block keeps right and top.
    const c = commonAnchors([byId('i-1').anchors, byId('i-3').anchors]);
    expect(c).toEqual({ h: null, v: null, box: 'margins' });
    expect(commonAnchors([byId('i-3').anchors, byId('i-6').anchors])).toEqual({ h: 'right', v: 'top', box: 'margins' });
    expect(anchorsSentence(null, 'top')).toContain('farklı');
    expect(anchorsSentence('right', 'bottom')).toBe('Sağ ve alt kenara bağlı kalır; kâğıt değişince oradan taşınır.');
    expect(anchorsSentence('leftRight', 'topBottom')).toContain('Dört kenara');
  });
});

describe('inspector', () => {
  it('shows one item’s values and, for several, what they share (null: “—”)', () => {
    const strip = [byId('i-3'), byId('i-6'), byId('i-8')];
    expect(frameValues(strip)).toEqual({ left: 302, top: null, width: 105, height: null, rotation: 0 });
    // A micrometre apart is the same value.
    const near = [byId('i-3'), { ...byId('i-3'), frame: { ...byId('i-3').frame, left: 302.0002 } } as ItemView];
    expect(frameValues(near).left).toBe(302);
    expect(common([], (i) => i.name)).toBeNull();
  });

  it('names one item by its kind in the mode (and its name when that says more), several by count and kinds', () => {
    expect(summaryOf([byId('i-1')], profile)).toEqual({ icon: 'sheetMap', title: 'Görünüm penceresi', sub: 'Harita · 1/1000' });
    const s = summaryOf([byId('i-3'), byId('i-8'), byId('i-7')], profile);
    expect(s.title).toBe('3 öğe seçili');
    expect(s.sub).toBe('2 antet, 1 tablo');
  });

  it('names the bound properties as the engine does, in its order', () => {
    const b = e.bindableProperties();
    const item = { ...byId('i-1'), bindings: { 'x.y': '1', 'map.scale': '@olcek_payda', 'frame.left': '2' } };
    expect(bindingsOf(item, b)).toEqual([
      { path: 'frame.left', label: 'Sol', unit: 'mm', expression: '2' },
      { path: 'map.scale', label: 'Ölçek', unit: '', expression: '@olcek_payda' },
      { path: 'x.y', label: 'x.y', unit: '', expression: '1' },
    ]);
    expect(bindablesFor([byId('i-1')], b).some((x) => x.property === 'map.scale')).toBe(true);
    expect(bindablesFor([byId('i-1'), byId('i-3')], b).some((x) => x.property === 'map.scale')).toBe(false);
  });

  it('knows a group’s parts, and says the engine’s note for another mode’s item once', () => {
    expect(sameGroup([byId('i-4'), byId('i-5')])).toBe(true);
    expect(sameGroup([byId('i-4'), byId('i-3')])).toBe(false);
    const notes = notesOf([byId('i-7'), byId('i-3'), byId('i-7')]);
    expect(notes).toHaveLength(1);
    expect(notes[0]).toContain('CBS kipinin aracıdır');
  });
});

describe('item tree', () => {
  it('lists the front first and a group’s items under it', () => {
    const nodes = itemNodes(sheet);
    expect(nodes[0].id).toBe('i-8');
    const group = nodes.find((n) => n.id === 'g')!;
    expect(group.children.map((n) => n.id)).toEqual(['i-5', 'i-4']);
    expect(nodes.some((n) => n.id === 'i-4')).toBe(false);
  });

  it('drops in front of a row or behind it, among the item’s siblings only', () => {
    expect(dropOf(sheet, ['i-1'], 'i-3', 0.2)).toEqual({ zone: 'before', before: 'g', group: null });
    expect(dropOf(sheet, ['i-1'], 'i-3', 0.8)).toEqual({ zone: 'after', before: 'i-3', group: null });
    expect(dropOf(sheet, ['i-1'], 'i-8', 0.1)).toEqual({ zone: 'before', before: null, group: null });
    expect(dropOf(sheet, ['i-4'], 'i-5', 0.9)).toEqual({ zone: 'after', before: 'i-5', group: 'g' });
    expect(dropOf(sheet, ['i-1'], 'i-1', 0.5)).toBeNull();
    // Into or out of a group is Grupla's and Grubu çöz's.
    expect(dropOf(sheet, ['i-1'], 'i-4', 0.5)).toBeNull();
    expect(dropOf(sheet, ['i-4'], 'i-3', 0.5)).toBeNull();
  });

  it('makes the whole drawing order of a drop, which the engine takes', () => {
    const drop = dropOf(sheet, ['i-1'], 'i-8', 0.1)!;
    const order = orderAfterDrop(sheet, ['i-1'], drop);
    expect(order.at(-1)).toBe('i-1');
    expect([...order].sort()).toEqual(sheet.items.map((i) => i.id).sort());
    const a = e.applyOp(bookText(grouped.book), { op: 'setOrder', owner: { kind: 'sheet', id: made.sheet }, order });
    expect(a.book.sheets[0].items.at(-1)!.id).toBe('i-1');
    // On top of a group: just under the group item, which stands above its children.
    const inGroup = orderAfterDrop(sheet, ['i-4'], dropOf(sheet, ['i-4'], 'i-5', 0.1)!);
    expect(inGroup.indexOf('i-4')).toBe(inGroup.indexOf('g') - 1);
  });
});

describe('template gallery offers', () => {
  const ready = { engine: null, cloud: null, account: null };
  const away = { engine: ENGINE_TEXTS.failed('ağ'), cloud: 'bulut yok', account: 'giriş yok' };
  const offline = { engine: null, cloud: 'sunucu yok', account: null };
  const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi')!;
  const system = cardOf(t, 'system', ['system']);
  const device = cardOf({ ...t, meta: { ...t.meta, id: 'dev' } } as Template, 'device', ['device', 'unsynced']);
  const cloud = { ...device, source: 'cloud' as const, role: 'owner' as const };
  const shared = { ...device, source: 'shared' as const, role: 'viewer' as const };
  const editing = { ...shared, role: 'editor' as const };
  const name = (p: string) => e.paperSizes().find((x) => x.id === p)?.name ?? p;

  it('keeps a system template read-only: used, copied, its copy edited', () => {
    const a = actionsOf(system, ready);
    expect(Object.entries(a).filter(([, v]) => v.shown).map(([k]) => k)).toEqual(['use', 'duplicate', 'edit']);
    expect(a.edit.label).toBe('Kopyasını düzenle');
    expect(a.duplicate.label).toBe('Şablonlarıma kopyala');
  });

  it('cannot share a template only on this device before it is synced, and says so', () => {
    const a = actionsOf(device, ready);
    expect(a.share).toEqual({ shown: true, label: 'Paylaş…', reason: SHARE_NEEDS_SYNC });
    expect(a.delete.reason).toBeNull();
    expect(a.sync.label).toBe('Buluta eşitle');
    expect(actionsOf(cloud, ready).share.reason).toBeNull();
  });

  it('lets the owner decide on a shared one, and the engine on anything it does', () => {
    const a = actionsOf(shared, ready);
    expect(a.share.reason).toBe('Yalnız şablonun sahibi paylaşır.');
    expect(a.delete.reason).toContain('silinmez');
    expect(a.edit.label).toBe('Kopyasını düzenle');
    expect(a.duplicate.label).toBe('Şablonlarıma kopyala');
    // One shared to edit: edited in place (its new revision goes up).
    expect(actionsOf(editing, ready).edit.label).toBe('Düzenle');
    const off = actionsOf(cloud, away);
    expect(off.use.reason).toContain('yüklenemedi');
    expect(off.sync.reason).toBe('bulut yok');
    expect(off.delete.reason).toBe('giriş yok');
  });

  it('deletes and edits one of the cloud offline (it waits for the connection), but syncs and shares it only online', () => {
    const a = actionsOf(cloud, offline);
    expect(a.delete.reason).toBeNull();
    expect(a.edit.reason).toBeNull();
    expect(a.sync).toMatchObject({ label: 'Şimdi eşitle', reason: 'sunucu yok' });
    expect(a.share.reason).toBe('sunucu yok');
  });

  it('says why a section has nothing, and what an empty one is for', () => {
    expect(sectionNote({ state: 'ready' })).toBeNull();
    expect(sectionNote({ state: 'soon', reason: 'sonra' })).toBe('sonra');
    expect(emptyText('mine', false, { state: 'ready' }).text).toContain('Şablon olarak kaydet');
    expect(emptyText('system', false, { state: 'unavailable', reason: 'motor yok' })).toEqual({ title: 'Şimdi listelenemiyor', text: 'motor yok' });
    expect(emptyText('all', true, { state: 'ready' }).title).toBe('Eşleşen şablon yok');
  });

  it('offers the papers the template suits, its recommended one first', () => {
    const choices = paperChoices(system, name);
    expect(choices.map((c) => c.label)).toEqual(['A3 yatay (önerilen)', 'A2 yatay', 'A1 yatay']);
    expect(choices[1].choice).toEqual({ paper: 'a2', orientation: 'landscape' });
  });
});

describe('sheet tabs', () => {
  it('say nothing when the sheets can be made, and why not otherwise', () => {
    expect(tabsNote(2, { state: 'ready' })).toBe('');
    expect(tabsNote(2, { state: 'idle' })).toBe('');
    expect(tabsNote(0, { state: 'loading' })).toBe('Pafta motoru yükleniyor…');
    expect(tabsNote(2, { state: 'failed', reason: 'x' })).toContain('açılamıyor');
  });
});
