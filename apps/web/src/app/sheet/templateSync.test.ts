import { describe, expect, it } from 'vitest';
import type { Template } from '../../contracts/generated/sheet/Template';
import { testEngine } from '../../product/sheet/engineTesting';
import { MemoryKeyValue } from '../../product/sheet/store';
import { DeviceTemplateStore, type StoredTemplate } from '../../product/sheet/templateStore';
import { FakeTemplateCloud } from './templateCloudTesting';
import { TemplateSync, type SyncHost, type Timers } from './templateSync';

/**
 * The template library's sync against the cloud's rules (templateCloudTesting.ts):
 * every row of design §13's table carried out, the id the cloud gives a
 * template created from this device, a command whose answer was lost sent
 * again without a duplicate, the network failing and coming back, another
 * account's copies left alone, a role taken away while changes wait, and the
 * cloud's events starting a run.
 */

const e = testEngine();
const base = e.systemTemplates().find((t) => t.meta.id === 'sys:genel-a4-dikey')!;

function userTemplate(id: string, name: string): Template {
  return e.validateTemplate(JSON.stringify({ ...base, meta: { ...base.meta, id, revision: 1, name } }));
}

class ManualTimers implements Timers {
  readonly queue: { fn: () => void; ms: number; id: number }[] = [];
  private n = 0;
  set(fn: () => void, ms: number) {
    this.queue.push({ fn, ms, id: ++this.n });
    return this.n;
  }
  clear(h: unknown) {
    const at = this.queue.findIndex((q) => q.id === h);
    if (at >= 0) this.queue.splice(at, 1);
  }
  fire(): number[] {
    const due = this.queue.splice(0);
    due.forEach((q) => q.fn());
    return due.map((q) => q.ms);
  }
}

function device(cloud: FakeTemplateCloud, user: string, opts: { store?: DeviceTemplateStore } = {}) {
  const store = opts.store ?? new DeviceTemplateStore(new MemoryKeyValue());
  const said: { text: string; kind: string }[] = [];
  let account: { id: string; name: string } | null = { id: user, name: cloud.people.get(user)!.name };
  const host: SyncHost = {
    engine: () => e,
    store,
    account: () => account,
    personal: () => `kisisel-${user}`,
    say: (text, kind) => said.push({ text, kind }),
    changed: () => {},
    unreachable: () => {},
  };
  const timers = new ManualTimers();
  const sync = new TemplateSync(cloud.as(user), host, timers);
  return { store, said, sync, timers, signOut: () => (account = null) };
}

const records = async (store: DeviceTemplateStore): Promise<StoredTemplate[]> => (await store.list()).flatMap((r) => (r.status === 'ok' ? [r.record] : []));
const named = (r: StoredTemplate) => (r.template as Template).meta.name;
const tick = () => new Promise((r) => setTimeout(r, 0));

function people() {
  return new FakeTemplateCloud().person('ayse', 'Ayşe Yılmaz').person('mehmet', 'Mehmet Demir').person('disari', 'Dış Kişi', ['baska-kurum']);
}

describe('template sync', () => {
  it('creates a template asked for on the cloud, takes the id it gives, and keeps the old one for its sheets', async () => {
    const cloud = people();
    const a = device(cloud, 'ayse');
    await a.store.save('yerel-1', userTemplate('yerel-1', 'Belediye paftası'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await a.sync.run();
    expect(a.sync.status.value.state).toBe('synced');
    const [t] = [...cloud.templates.values()];
    expect(t.content.meta).toMatchObject({ id: t.id, revision: 1, name: 'Belediye paftası' });
    const [r] = await records(a.store);
    expect(r.id).toBe(t.id);
    expect(r.cloud).toMatchObject({ account: 'ayse', revision: 1, changed: false, role: 'owner', formerIds: ['yerel-1'] });
    expect((r.template as Template).meta.id).toBe(t.id);
    expect(a.sync.created.get('yerel-1')).toBe(t.id);
  });

  it('downloads what another device made, uploads a change here, and a template made only here stays here', async () => {
    const cloud = people();
    const one = device(cloud, 'ayse');
    const two = device(cloud, 'ayse');
    await one.store.save('y1', userTemplate('y1', 'Pafta A'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await two.store.save('yalniz-burada', userTemplate('yalniz-burada', 'Yalnız bu cihazda'));
    await one.sync.run();
    await two.sync.run();
    const id = [...cloud.templates.keys()][0];
    const got = await records(two.store);
    expect(got.map((r) => [named(r), r.cloud?.revision ?? null]).sort()).toEqual([
      ['Pafta A', 1],
      ['Yalnız bu cihazda', null],
    ]);
    // A change on the second device goes up as revision 2, and the first one downloads it.
    const mine = got.find((r) => r.id === id)!;
    const t = mine.template as Template;
    await two.store.save(id, { ...t, meta: { ...t.meta, description: 'ikinci cihazda değişti' } }, { ...mine.cloud!, changed: true });
    await two.sync.run();
    expect(cloud.templates.get(id)!.revision).toBe(2);
    expect((await two.store.get(id))).toMatchObject({ record: { cloud: { revision: 2, changed: false } } });
    await one.sync.run();
    const first = (await one.store.get(id))!;
    expect(first.status === 'ok' && (first.record.template as Template).meta).toMatchObject({ revision: 2, description: 'ikinci cihazda değişti' });
    expect(cloud.templates.size).toBe(1);
  });

  it('keeps both sides of a conflict: the cloud’s downloaded, this device’s uploaded as “(bu cihazdaki kopya)”', async () => {
    const cloud = people();
    const one = device(cloud, 'ayse');
    const two = device(cloud, 'ayse');
    await one.store.save('y1', userTemplate('y1', 'Ortak pafta'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await one.sync.run();
    await two.sync.run();
    const id = [...cloud.templates.keys()][0];
    const change = async (d: ReturnType<typeof device>, description: string) => {
      const r = (await d.store.get(id))!;
      if (r.status !== 'ok') throw new Error('okunamadı');
      const t = r.record.template as Template;
      await d.store.save(id, { ...t, meta: { ...t.meta, description } }, { ...r.record.cloud!, changed: true });
    };
    await change(one, 'birinci cihaz');
    await change(two, 'ikinci cihaz');
    await two.sync.run();
    await one.sync.run();
    const all = await records(one.store);
    expect(all.map(named).sort()).toEqual(['Ortak pafta', 'Ortak pafta (bu cihazdaki kopya)']);
    const copy = all.find((r) => named(r).endsWith('(bu cihazdaki kopya)'))!;
    expect((copy.template as Template).meta.description).toBe('birinci cihaz');
    expect(copy.cloud).toMatchObject({ revision: 1, changed: false, conflictOf: id });
    expect((all.find((r) => r.id === id)!.template as Template).meta.description).toBe('ikinci cihaz');
    expect([...cloud.templates.values()].map((t) => t.content.meta.name).sort()).toEqual(['Ortak pafta', 'Ortak pafta (bu cihazdaki kopya)']);
    expect(one.said.some((s) => s.kind === 'warn' && s.text.includes('ikisi de kaldı'))).toBe(true);
    // The other device hears of the copy at its next run.
    await two.sync.run();
    expect((await records(two.store)).length).toBe(2);
  });

  it('keeps a change whose template was deleted elsewhere as this device’s own, and says so', async () => {
    const cloud = people();
    const one = device(cloud, 'ayse');
    await one.store.save('y1', userTemplate('y1', 'Silinecek'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await one.sync.run();
    const id = [...cloud.templates.keys()][0];
    const r = (await one.store.get(id))!;
    if (r.status !== 'ok') throw new Error();
    await one.store.save(id, r.record.template, { ...r.record.cloud!, changed: true });
    cloud.templates.get(id)!.deleted = true;
    await one.sync.run();
    const kept = (await one.store.get(id))!;
    expect(kept).toMatchObject({ status: 'ok', record: { id } });
    expect(kept.status === 'ok' && kept.record.cloud).toBeUndefined();
    expect(one.said.at(-1)).toMatchObject({ kind: 'warn' });
  });

  it('deletes in the cloud what was deleted here, and brings back one that changed after', async () => {
    const cloud = people();
    const one = device(cloud, 'ayse');
    const two = device(cloud, 'ayse');
    await one.store.save('y1', userTemplate('y1', 'A'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await one.store.save('y2', userTemplate('y2', 'B'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await one.sync.run();
    await two.sync.run();
    const [a, b] = [...cloud.templates.values()];
    // Deleted here at the cloud's revision: deleted there too.
    const ra = (await one.store.get(a.id))!;
    if (ra.status !== 'ok') throw new Error();
    await one.store.save(a.id, ra.record.template, { ...ra.record.cloud!, deleted: true });
    // Deleted here while the other device changed it: the cloud's stays and comes back.
    const rb = (await one.store.get(b.id))!;
    if (rb.status !== 'ok') throw new Error();
    await one.store.save(b.id, rb.record.template, { ...rb.record.cloud!, deleted: true });
    const tb = (await two.store.get(b.id))!;
    if (tb.status !== 'ok') throw new Error();
    await two.store.save(b.id, tb.record.template, { ...tb.record.cloud!, changed: true });
    await two.sync.run();
    await one.sync.run();
    expect(cloud.templates.get(a.id)!.deleted).toBe(true);
    expect(await one.store.get(a.id)).toBeNull();
    expect(await one.store.get(b.id)).toMatchObject({ status: 'ok', record: { cloud: { revision: 2, changed: false } } });
    expect(one.said.some((s) => s.text.includes('yeniden indirildi'))).toBe(true);
  });

  it('takes a template off the list of the one it is no longer shared with; a view-only one cannot be written', async () => {
    const cloud = people();
    const ayse = device(cloud, 'ayse');
    const mehmet = device(cloud, 'mehmet');
    await ayse.store.save('y1', userTemplate('y1', 'Paylaşılan'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await ayse.sync.run();
    const id = [...cloud.templates.keys()][0];
    await cloud.as('ayse').command('kisisel-ayse', { name: 'sheet.template.share', input: { templateId: id, userId: 'mehmet', role: 'viewer' } }, 'paylas-1');
    // Not with someone outside every common organisation.
    await expect(cloud.as('ayse').command('kisisel-ayse', { name: 'sheet.template.share', input: { templateId: id, userId: 'disari', role: 'viewer' } }, 'paylas-2')).rejects.toMatchObject({ status: 422 });
    await mehmet.sync.run();
    expect(await mehmet.store.get(id)).toMatchObject({ record: { cloud: { role: 'viewer', ownerName: 'Ayşe Yılmaz' } } });
    await ayse.sync.run();
    expect(await ayse.store.get(id)).toMatchObject({ record: { cloud: { role: 'owner', shared: true } } });
    await cloud.as('ayse').command('kisisel-ayse', { name: 'sheet.template.unshare', input: { templateId: id, userId: 'mehmet' } }, 'kaldir-1');
    await mehmet.sync.run();
    expect(await mehmet.store.get(id)).toBeNull();
    expect(mehmet.said.at(-1)!.text).toContain('artık sizinle paylaşılmıyor');
  });

  it('keeps the changes of an editor made view-only as a template of their own, and goes back to the cloud’s', async () => {
    const cloud = people();
    const ayse = device(cloud, 'ayse');
    const mehmet = device(cloud, 'mehmet');
    await ayse.store.save('y1', userTemplate('y1', 'Ortak'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await ayse.sync.run();
    const id = [...cloud.templates.keys()][0];
    const share = (role: 'viewer' | 'editor', key: string) => cloud.as('ayse').command('kisisel-ayse', { name: 'sheet.template.share', input: { templateId: id, userId: 'mehmet', role } }, key);
    await share('editor', 'p1');
    await mehmet.sync.run();
    const r = (await mehmet.store.get(id))!;
    if (r.status !== 'ok') throw new Error();
    const t = r.record.template as Template;
    await mehmet.store.save(id, { ...t, meta: { ...t.meta, description: 'Mehmet’in değişikliği' } }, { ...r.record.cloud!, changed: true });
    await share('viewer', 'p2');
    await mehmet.sync.run();
    const all = await records(mehmet.store);
    expect(all.map(named).sort()).toEqual(['Ortak', 'Ortak (bu cihazdaki kopya)']);
    expect(all.find((x) => x.id === id)!.cloud).toMatchObject({ role: 'viewer', changed: false });
    expect((all.find((x) => x.id !== id)!.template as Template).meta.description).toBe('Mehmet’in değişikliği');
    expect(all.find((x) => x.id !== id)!.cloud).toBeUndefined();
    expect(cloud.templates.get(id)!.revision).toBe(1);
  });

  it('sends a command whose answer was lost again under the same key: one template, not two', async () => {
    const cloud = people();
    const one = device(cloud, 'ayse');
    await one.store.save('y1', userTemplate('y1', 'Bir kez'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    cloud.loseNextAnswer = true;
    await one.sync.run();
    expect(one.sync.status.value.state).toBe('offline');
    expect(await one.store.get('y1')).not.toBeNull();
    await one.sync.run();
    expect(cloud.templates.size).toBe(1);
    expect((await records(one.store)).map((r) => r.id)).toEqual([...cloud.templates.keys()]);
  });

  it('waits offline with the cached copies, retries later, and runs at once when the connection is back', async () => {
    const cloud = people();
    const one = device(cloud, 'ayse');
    await one.store.save('y1', userTemplate('y1', 'Çevrimdışı'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    cloud.offline = true;
    await one.sync.run();
    expect(one.sync.status.value.state).toBe('offline');
    expect(one.timers.queue.map((q) => q.ms)).toEqual([5_000]);
    expect(cloud.templates.size).toBe(0);
    // Still offline at the retry: the next wait is longer.
    one.timers.fire();
    await tick();
    await tick();
    expect(one.timers.queue.map((q) => q.ms)).toContain(15_000);
    cloud.offline = false;
    one.sync.reconnected();
    await one.sync.run();
    expect(one.sync.status.value.state).toBe('synced');
    expect(cloud.templates.size).toBe(1);
    one.sync.stop();
  });

  it('leaves the copies of another account signed in on this browser alone', async () => {
    const cloud = people();
    const store = new DeviceTemplateStore(new MemoryKeyValue());
    const ayse = device(cloud, 'ayse', { store });
    await store.save('y1', userTemplate('y1', 'Ayşe’nin'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await ayse.sync.run();
    const mehmet = device(cloud, 'mehmet', { store });
    await mehmet.sync.run();
    expect((await records(store)).map((r) => r.cloud?.account)).toEqual(['ayse']);
    expect(cloud.templates.size).toBe(1);
  });

  it('runs again when the cloud’s events say something changed', async () => {
    const cloud = people();
    const one = device(cloud, 'ayse');
    const two = device(cloud, 'ayse');
    two.sync.start();
    await tick();
    await tick();
    await one.store.save('y1', userTemplate('y1', 'Olayla gelen'), { account: 'ayse', revision: 0, changed: true, role: 'owner' });
    await one.sync.run();
    await tick();
    // The event woke the long poll; the run waits for the burst to settle.
    expect(two.timers.queue.map((q) => q.ms)).toEqual([300]);
    two.timers.fire();
    for (let i = 0; i < 6 && !(await records(two.store)).length; i++) await tick();
    expect((await records(two.store)).map(named)).toEqual(['Olayla gelen']);
    two.sync.stop();
  });
});
