import { afterEach, describe, expect, it } from 'vitest';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import { ProjectLifecycle, type ProjectRef } from './lifecycle';
import type { CloudProject, CloudSession } from './session';
import type { ProjectSync } from './sync';
import { disposeAll, pt, setup, storedDraft } from './syncTesting';

/**
 * Archiving and the other catalog commands as the open project's autosave
 * lives them (docs/adr/0028): archived by someone else, it stops like a
 * deleted one and keeps the edits on this device; archived from here, it
 * stops without a warning; a write refused as archived stops it too; the
 * catalog's commands go first through the open project's own paths.
 */

afterEach(disposeAll);

const ref: ProjectRef = { tenantId: 't', projectId: 'p', name: 'Ada 101' };

describe('an archived project’s autosave', () => {
  it('stops when someone archives it, and keeps what is not sent on this device', async () => {
    const { doc, server, sync, drafts, told } = setup();
    doc.add(pt(486510));
    await sync.flush();
    doc.add(pt(486520));
    await sync.receive([server.archiveAs('baskasi')]);
    expect(sync.state.value).toBe('archived');
    expect(told.archived).toBe(1);
    // Edits from now on stay on the device and are not sent.
    doc.add(pt(486530));
    await new Promise((r) => setTimeout(r, 400));
    expect(await sync.flush()).toBe(false);
    expect(server.store.size).toBe(1);
    const draft = await storedDraft(drafts);
    expect(Object.keys(draft!.changes)).toHaveLength(2);
  });

  it('stops quietly when this window archived it', async () => {
    const { server, sync, told } = setup();
    sync.expect('buradan');
    await sync.receive([server.archiveAs('buradan')]);
    expect([sync.state.value, told.archived]).toEqual(['archived', 0]);
  });

  it('stops when a write is refused as archived (409), before the event arrives', async () => {
    const { doc, server, sync, told } = setup();
    server.archived = true;
    doc.add(pt(486510));
    expect(await sync.flush()).toBe(false);
    expect([sync.state.value, told.archived]).toEqual(['archived', 1]);
  });

  it('opened already archived: read-only, nothing announced, edits not kept', async () => {
    const { doc, sync, told, warnings, drafts } = setup({ canWrite: false, canEditMeta: false });
    sync.markArchived(true);
    doc.add(pt(486510));
    await new Promise((r) => setTimeout(r, 400));
    expect([sync.state.value, told.archived]).toEqual(['archived', 0]);
    expect(warnings.some((w) => /arşivlenmiş, salt okunurdur/.test(w))).toBe(true);
    expect(await storedDraft(drafts)).toBeNull();
  });
});

describe('the catalog’s commands on the open project', () => {
  /** A session stub with an open project whose autosave is `sync`. */
  function session(sync: ProjectSync, server: ReturnType<typeof setup>['server'], open = true) {
    const calls: string[] = [];
    let project: CloudProject | null = open
      ? { tenantId: 't', tenantName: 'Büro', tenantKind: 'organization', projectId: 'p', name: 'Ada 101', role: 'owner', state: 'active', permissions: [], canWrite: true, canEditMeta: true }
      : null;
    const s = {
      api: server,
      sync: { value: sync },
      openProject: () => project,
      flush: async () => {
        calls.push('flush');
        return sync.flush();
      },
      projectArchived: (_p: CloudProject, byMe: boolean) => {
        calls.push(`archived:${byMe}`);
        if (project) project = { ...project, state: 'archived' };
      },
      leftTrashed: (_p: CloudProject, until: string) => {
        calls.push(`trashed:${until}`);
        project = null;
      },
      rename: async (_t: string, _p: string, name: string) => {
        calls.push(`rename:${name}`);
        return true;
      },
      open: async () => {
        calls.push('open');
        return true;
      },
    };
    return { lifecycle: new ProjectLifecycle(s as unknown as CloudSession, { info: (t) => calls.push(`info:${t}`) }), calls };
  }

  it('archives after sending what waits, and stays quiet when the event comes back', async () => {
    const { doc, server, sync, told } = setup();
    doc.add(pt(486510));
    const { lifecycle, calls } = session(sync, server);
    const done = await lifecycle.archive(ref);
    expect(done.changed).toBe(true);
    expect(server.store.size).toBe(1);
    expect(calls).toEqual(['flush', 'archived:true']);
    // The server's event names this window's request: no second notice.
    await sync.receive(server.history.slice(-1));
    expect([sync.state.value, told.archived]).toEqual(['archived', 0]);
  });

  it('opens the project again once it is unarchived, and leaves it when it goes to the trash', async () => {
    const { server, sync } = setup();
    server.archived = true;
    const { lifecycle, calls } = session(sync, server);
    await lifecycle.unarchive(ref);
    expect(calls).toEqual(['open']);
    await lifecycle.trash(ref);
    expect(calls.at(-1)).toMatch(/^trashed:26\.10\.2026 tarihine kadar geri yüklenebilir$/);
    expect(server.deleted).toBe(true);
  });

  it('names what it removes for good, and sends the rest of a metadata change from the version the rename made', async () => {
    const { server, sync } = setup();
    const { lifecycle, calls } = session(sync, server);
    const sent: CommandEnvelope[] = server.lifecycleLog;
    await lifecycle.purge(ref).catch(() => undefined);
    expect(sent.at(-1)!.input).toEqual({ confirmName: 'Ada 101' });
    // The open project's new name goes through its autosave; the rest from the version the server has after it.
    await lifecycle.updateMetadata(ref, { name: 'Ada 101 (revize)', tags: ['Kadıköy'] }, '1');
    expect(calls).toContain('rename:Ada 101 (revize)');
    const update = sent.at(-1)!;
    expect([update.commandName, update.input, update.expectedVersions]).toEqual(['project.metadata.update', { tags: ['Kadıköy'] }, { '@catalog': '1' }]);
    // Not the open project: one command, the name with it, from the version shown.
    const other = session(sync, server, false);
    await other.lifecycle.updateMetadata(ref, { name: 'Başka', projectType: 'gis' }, '7');
    expect([sent.at(-1)!.input, sent.at(-1)!.expectedVersions]).toEqual([{ name: 'Başka', projectType: 'gis' }, { '@catalog': '7' }]);
    expect(other.calls).toEqual([]);
  });
});
