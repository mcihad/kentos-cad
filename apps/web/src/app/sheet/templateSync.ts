import type { LocalTemplate } from '../../contracts/generated/sheet/LocalTemplate';
import type { SheetTemplateSummary } from '../../contracts/generated/sheet/SheetTemplateSummary';
import type { SyncAction } from '../../contracts/generated/sheet/SyncAction';
import type { Template } from '../../contracts/generated/sheet/Template';
import { Signal } from '../../core/signal';
import { uuidv7 } from '../../core/uuid';
import { errorText, type SheetEngine } from '../../product/sheet/engine';
import type { DeviceTemplateStore, StoredTemplate } from '../../product/sheet/templateStore';
import { ApiFailure } from '../cloud/api';
import type { TemplateCloudApi, TemplateCommand } from './cloudApi';
import { cloudOf, digest, listChanged, nameOf, status, SYNC_TEXTS, templateOf, transient, withMeta } from './templateSyncParts';

export { SYNC_TEXTS } from './templateSyncParts';

/**
 * The template library's sync (docs/sheet/design.md §13): the account's
 * copies on this device against the cloud's list (its own, those shared
 * with it, and its organisations' libraries), planned by the engine
 * (`planSync`: pure, the rules of design §13's table, no path losing data
 * silently) and carried out here, one action after another. A command goes
 * to the route of the library its template is in: an organisation's
 * template's to that organisation, the rest to the personal space; a new
 * template and a conflict's copy always to the personal space (an
 * organisation's library takes no template by itself):
 *
 * - **download**: the revision's content, read by the engine, becomes this
 *   device's copy (the cache used offline);
 * - **upload / create**: the copy goes up (`sheet.template.update` with the
 *   revision it is based on, `…create` for one never uploaded, which takes
 *   the id the cloud gives it; its old id is kept for the sheets made from
 *   it);
 * - **uploadCopy**: a conflict: the copy changed here goes up as a new
 *   template “… (bu cihazdaki kopya)”, then the cloud's is downloaded;
 * - **removeLocal / keepLocal / deleteRemote / keepRemote**: as the plan says,
 *   with its words to the user.
 *
 * A command's idempotency key is derived from what it sends, so a retry after
 * an answer that never came is the same request (the server replays it). A
 * network failure stops the run and leaves everything as it was; the next
 * run (on reconnect, on an event, on a change here) picks it up. The cloud's
 * change events come by a long poll (`events`, tasks-rust.md Sapmalar 30);
 * one heard is a reason to run again, never content.
 */

export type SyncStatus =
  | { readonly state: 'signedOut' }
  /** The server could not be reached: the cached copies are used; it runs again on reconnect. */
  | { readonly state: 'offline' }
  | { readonly state: 'syncing' }
  | { readonly state: 'synced'; readonly at: number }
  /** A refusal that is not the network's (the server's words). */
  | { readonly state: 'failed'; readonly reason: string };

export interface SyncHost {
  engine(): SheetEngine | null;
  readonly store: DeviceTemplateStore;
  /** The signed-in account, or null. */
  account(): { readonly id: string; readonly name: string } | null;
  /** The account's personal space, where its template commands go; null when the server named none. */
  personal(): string | null;
  /** Says something to the user (the message log). */
  say(text: string, kind: 'info' | 'warn' | 'error' | 'success'): void;
  /** The templates on this device changed (the gallery's lists, the sheets' “Yeni sürüm var”). */
  changed(): void;
  /** The network failed: let the app re-check whether the server answers. */
  unreachable(): void;
}

/** Waits after a network failure (ms), the last one again and again. */
export const RETRY_MS = [5_000, 15_000, 30_000, 60_000] as const;
/**
 * Seconds an events request may wait for one (the server's own limit is 25).
 * Short on the web: a proxy in front of the server may cut a request that
 * long sooner (the dev server's cuts one at 10 s, and a cut one would look
 * like the server being away); a change still arrives within a second.
 */
export const EVENTS_WAIT_S = 8;
const EVENTS_PAGE = 500;
/** A run asked for by an event waits this long for others (one run for a burst). */
const EVENT_SETTLE_MS = 300;

export interface Timers {
  set(fn: () => void, ms: number): unknown;
  clear(handle: unknown): void;
}
const realTimers: Timers = { set: (fn, ms) => setTimeout(fn, ms), clear: (h) => clearTimeout(h as ReturnType<typeof setTimeout>) };

/** An organisation whose library the account sees, as the last list gave it. */
export interface LibraryOrganization {
  readonly tenantId: string;
  readonly name: string;
  /** The account may publish into it (“Kuruma yayımla…”): its owner, administrators and those who may create projects. */
  readonly canPublish: boolean;
}

export class TemplateSync {
  readonly status = new Signal<SyncStatus>({ state: 'signedOut' });
  /** The organisations of the account's library, by name, as the last list gave them; null before one (and when signed out). */
  readonly organizations = new Signal<readonly LibraryOrganization[] | null>(null, (a, b) => JSON.stringify(a) === JSON.stringify(b));
  /** The templates an action is under way for (“Eşitleniyor”). */
  readonly busy = new Signal<ReadonlySet<string>>(new Set(), () => false);
  /** Ids the cloud gave the templates created from this device in this session: local id → cloud id. */
  readonly created = new Map<string, string>();
  private readonly api: TemplateCloudApi;
  private readonly host: SyncHost;
  private readonly timers: Timers;
  private running: Promise<void> | null = null;
  private again = false;
  private watching: AbortController | null = null;
  private cursor: string | null = null;
  private retry: unknown = null;
  private failures = 0;
  private settle: unknown = null;

  constructor(api: TemplateCloudApi, host: SyncHost, timers: Timers = realTimers) {
    this.api = api;
    this.host = host;
    this.timers = timers;
  }

  /** One sync, now; a call during one makes another follow it. Resolves when none is left to run. */
  run(): Promise<void> {
    if (this.running) {
      this.again = true;
      return this.running;
    }
    this.running = (async () => {
      do {
        this.again = false;
        await this.once();
      } while (this.again);
    })().finally(() => (this.running = null));
    return this.running;
  }

  /** Starts hearing the cloud's events (after a first sync); a second call while it hears does nothing. */
  start(): void {
    if (this.watching || !this.host.account()) return;
    const abort = new AbortController();
    this.watching = abort;
    void this.listen(abort.signal);
  }

  /** Stops hearing events and retrying (signed out, the service gone). */
  stop(): void {
    this.watching?.abort();
    this.watching = null;
    this.cursor = null;
    if (this.retry !== null) this.timers.clear(this.retry);
    if (this.settle !== null) this.timers.clear(this.settle);
    this.retry = this.settle = null;
  }

  /** The connection came back: run now, and hear events again. */
  reconnected(): void {
    if (!this.host.account()) return;
    this.failures = 0;
    if (this.retry !== null) this.timers.clear(this.retry);
    this.retry = null;
    void this.run();
    this.start();
  }

  // ── One run ─────────────────────────────────────────────────────

  private async once(): Promise<void> {
    const engine = this.host.engine();
    const me = this.host.account();
    if (!engine) return;
    if (!me) {
      this.organizations.set(null);
      return this.status.set({ state: 'signedOut' });
    }
    let list: SheetTemplateSummary[];
    try {
      const answer = await this.api.list();
      // The remote list is the account's own and shared ones and every organisation's library.
      const orgs = answer.organizations ?? [];
      list = [...answer.templates, ...orgs.flatMap((o) => o.templates)];
      this.organizations.set(orgs.map((o) => ({ tenantId: o.tenantId, name: o.name, canPublish: o.canPublish })));
    } catch (e) {
      return this.failed(e, SYNC_TEXTS.listFailed(errorText(e)));
    }
    const records = new Map<string, StoredTemplate>();
    for (const r of await this.host.store.list()) if (r.status === 'ok' && r.record.cloud?.account === me.id) records.set(r.record.id, r.record);
    const summaries = new Map(list.map((s) => [s.id, s]));
    const local: LocalTemplate[] = [...records.values()].map((r) => ({
      id: r.id,
      name: nameOf(r.template),
      ...(r.cloud!.revision > 0 && { baseRevision: r.cloud!.revision }),
      dirty: r.cloud!.changed || r.cloud!.revision === 0,
      deleted: !!r.cloud!.deleted,
    }));
    const plan = engine.planSync(
      local,
      list.map((s) => ({ id: s.id, name: s.name, revision: s.revision, deleted: false })),
    );
    let touched = false;
    // “Eşitleniyor” only while something goes up or comes down (a look at the list alone says nothing).
    if (plan.actions.length) this.status.set({ state: 'syncing' });
    for (const a of plan.actions) {
      this.mark(a.id, true);
      try {
        touched = (await this.act(a, engine, me.id, records)) || touched;
      } catch (e) {
        if (transient(e)) {
          this.mark(a.id, false);
          if (touched) this.host.changed();
          return this.failed(e, '');
        }
        if (status(e) === 409) this.again = true;
        else this.host.say(SYNC_TEXTS.actionFailed(records.get(a.id) ? nameOf(records.get(a.id)!.template) : (summaries.get(a.id)?.name ?? a.id), errorText(e)), 'error');
      } finally {
        this.mark(a.id, false);
      }
    }
    // What the list says of the others (their role, owner, sharing) is kept with them; they are clean (else the plan acts).
    for (const r of records.values()) {
      const s = summaries.get(r.id);
      const c = r.cloud!;
      if (!s || c.deleted || c.changed || plan.actions.some((a) => a.id === r.id)) continue;
      const next = cloudOf(me.id, s, c);
      if (listChanged(next, c)) {
        await this.host.store.save(r.id, r.template, next);
        touched = true;
      }
    }
    this.failures = 0;
    this.status.set({ state: 'synced', at: Date.now() });
    if (touched) this.host.changed();
  }

  /** Carries out one action; true when this device's templates changed. */
  private async act(a: SyncAction, engine: SheetEngine, account: string, records: Map<string, StoredTemplate>): Promise<boolean> {
    const r = records.get(a.id);
    const store = this.host.store;
    switch (a.type) {
      case 'download': {
        const d = await this.api.detail(a.id);
        const t = engine.validateTemplate(JSON.stringify(d.content));
        await store.save(a.id, t, cloudOf(account, d.summary, r?.cloud));
        return true;
      }
      case 'upload': {
        if (!r) return false;
        const content = templateOf(r, a.id);
        try {
          const res = await this.command({ name: 'sheet.template.update', input: { templateId: a.id, expectedRevision: a.expectedRevision, content } }, `sheet-tpl-update-${a.id}-${a.expectedRevision}-${await digest(JSON.stringify(content))}`, r.cloud?.organization?.tenantId);
          await store.save(a.id, withMeta(content, a.id, res.revision), { ...r.cloud!, revision: res.revision, changed: false });
        } catch (e) {
          // No right to write any more (the owner made it view-only): the changes stay here as a template of
          // their own, and this device's copy goes back to the cloud's.
          if (status(e) !== 403) throw e;
          const copy = `${nameOf(r.template)} (bu cihazdaki kopya)`;
          const id = uuidv7();
          await store.save(id, { ...content, meta: { ...content.meta, id, name: copy } });
          const d = await this.api.detail(a.id);
          await store.save(a.id, engine.validateTemplate(JSON.stringify(d.content)), cloudOf(account, d.summary, r.cloud));
          this.host.say(SYNC_TEXTS.readOnly(nameOf(r.template), copy), 'warn');
        }
        return true;
      }
      case 'create': {
        if (!r) return false;
        const content = templateOf(r, a.id);
        const res = await this.command({ name: 'sheet.template.create', input: { content } }, `sheet-tpl-create-${a.id}-${await digest(JSON.stringify(content))}`);
        await store.save(res.templateId, withMeta(content, res.templateId, res.revision), { account, revision: res.revision, changed: false, role: 'owner', formerIds: [...(r.cloud?.formerIds ?? []), a.id], formerRevision: content.meta.revision });
        if (res.templateId !== a.id) await store.remove(a.id);
        this.created.set(a.id, res.templateId);
        return true;
      }
      case 'uploadCopy': {
        // A conflict's copy goes to the personal space, an organisation's template's too.
        if (!r) return false;
        const own = templateOf(r, a.id);
        const content: Template = { ...own, meta: { ...own.meta, name: a.name } };
        const res = await this.command({ name: 'sheet.template.create', input: { content } }, `sheet-tpl-copy-${a.id}-${r.cloud!.revision}-${await digest(JSON.stringify(content))}`);
        await store.save(res.templateId, withMeta(content, res.templateId, res.revision), { account, revision: res.revision, changed: false, role: 'owner', conflictOf: a.id });
        this.host.say(SYNC_TEXTS.conflict(nameOf(r.template), a.name), 'warn');
        return true;
      }
      case 'removeLocal': {
        if (!r) return false;
        await store.remove(a.id);
        const org = r.cloud!.organization;
        if (!r.cloud!.deleted) this.host.say(org ? SYNC_TEXTS.orgGone(nameOf(r.template), org.name) : r.cloud!.role === 'owner' ? SYNC_TEXTS.removed(nameOf(r.template)) : SYNC_TEXTS.unshared(nameOf(r.template)), 'info');
        return true;
      }
      case 'keepLocal': {
        if (!r) return false;
        // A template of this device only now (its cloud one is gone): kept as it is, said.
        await store.save(a.id, r.template);
        this.host.say(a.message, 'warn');
        return true;
      }
      case 'deleteRemote': {
        try {
          await this.command({ name: 'sheet.template.delete', input: { templateId: a.id, expectedRevision: a.revision } }, `sheet-tpl-delete-${a.id}-${a.revision}`, r?.cloud?.organization?.tenantId);
        } catch (e) {
          // Gone already: nothing to do. Not ours to delete (shared with us): it comes back on the next run.
          if (status(e) === 403) this.again = true;
          else if (status(e) !== 404) throw e;
        }
        await store.remove(a.id);
        return true;
      }
      case 'keepRemote':
        this.host.say(a.message, 'warn');
        return false;
    }
    return false;
  }

  /** A command on its library's route: an organisation's (`tenant`), else the personal space. */
  private command(c: TemplateCommand, key: string, tenant?: string) {
    if (tenant) return this.api.command(tenant, c, key);
    const personal = this.host.personal();
    if (!personal) throw new ApiFailure(422, { error: 'no_personal_space', message: SYNC_TEXTS.noSpace }, SYNC_TEXTS.noSpace);
    return this.api.command(personal, c, key);
  }

  private mark(id: string, on: boolean): void {
    const next = new Set(this.busy.value);
    if (on) next.add(id);
    else next.delete(id);
    this.busy.set(next);
  }

  private failed(e: unknown, reason: string): void {
    if (status(e) === 401) return this.status.set({ state: 'signedOut' });
    if (!transient(e)) return this.status.set({ state: 'failed', reason: reason || errorText(e) });
    this.status.set({ state: 'offline' });
    this.host.unreachable();
    this.retryLater();
  }

  private retryLater(): void {
    if (this.retry !== null) return;
    const ms = RETRY_MS[Math.min(this.failures++, RETRY_MS.length - 1)];
    this.retry = this.timers.set(() => {
      this.retry = null;
      void this.run();
      this.start();
    }, ms);
  }

  // ── Events ──────────────────────────────────────────────────────

  /** The cursor at the newest event first (so nothing between it and the sync is missed), a sync, then the long poll. */
  private async listen(signal: AbortSignal): Promise<void> {
    try {
      if (this.cursor === null) {
        let after = '0';
        for (;;) {
          const page = await this.api.events(after, 0, signal, EVENTS_PAGE);
          after = page.next;
          if (page.events.length < EVENTS_PAGE) break;
        }
        this.cursor = after;
        await this.run();
      }
      while (!signal.aborted && this.host.account()) {
        const page = await this.api.events(this.cursor!, EVENTS_WAIT_S, signal);
        this.cursor = page.next;
        if (page.events.length) this.soon();
      }
    } catch (e) {
      if (signal.aborted) return;
      if (this.watching?.signal === signal) this.watching = null;
      if (status(e) === 401) return this.status.set({ state: 'signedOut' });
      this.failed(e, '');
      if (!transient(e)) this.retryLater();
    }
  }

  /** A run soon: events come in bursts (one change tells the owner and everyone it is shared with). */
  private soon(): void {
    if (this.settle !== null) return;
    this.settle = this.timers.set(() => {
      this.settle = null;
      void this.run();
    }, EVENT_SETTLE_MS);
  }
}
