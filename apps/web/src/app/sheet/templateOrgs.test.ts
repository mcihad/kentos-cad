import { describe, expect, it } from 'vitest';
import type { Template } from '../../contracts/generated/sheet/Template';
import { testEngine } from '../../product/sheet/engineTesting';
import { MemoryKeyValue } from '../../product/sheet/store';
import { DeviceTemplateStore, readStoredTemplate, type StoredTemplate } from '../../product/sheet/templateStore';
import { actionsOf, ORGANISATION_NO_DELETE, PUBLISH_NEEDS_SYNC, PUBLISH_NO_ORGANISATION, PUBLISH_WAIT_SYNC } from '../../ui/sheet/galleryPlan';
import { libraryBadges } from './providers';
import { SheetService } from './service';
import { fakeApp } from './sheetAppTesting';
import { FakeTemplateCloud } from './templateCloudTesting';
import { SYNC_TEXTS, TemplateSync, type SyncHost } from './templateSync';

/**
 * Organisations' template libraries on the web (docs/sheet/design.md §13
 * “Kurum şablonları”; .run/sheet-engine-ready “KURUM”), against the cloud's
 * rules (templateCloudTesting.ts): a member's sync brings its organisations'
 * libraries with the member's role (published it, administers it, a plain
 * member); a template is published from one's own on the organisation's
 * route; an organisation's template's commands go to that organisation, a
 * conflict's copy of one to the personal space; one who leaves loses its
 * templates here with a word; the gallery's actions follow the role; and
 * “Kurumdakini güncelle” puts a template's content into the organisation's copy.
 */

const e = testEngine();
const base = e.systemTemplates().find((t) => t.meta.id === 'sys:genel-a4-dikey')!;
const userTemplate = (id: string, name: string): Template => e.validateTemplate(JSON.stringify({ ...base, meta: { ...base.meta, id, revision: 1, name } }));
const ORG = 'ornek-buro';

/** Zeynep administers the organisation, Ayşe may publish (a project manager), Mehmet is a plain member, Dış Kişi is of none. */
function cloud() {
  return new FakeTemplateCloud()
    .person('zeynep', 'Zeynep Kaya')
    .person('ayse', 'Ayşe Yılmaz')
    .person('mehmet', 'Mehmet Demir')
    .person('disari', 'Dış Kişi', [])
    .org(ORG, 'Örnek Harita Bürosu', { zeynep: 'admin', ayse: 'project_manager', mehmet: 'editor' });
}

function device(c: FakeTemplateCloud, user: string) {
  const store = new DeviceTemplateStore(new MemoryKeyValue());
  const said: { text: string; kind: string }[] = [];
  const host: SyncHost = {
    engine: () => e,
    store,
    account: () => ({ id: user, name: c.people.get(user)!.name }),
    personal: () => `kisisel-${user}`,
    say: (text, kind) => said.push({ text, kind }),
    changed: () => {},
    unreachable: () => {},
  };
  return { store, said, sync: new TemplateSync(c.as(user), host) };
}

const records = async (store: DeviceTemplateStore): Promise<StoredTemplate[]> => (await store.list()).flatMap((r) => (r.status === 'ok' ? [r.record] : []));

/** Ayşe's own template in the cloud, published into the organisation; its id there. */
async function published(c: FakeTemplateCloud): Promise<{ own: string; org: string }> {
  const api = c.as('ayse');
  const own = await api.command('kisisel-ayse', { name: 'sheet.template.create', input: { content: userTemplate('y', 'Belediye ifraz paftası') } }, 'k-1');
  const org = await api.command(ORG, { name: 'sheet.template.publish', input: { templateId: own.templateId, tenantId: ORG } }, 'k-2');
  return { own: own.templateId, org: org.templateId };
}

describe('organisations’ template libraries', () => {
  it('keeps a record’s organisation and source, and an administrator’s role', () => {
    const raw = { format: 'kentos.sheets.deviceTemplate', version: 1, id: 't', saved: 1, template: {}, cloud: { account: 'zeynep', revision: 2, changed: false, role: 'admin', organization: { tenantId: ORG, name: 'Örnek Harita Bürosu' }, publishedFrom: 'kaynak' } };
    const read = readStoredTemplate(raw, 't');
    expect(read.status === 'ok' && read.record.cloud).toEqual({ account: 'zeynep', revision: 2, changed: false, role: 'admin', organization: { tenantId: ORG, name: 'Örnek Harita Bürosu' }, publishedFrom: 'kaynak' });
  });

  it('brings each member the organisation’s library with their role, and says who may publish', async () => {
    const c = cloud();
    const { own, org } = await published(c);
    const want = { ayse: 'owner', zeynep: 'admin', mehmet: 'viewer' } as const;
    for (const [user, role] of Object.entries(want)) {
      const d = device(c, user);
      await d.sync.run();
      const r = (await records(d.store)).find((x) => x.id === org)!;
      expect(r.cloud).toMatchObject({ role, organization: { tenantId: ORG, name: 'Örnek Harita Bürosu' }, publishedFrom: own });
      expect(d.sync.organizations.value).toEqual([{ tenantId: ORG, name: 'Örnek Harita Bürosu', canPublish: user !== 'mehmet' }]);
      // Its badges: the organisation, then the account's part in it (a plain member none), then the sync.
      expect(libraryBadges(r, new Set())).toEqual(role === 'owner' ? ['org', 'published', 'synced'] : role === 'admin' ? ['org', 'admin', 'synced'] : ['org', 'synced']);
    }
    // One of no organisation sees none of it.
    const out = device(c, 'disari');
    await out.sync.run();
    expect(await records(out.store)).toEqual([]);
    expect(out.sync.organizations.value).toEqual([]);
  });

  it('sends an organisation’s template’s change to the organisation’s route, a conflict’s copy to the personal space', async () => {
    const c = cloud();
    const { org } = await published(c);
    const z1 = device(c, 'zeynep');
    const z2 = device(c, 'zeynep');
    await z1.sync.run();
    await z2.sync.run();
    // The administrator changes it on one device: it goes up as revision 2 on the organisation's route.
    const r1 = (await records(z1.store)).find((x) => x.id === org)!;
    await z1.store.save(org, { ...(r1.template as Template), meta: { ...(r1.template as Template).meta, description: 'yöneticinin değişikliği' } }, { ...r1.cloud!, changed: true });
    await z1.sync.run();
    expect(c.templates.get(org)!.revision).toBe(2);
    // The other device changed it too, from revision 1: the cloud's comes down, its own goes up as a personal copy.
    const r2 = (await records(z2.store)).find((x) => x.id === org)!;
    await z2.store.save(org, { ...(r2.template as Template), meta: { ...(r2.template as Template).meta, description: 'öbür cihaz' } }, { ...r2.cloud!, changed: true });
    await z2.sync.run();
    const copy = (await records(z2.store)).find((x) => x.cloud?.conflictOf === org)!;
    expect(copy.cloud!.organization).toBeUndefined();
    expect(c.templates.get(copy.id)!.org).toBeUndefined();
    expect(c.templates.get(copy.id)!.owner).toBe('zeynep');
    expect((await records(z2.store)).find((x) => x.id === org)!.cloud).toMatchObject({ revision: 2, changed: false, organization: { tenantId: ORG } });
  });

  it('takes an organisation’s templates off the device of one who left it, and says so', async () => {
    const c = cloud();
    const { org } = await published(c);
    const m = device(c, 'mehmet');
    await m.sync.run();
    expect((await records(m.store)).some((x) => x.id === org)).toBe(true);
    c.leave(ORG, 'mehmet');
    await m.sync.run();
    expect(await records(m.store)).toEqual([]);
    expect(m.sync.organizations.value).toEqual([]);
    expect(m.said).toContainEqual({ text: SYNC_TEXTS.orgGone('Belediye ifraz paftası', 'Örnek Harita Bürosu'), kind: 'info' });
  });

  it('offers each role what it may do with an organisation’s template, and says why not', () => {
    const can = { engine: null, cloud: null, account: null, publishTo: 1 };
    const card = (role: 'owner' | 'admin' | 'viewer') => ({ source: 'org' as const, role, badges: ['org', 'synced'] as const }) as never;
    for (const role of ['owner', 'admin'] as const) {
      const a = actionsOf(card(role), can);
      expect([a.edit.label, a.delete.reason, a.share.shown, a.duplicate.label, a.publish.shown]).toEqual(['Düzenle', null, false, 'Şablonlarıma kopyala', false]);
    }
    const viewer = actionsOf(card('viewer'), can);
    expect([viewer.edit.label, viewer.delete.shown, viewer.delete.reason, viewer.share.shown, viewer.sync.shown]).toEqual(['Kopyasını düzenle', true, ORGANISATION_NO_DELETE, false, true]);
    // Kuruma yayımla… on one's own: only once synced, where an organisation takes it, with nothing waiting here.
    const own = (source: 'device' | 'cloud', badges: string[]) => ({ source, role: 'owner', badges }) as never;
    expect(actionsOf(own('device', ['device']), can).publish).toMatchObject({ shown: true, label: 'Kuruma yayımla…', reason: PUBLISH_NEEDS_SYNC });
    expect(actionsOf(own('cloud', ['synced']), can).publish.reason).toBeNull();
    expect(actionsOf(own('cloud', ['unsynced']), can).publish.reason).toBe(PUBLISH_WAIT_SYNC);
    expect(actionsOf(own('cloud', ['synced']), { ...can, publishTo: 0 }).publish.reason).toBe(PUBLISH_NO_ORGANISATION);
  });

  it('publishes from the library and brings the organisation’s copy up to date (“Kurumdakini güncelle”)', async () => {
    const c = cloud();
    const { app } = fakeApp({ account: { id: 'ayse', name: 'Ayşe Yılmaz' } });
    const s = new SheetService(app, new MemoryKeyValue(), () => Promise.resolve(e), { templateApi: c.as('ayse') });
    await s.ensureEngine();
    const lib = s.shelf.library!;
    const own = await c.as('ayse').command('kisisel-ayse', { name: 'sheet.template.create', input: { content: userTemplate('y', 'Belediye ifraz paftası') } }, 'k-1');
    await lib.sync.run();
    expect(await lib.publishTargets(own.templateId)).toEqual([{ tenantId: ORG, name: 'Örnek Harita Bürosu' }]);
    const id = await lib.publish(own.templateId, ORG);
    expect(c.templates.get(id)).toMatchObject({ org: ORG, publishedFrom: own.templateId, revision: 1 });
    await s.shelf.org.refresh();
    expect(s.shelf.org.cards.value.map((x) => [x.id, x.source, x.role, x.organization?.name])).toEqual([[id, 'org', 'owner', 'Örnek Harita Bürosu']]);
    // Published before: the window offers the copy; its content changes here and goes up as its revision 2.
    const [target] = await lib.publishTargets(own.templateId);
    expect(target.copy).toEqual({ id, name: 'Belediye ifraz paftası', revision: 1 });
    const mine = (await s.templates.get(own.templateId))!;
    if (mine.status !== 'ok') throw new Error('kayıt yok');
    const t = mine.record.template as Template;
    await s.templates.save(own.templateId, { ...t, meta: { ...t.meta, description: 'yeni açıklama' } }, mine.record.cloud);
    await lib.republish(own.templateId, id);
    expect(c.templates.get(id)).toMatchObject({ revision: 2, org: ORG });
    expect(c.templates.get(id)!.content.meta.description).toBe('yeni açıklama');
    expect(app.log.entries.value.map((x) => x.text)).toContain('“Belediye ifraz paftası” “Örnek Harita Bürosu” kurumunda yayımlandı: kurumun etkin üyeleri görür ve kullanır.');
    s.dispose();
  });
});
