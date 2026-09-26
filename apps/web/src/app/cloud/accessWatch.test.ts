import { afterEach, describe, expect, it } from 'vitest';
import { CadDocument } from '../../model/document';
import { LayerStore } from '../../model/layers';
import { AccessWatch } from './accessWatch';
import { ApiFailure } from './api';
import { MemoryDraftStore } from './drafts';
import { FakeServer } from './fakeServer';
import type { CloudProject } from './session';
import { ProjectSync } from './sync';

const open: ProjectSync[] = [];
afterEach(() => {
  for (const s of open.splice(0)) s.dispose();
});

/** An open project owned by the caller, its autosave and the watch over its access. */
function setup() {
  const doc = new CadDocument({ name: 'Ada 101', layers: new LayerStore([{ id: 'cizim', name: 'Çizim' }], 'cizim'), origin: { x: 486500, y: 4420200 } });
  const server = new FakeServer({ name: 'Ada 101', settings: doc.settings.toJSON(), layers: [...doc.layers.tree], activeLayer: 'cizim', styles: { items: [], categories: [] }, origin: doc.origin });
  const said: string[] = [];
  const revoked: string[] = [];
  let project: CloudProject | null = {
    tenantId: 't',
    tenantName: 'Büro',
    tenantKind: 'organization',
    projectId: 'p',
    name: 'Ada 101',
    role: 'owner',
    state: 'active',
    permissions: ['project.read', 'feature.write', 'project.edit', 'project.delete', 'project.comment', 'project.download', 'project.history', 'project.share', 'project.transfer', 'project.jobs.run'],
    canWrite: true,
    canEditMeta: true,
  };
  let watch: AccessWatch | null = null;
  const sync = new ProjectSync({
    doc,
    api: server,
    drafts: new MemoryDraftStore(),
    draftKey: 'u1/t/p',
    userId: 'u1',
    tenantId: 't',
    projectId: 'p',
    canEditMeta: true,
    metaVersion: '1',
    cursor: '0',
    records: [],
    warn: (t) => said.push(t),
    onRevoked: (r) => revoked.push(r),
    onAccessChanged: () => void watch?.check(),
    debounceMs: 60_000,
    maxDelayMs: 60_000,
  });
  open.push(sync);
  let resubscribed = 0;
  watch = new AccessWatch({
    api: server,
    sync,
    project: () => project,
    update: (next) => (project = next),
    resubscribe: () => resubscribed++,
    info: (t) => said.push(t),
    warn: (t) => said.push(t),
  });
  return { doc, server, sync, watch, said, revoked, now: () => project, leave: () => (project = null), resubscribed: () => resubscribed };
}

describe('the open project’s access, kept current', () => {
  it('a lowered role disables writing and says so; raised again, saving resumes', async () => {
    const { doc, server, sync, watch, said, now } = setup();
    server.grant('viewer');
    await watch.check();
    expect([now()!.role, now()!.canWrite, now()!.canEditMeta, now()!.permissions.includes('project.share')]).toEqual(['viewer', false, false, false]);
    expect(sync.state.value).toBe('readonly');
    expect(said.at(-1)).toBe('“Ada 101” projesinde rolünüz artık Görüntüleyici: değişiklikleriniz buluta kaydedilmiyor.');
    doc.add({ kind: 'point', layerId: 'cizim', p: { x: 1, y: 2 }, attrs: {} });
    expect(await sync.flush()).toBe(false);
    server.grant('editor');
    await watch.check();
    expect(now()!.canWrite).toBe(true);
    expect(said.at(-1)).toMatch(/Düzenleyici: değişiklikleriniz yeniden buluta kaydediliyor/);
    await sync.flush();
    expect(server.commits).toBe(1);
    // Asking again with nothing changed says nothing more.
    const before = said.length;
    await watch.check();
    expect(said.length).toBe(before);
  });

  it('access taken away stops the project; one still there after a refused subscription subscribes again', async () => {
    const first = setup();
    await first.watch.check(true);
    expect([first.resubscribed(), first.revoked.length]).toEqual([1, 0]);
    first.server.grant(null);
    await first.watch.check(true);
    expect([first.sync.state.value, first.revoked, first.resubscribed()]).toEqual(['revoked', [''], 1]);
    // A membership that may not be used now: the server's reason goes with it.
    const second = setup();
    second.server.project = async () => {
      throw new ApiFailure(403, { error: 'forbidden', message: '“Büro” kurumunda size koltuk ayrılmamış; kurum yöneticinize başvurun.' }, 'x');
    };
    await second.watch.check();
    expect(second.revoked).toEqual(['“Büro” kurumunda size koltuk ayrılmamış; kurum yöneticinize başvurun.']);
  });

  it('checks never overlap, and an answer for a project already left is dropped', async () => {
    const { server, watch, sync, leave, now } = setup();
    let asked = 0;
    const real = server.project.bind(server);
    server.project = async () => {
      asked++;
      return real();
    };
    server.grant('viewer');
    const a = watch.check();
    const b = watch.check();
    expect(a).toBe(b);
    await a;
    // The second question ran after the first answer.
    await new Promise((r) => setTimeout(r, 0));
    expect(asked).toBe(2);
    leave();
    server.grant(null);
    await watch.check();
    expect([now(), sync.state.value]).toEqual([null, 'readonly']);
  });
});
