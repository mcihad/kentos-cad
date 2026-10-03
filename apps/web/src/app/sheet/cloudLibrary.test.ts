import { describe, expect, it } from 'vitest';
import type { Template } from '../../contracts/generated/sheet/Template';
import { testEngine } from '../../product/sheet/engineTesting';
import { MemoryKeyValue } from '../../product/sheet/store';
import { SheetService } from './service';
import { fakeApp } from './sheetAppTesting';
import { FakeTemplateCloud } from './templateCloudTesting';
import { templateAction } from './templateActions';

/**
 * The cloud's template library in the sheet service (docs/sheet/design.md
 * §12, §13): it comes with the engine and syncs at once for the signed-in
 * account; a template shared with the account is listed under “Benimle
 * paylaşılanlar” with its role and owner, used with its questions asked,
 * and the sheet made from it says “Yeni sürüm var” when the owner saves a
 * newer revision. A template made on this device and synced keeps its
 * sheets: the cloud's new id answers their old one. Leaving the questions
 * makes no sheet.
 */

const e = testEngine();
const loaded = () => Promise.resolve(e);
const ifraz = e.systemTemplates().find((t) => t.meta.id === 'sys:ifraz-paftasi')!;
const noQuestions = async () => [];

function userTemplate(id: string, name: string, revision = 1): Template {
  return e.validateTemplate(JSON.stringify({ ...ifraz, meta: { ...ifraz.meta, id, revision, name } }));
}

function service(cloud: FakeTemplateCloud, id: string, name: string) {
  const { app, ctx } = fakeApp({ account: { id, name } });
  const sheets = new SheetService(app, new MemoryKeyValue(), loaded, { templateApi: cloud.as(id) });
  return { app, ctx, sheets };
}

describe('cloud template library in the sheet service', () => {
  it('lists a template shared with the account, uses it with its questions answered, and hears its newer revision', async () => {
    const cloud = new FakeTemplateCloud().person('ayse', 'Ayşe Yılmaz').person('mehmet', 'Mehmet Demir');
    const ayse = cloud.as('ayse');
    const made = await ayse.command('kisisel-ayse', { name: 'sheet.template.create', input: { content: userTemplate('y', 'Belediye ifraz paftası') } }, 'k-1');
    await ayse.command('kisisel-ayse', { name: 'sheet.template.share', input: { templateId: made.templateId, userId: 'mehmet', role: 'viewer' } }, 'k-2');
    const { app, sheets } = service(cloud, 'mehmet', 'Mehmet Demir');
    await sheets.ensureEngine();
    await sheets.shelf.library!.sync.run();
    await sheets.shared.refresh();
    const card = sheets.shared.cards.value[0];
    expect(card).toMatchObject({ name: 'Belediye ifraz paftası', source: 'shared', role: 'viewer', sharedBy: 'Ayşe Yılmaz', badges: ['viewer', 'synced'] });
    expect(sheets.galleryAbilities()).toMatchObject({ cloud: null, account: null });
    // Leaving the questions makes nothing; answering them makes the sheet with the answers.
    expect(await templateAction(app, sheets, card, 'use', null, async () => null)).toBe(false);
    expect(sheets.book()?.book.sheets.length ?? 0).toBe(0);
    const answers = async (t: Template) => t.variables.map((v) => ({ name: v.name, value: v.name === 'ada' ? '1245' : v.value }));
    expect(await templateAction(app, sheets, card, 'use', null, answers)).toBe(true);
    expect(sheets.book()!.book.sheets[0].variables.find((v) => v.name === 'ada')!.value).toBe('1245');
    expect(sheets.state.book.value.sheets[0].template).toEqual({ name: 'Belediye ifraz paftası', newer: false });
    // Ayşe saves revision 2: the next sync brings it, and the sheet says “Yeni sürüm var”.
    await ayse.command('kisisel-ayse', { name: 'sheet.template.update', input: { templateId: card.id, expectedRevision: 1, content: { ...card.template, meta: { ...card.template.meta, description: 'ikinci' } } } }, 'k-3');
    await sheets.shelf.library!.sync.run();
    await sheets.shared.refresh();
    expect(sheets.shared.cards.value[0].revision).toBe(2);
    expect(sheets.state.book.value.sheets[0].template).toEqual({ name: 'Belediye ifraz paftası', newer: true });
    sheets.dispose();
  });

  it('keeps the sheets of a template made here when the cloud gives it its own id, and they hear its newer revisions', async () => {
    const cloud = new FakeTemplateCloud().person('ayse', 'Ayşe Yılmaz');
    const { app, sheets } = service(cloud, 'ayse', 'Ayşe Yılmaz');
    await sheets.ensureEngine();
    // Saved here and updated once (revision 2), a sheet made from it.
    await sheets.templates.save('yerel', userTemplate('yerel', 'Yerel şablon', 2));
    await sheets.device.refresh();
    const card = sheets.device.cards.value.find((c) => c.id === 'yerel')!;
    expect(card.badges).toEqual(['device']);
    expect(await templateAction(app, sheets, card, 'use', null, noQuestions)).toBe(true);
    // Buluta eşitle.
    expect(await templateAction(app, sheets, card, 'sync', null, noQuestions)).toBe(true);
    await sheets.device.refresh();
    const up = sheets.device.cards.value[0];
    expect(up).toMatchObject({ source: 'cloud', revision: 1, formerIds: ['yerel'], formerRevision: 2, badges: ['synced'] });
    expect(sheets.state.book.value.sheets[0].template).toEqual({ name: 'Yerel şablon', newer: false });
    // Another device saves revision 2 in the cloud: the sheet made here from “yerel” says “Yeni sürüm var”.
    await cloud.as('ayse').command('kisisel-ayse', { name: 'sheet.template.update', input: { templateId: up.id, expectedRevision: 1, content: { ...up.template, meta: { ...up.template.meta, description: 'başka cihaz' } } } }, 'k-9');
    await sheets.shelf.library!.sync.run();
    await sheets.device.refresh();
    expect(sheets.state.book.value.sheets[0].template).toEqual({ name: 'Yerel şablon', newer: true });
    sheets.dispose();
  });

  it('takes a conflict’s copy saved again as the user’s own (its “Çakışma” goes), and deletes one of the library everywhere', async () => {
    const cloud = new FakeTemplateCloud().person('ayse', 'Ayşe Yılmaz');
    const { sheets } = service(cloud, 'ayse', 'Ayşe Yılmaz');
    await sheets.ensureEngine();
    const made = await cloud.as('ayse').command('kisisel-ayse', { name: 'sheet.template.create', input: { content: userTemplate('k', 'Kopya') } }, 'k-1');
    await sheets.shelf.library!.sync.run();
    const r = (await sheets.templates.get(made.templateId))!;
    if (r.status !== 'ok') throw new Error();
    await sheets.templates.save(made.templateId, r.record.template, { ...r.record.cloud!, conflictOf: 'asil' });
    await sheets.device.refresh();
    expect(sheets.device.cards.value[0].badges).toEqual(['synced', 'conflict']);
    await sheets.shelf.library!.saved(made.templateId, r.record.template as Template);
    await sheets.shelf.library!.sync.run();
    await sheets.device.refresh();
    expect(sheets.device.cards.value[0]).toMatchObject({ revision: 2, badges: ['synced'] });
    expect(await sheets.shelf.library!.remove(made.templateId, 'Kopya')).toBe(true);
    await sheets.shelf.library!.sync.run();
    expect(cloud.templates.get(made.templateId)!.deleted).toBe(true);
    expect(await sheets.templates.get(made.templateId)).toBeNull();
    sheets.dispose();
  });

  it('says why the cloud cannot be used when nobody is signed in, and lists only this device’s templates then', async () => {
    const { app } = fakeApp();
    const sheets = new SheetService(app, new MemoryKeyValue(), loaded, { templateApi: new FakeTemplateCloud().as('kimse') });
    await sheets.ensureEngine();
    await sheets.templates.save('ayse-nin', userTemplate('ayse-nin', 'Başkasının'), { account: 'ayse', revision: 3, changed: false, role: 'owner' });
    await sheets.templates.save('burada', userTemplate('burada', 'Bu cihazın'));
    await sheets.device.refresh();
    await sheets.shared.refresh();
    expect(sheets.galleryAbilities().cloud).toContain('giriş yapın');
    expect(sheets.device.cards.value.map((c) => c.name)).toEqual(['Bu cihazın']);
    expect(sheets.shared.state.value).toMatchObject({ state: 'unavailable' });
    sheets.dispose();
  });
});
