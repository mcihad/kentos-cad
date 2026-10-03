import type { SheetTemplateChanged } from '../../contracts/generated/sheet/SheetTemplateChanged';
import type { SheetTemplateEvent } from '../../contracts/generated/sheet/SheetTemplateEvent';
import type { SheetTemplateEventKind } from '../../contracts/generated/sheet/SheetTemplateEventKind';
import type { SheetTemplateSummary } from '../../contracts/generated/sheet/SheetTemplateSummary';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { TemplateGrantRole } from '../../contracts/generated/sheet/TemplateGrantRole';
import type { TemplateRole } from '../../contracts/generated/sheet/TemplateRole';
import { ApiFailure } from '../cloud/api';
import type { TemplateCloudApi, TemplateCommand } from './cloudApi';

/**
 * For tests only: the cloud's template library in memory, with the server's
 * rules as tasks-rust.md and .run/sheet-engine-ready state them: ids,
 * revisions and dates written by the server; `update` and `delete` refused
 * with 409 when the revision is not the latest; writes by the owner (and an
 * editor's update) only, 403 otherwise; another's template 404 alike for a
 * missing one; sharing only with people of a common organisation; one event
 * per person who should hear a change; a key used again with the same
 * request replayed, with another refused (422). `offline` makes every
 * request fail as the network does; `loseNextAnswer` applies a command and
 * then fails as a lost answer would.
 *
 * Organisations (“KURUM ŞABLONLARI”): a member sees its organisations'
 * libraries (one who left sees none); the one who published a template is
 * its `owner` while they may still publish, the organisation's owner and
 * administrators are `admin`, everyone else `viewer`; those who may
 * publish are the owner, administrators and project managers. A command
 * goes to its library's route (another gives 422; a route of an
 * organisation one is not a member of 404); an organisation's template is
 * not shared one by one (422); `publish` copies one's own template into
 * the organisation as a template of its own. Every change of one is told
 * to every member.
 */

interface Person {
  readonly id: string;
  readonly name: string;
  readonly email: string;
  readonly orgs: readonly string[];
}

/** A member's role in an organisation (the server's `TenantRole`). */
export type OrgRole = 'owner' | 'admin' | 'project_manager' | 'editor' | 'viewer';

interface Org {
  readonly id: string;
  readonly name: string;
  readonly members: Map<string, OrgRole>;
}

interface Kept {
  readonly id: string;
  /** Who made it; of an organisation's, who published it. */
  readonly owner: string;
  /** The organisation whose library it is in; none: its owner's personal space. */
  readonly org?: string;
  readonly publishedFrom?: string;
  revision: number;
  content: Template;
  deleted: boolean;
  readonly grants: Map<string, TemplateGrantRole>;
  created: string;
  updated: string;
}

const failure = (status: number, error: string, message: string) => new ApiFailure(status, { error, message }, message);
const NOT_FOUND = () => failure(404, 'not_found', 'Şablon bulunamadı.');

export class FakeTemplateCloud {
  readonly people = new Map<string, Person>();
  readonly orgs = new Map<string, Org>();
  readonly templates = new Map<string, Kept>();
  readonly events: (SheetTemplateEvent & { readonly user: string })[] = [];
  offline = false;
  loseNextAnswer = false;
  private seq = 0;
  private ids = 0;
  private clock = 0;
  private readonly keys = new Map<string, { request: string; answer: SheetTemplateChanged }>();
  private waiters: (() => void)[] = [];

  person(id: string, name: string, orgs: readonly string[] = ['ornek-buro']): this {
    this.people.set(id, { id, name, email: `${id}@ornek.gov.tr`, orgs });
    return this;
  }

  /** An organisation and its active, seated members with their roles. */
  org(id: string, name: string, members: Record<string, OrgRole>): this {
    this.orgs.set(id, { id, name, members: new Map(Object.entries(members)) });
    return this;
  }

  /** A member leaves the organisation: they see its library no more, and hear nothing of it. */
  leave(org: string, user: string): void {
    this.orgs.get(org)?.members.delete(user);
  }

  private member(org: string, user: string): OrgRole | null {
    return this.orgs.get(org)?.members.get(user) ?? null;
  }

  private canPublish(org: string, user: string): boolean {
    const r = this.member(org, user);
    return r === 'owner' || r === 'admin' || r === 'project_manager';
  }

  /** The library as one person's client sees it. */
  as(user: string): TemplateCloudApi {
    const net = <T>(f: () => T): Promise<T> => (this.offline ? Promise.reject(failure(0, 'network', 'Sunucuya ulaşılamadı.')) : Promise.resolve().then(f));
    return {
      list: () =>
        net(() => ({
          templates: [...this.templates.values()].filter((t) => !t.deleted && !t.org && this.role(t, user)).map((t) => this.summary(t, user)),
          organizations: [...this.orgs.values()]
            .filter((o) => o.members.has(user))
            .sort((a, b) => a.name.localeCompare(b.name, 'tr'))
            .map((o) => ({ tenantId: o.id, name: o.name, canPublish: this.canPublish(o.id, user), templates: [...this.templates.values()].filter((t) => !t.deleted && t.org === o.id).map((t) => this.summary(t, user)) })),
        })),
      detail: (id) =>
        net(() => {
          const t = this.visible(id, user);
          return { summary: this.summary(t, user), content: structuredClone(t.content) };
        }),
      access: (id) =>
        net(() => {
          const t = this.owned(id, user);
          return { ownerId: t.owner, ownerName: this.people.get(t.owner)!.name, grants: [...t.grants].map(([u, role]) => ({ userId: u, displayName: this.people.get(u)!.name, email: this.people.get(u)!.email, role, grantedByName: this.people.get(t.owner)!.name, createdAt: t.created })) };
        }),
      candidates: (id, q) =>
        net(() => {
          this.owned(id, user);
          if (q.trim().length < 2) throw failure(422, 'invalid', 'En az iki harf yazın.');
          const me = this.people.get(user)!;
          const words = q.toLocaleLowerCase('tr').split(/\s+/).filter(Boolean);
          return {
            candidates: [...this.people.values()]
              .filter((p) => p.id !== user && p.orgs.some((o) => me.orgs.includes(o)) && words.every((w) => `${p.name} ${p.email}`.toLocaleLowerCase('tr').includes(w)))
              .map((p) => ({ userId: p.id, displayName: p.name, email: p.email })),
          };
        }),
      events: (after, wait, signal) => {
        if (this.offline) return Promise.reject(failure(0, 'network', 'Sunucuya ulaşılamadı.'));
        const page = () => {
          const mine = this.events.filter((e) => e.user === user && Number(e.seq) > Number(after));
          return { events: mine.map(({ user: _u, ...e }) => e), next: mine.length ? mine[mine.length - 1].seq : after };
        };
        const now = page();
        if (now.events.length || !wait) return Promise.resolve(now);
        return new Promise((resolve, reject) => {
          const go = () => resolve(page());
          this.waiters.push(go);
          signal?.addEventListener('abort', () => {
            this.waiters = this.waiters.filter((w) => w !== go);
            reject(failure(499, 'aborted', 'İstek iptal edildi.'));
          });
        });
      },
      command: (tenant, c, key) => net(() => this.command(user, tenant, c, key)),
    };
  }

  private role(t: Kept, user: string): TemplateRole | null {
    if (t.org) {
      const m = this.member(t.org, user);
      if (!m) return null;
      if (t.owner === user && this.canPublish(t.org, user)) return 'owner';
      return m === 'owner' || m === 'admin' ? 'admin' : 'viewer';
    }
    return t.owner === user ? 'owner' : (t.grants.get(user) ?? null);
  }

  private visible(id: string, user: string): Kept {
    const t = this.templates.get(id);
    if (!t || t.deleted || !this.role(t, user)) throw NOT_FOUND();
    return t;
  }

  private owned(id: string, user: string): Kept {
    const t = this.visible(id, user);
    const role = this.role(t, user);
    if (role !== 'owner' && role !== 'admin') throw failure(403, 'forbidden', t.org ? 'Kurum şablonunu yalnız yayımlayan ve kurum yöneticileri değiştirir.' : 'Bunu yalnız şablonun sahibi yapabilir.');
    return t;
  }

  /** A template's command on its library's own route, else 422 (the server's rule). */
  private routed(t: Kept, user: string, tenant: string): Kept {
    if (tenant !== (t.org ?? `kisisel-${user}`) && !(t.org === undefined && tenant === `kisisel-${t.owner}`)) throw failure(422, 'wrong_route', 'Şablonun komutu kendi kitaplığının yoluna gider.');
    return t;
  }

  private summary(t: Kept, user: string): SheetTemplateSummary {
    const m = t.content.meta;
    return {
      id: t.id,
      name: m.name,
      description: m.description,
      category: m.category,
      tags: m.tags,
      papers: m.papers,
      workspaces: m.workspaces,
      projectTypes: m.projectTypes,
      revision: t.revision,
      sha256: '',
      size: JSON.stringify(t.content).length,
      role: this.role(t, user)!,
      ownerId: t.owner,
      ownerName: this.people.get(t.owner)!.name,
      shared: t.grants.size > 0,
      createdAt: t.created,
      updatedAt: t.updated,
      ...(t.org && { organization: { tenantId: t.org, name: this.orgs.get(t.org)!.name } }),
      ...(t.publishedFrom && { publishedFrom: t.publishedFrom }),
    };
  }

  private tell(t: Kept, kind: SheetTemplateEventKind, actor: string, extra: readonly string[] = []): void {
    const who = t.org ? [...(this.orgs.get(t.org)?.members.keys() ?? [])] : [t.owner, ...t.grants.keys(), ...extra];
    for (const user of new Set(who)) this.events.push({ seq: String(++this.seq), templateId: t.id, revision: t.revision, kind, actor, user });
    const waiting = this.waiters;
    this.waiters = [];
    waiting.forEach((w) => w());
  }

  private stamp(): string {
    return new Date(Date.UTC(2026, 9, 3, 12, 0, this.clock++)).toISOString();
  }

  private command(user: string, tenant: string, c: TemplateCommand, key: string): SheetTemplateChanged {
    // The personal space's route, or an organisation's one is a member of; another is not found.
    if (tenant !== `kisisel-${user}` && !this.member(tenant, user)) throw NOT_FOUND();
    const request = JSON.stringify(c);
    const seen = this.keys.get(`${user}|${key}`);
    if (seen) {
      if (seen.request !== request) throw failure(422, 'idempotency_key_reused', 'Bu anahtar başka bir istekte kullanılmış.');
      return { ...seen.answer, replayed: true };
    }
    const answer = this.apply(user, tenant, c);
    this.keys.set(`${user}|${key}`, { request, answer });
    if (this.loseNextAnswer) {
      this.loseNextAnswer = false;
      throw failure(0, 'network', 'Sunucuya ulaşılamadı.');
    }
    return answer;
  }

  private apply(user: string, tenant: string, c: TemplateCommand): SheetTemplateChanged {
    const done = (t: Kept, changed = true): SheetTemplateChanged => ({ templateId: t.id, revision: t.revision, deleted: t.deleted, changed, replayed: false });
    const org = tenant === `kisisel-${user}` ? undefined : tenant;
    const make = (content: Template, extra: Pick<Kept, 'org' | 'publishedFrom'> = {}): Kept => {
      const id = `00000000-0000-7000-8000-${String(++this.ids).padStart(12, '0')}`;
      const at = this.stamp();
      const t: Kept = { id, owner: user, ...extra, revision: 1, content: { ...structuredClone(content), meta: { ...content.meta, id, revision: 1, created: at, updated: at } }, deleted: false, grants: new Map(), created: at, updated: at };
      this.templates.set(id, t);
      this.tell(t, 'created', user);
      return t;
    };
    switch (c.name) {
      case 'sheet.template.create': {
        if (org && !this.canPublish(org, user)) throw failure(403, 'forbidden', 'Bu kurumda şablon yayımlayamazsınız.');
        return done(make(c.input.content, org ? { org } : {}));
      }
      case 'sheet.template.publish': {
        if (!org || c.input.tenantId !== org) throw failure(422, 'wrong_route', 'Yayımlama kurumun yoluna gider.');
        if (!this.canPublish(org, user)) throw failure(403, 'forbidden', 'Bu kurumda şablon yayımlayamazsınız.');
        const from = this.visible(c.input.templateId, user);
        if (from.owner !== user || from.org) throw failure(403, 'forbidden', 'Yalnız kendi şablonunuzu yayımlarsınız.');
        return done(make(from.content, { org, publishedFrom: from.id }));
      }
      case 'sheet.template.update': {
        const t = this.routed(this.visible(c.input.templateId, user), user, tenant);
        if (this.role(t, user) === 'viewer') throw failure(403, 'forbidden', 'Bu şablonu yalnız görüntüleyebilirsiniz.');
        if (c.input.expectedRevision !== t.revision) throw new ApiFailure(409, { error: 'conflict', message: 'Şablon siz okuduktan sonra değişmiş.', revision: String(t.revision) }, '409');
        t.revision++;
        t.updated = this.stamp();
        t.content = { ...structuredClone(c.input.content), meta: { ...c.input.content.meta, id: t.id, revision: t.revision, updated: t.updated } };
        this.tell(t, 'updated', user);
        return done(t);
      }
      case 'sheet.template.delete': {
        const t = this.routed(this.owned(c.input.templateId, user), user, tenant);
        if (c.input.expectedRevision !== undefined && c.input.expectedRevision !== t.revision) throw new ApiFailure(409, { error: 'conflict', message: 'Şablon siz okuduktan sonra değişmiş.' }, '409');
        t.deleted = true;
        this.tell(t, 'deleted', user);
        return done(t);
      }
      case 'sheet.template.share': {
        const t = this.routed(this.owned(c.input.templateId, user), user, tenant);
        if (t.org) throw failure(422, 'org_template', 'Kurum şablonu tek tek paylaşılmaz: kurumun bütün etkin üyeleri görür.');
        const to = this.people.get(c.input.userId);
        const me = this.people.get(user)!;
        if (!to || to.id === user || !to.orgs.some((o) => me.orgs.includes(o))) throw failure(422, 'invalid', 'Bu kişiyle paylaşılamaz: yalnız ortak kurumlarınızın etkin üyeleriyle paylaşabilirsiniz.');
        const changed = t.grants.get(to.id) !== c.input.role;
        t.grants.set(to.id, c.input.role);
        if (changed) this.tell(t, 'shared', user);
        return done(t, changed);
      }
      case 'sheet.template.unshare': {
        const t = this.routed(this.owned(c.input.templateId, user), user, tenant);
        if (t.org) throw failure(422, 'org_template', 'Kurum şablonu tek tek paylaşılmaz: kurumun bütün etkin üyeleri görür.');
        const had = t.grants.delete(c.input.userId);
        if (had) this.tell(t, 'unshared', user, [c.input.userId]);
        return done(t, had);
      }
    }
  }
}
