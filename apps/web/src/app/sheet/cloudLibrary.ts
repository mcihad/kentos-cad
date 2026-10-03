import type { SheetTemplateAccess } from '../../contracts/generated/sheet/SheetTemplateAccess';
import type { SheetTemplateCandidates } from '../../contracts/generated/sheet/SheetTemplateCandidates';
import type { SheetTemplateChanged } from '../../contracts/generated/sheet/SheetTemplateChanged';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { TemplateGrantRole } from '../../contracts/generated/sheet/TemplateGrantRole';
import { DisposableStore } from '../../core/disposable';
import { uuidv7 } from '../../core/uuid';
import type { SheetEngine } from '../../product/sheet/engine';
import type { DeviceTemplateStore } from '../../product/sheet/templateStore';
import { HttpCloudApi } from '../cloud/api';
import type { AppContext } from '../context';
import { HttpTemplateApi, type TemplateCloudApi } from './cloudApi';
import { TemplateSync } from './templateSync';

/**
 * The account's template library in the cloud, as the sheets use it
 * (docs/sheet/design.md §13): the sync (templateSync.ts) started when the
 * engine is (never at the app's start: it plans with the engine), run again
 * when the account signs in, when the server answers again and when the
 * cloud's events say so, and stopped at sign-out; and what the gallery and
 * the share window ask of the cloud: “Buluta eşitle”, a change saved here,
 * a deletion, who a template is shared with, whom it may be shared with,
 * and sharing; and the organisations' libraries (design §13 “Kurum
 * şablonları”): where the account may publish, publishing one's own
 * template into an organisation (`sheet.template.publish`, on the
 * organisation's route), and putting its content into the organisation's
 * copy published from it before (“Kurumdakini güncelle”). A refusal comes
 * back in the server's own words.
 */

export interface LibraryHost {
  engine(): SheetEngine | null;
  readonly templates: DeviceTemplateStore;
  /** The templates on this device changed: the gallery's lists and the sheets' “Yeni sürüm var” follow. */
  libraryChanged(): void;
  /** Only the sync's state changed (a badge, the gallery's line). */
  libraryState(): void;
}

export const LIBRARY_TEXTS = {
  signIn: 'Bulut şablonları için önce bulut hesabınızla giriş yapın.',
  offline: 'Sunucuya şu an ulaşılamıyor: bulut şablonlarının son indirilen sürümleri kullanılıyor; bağlantı gelince eşitlenir.',
  syncing: 'Şablonlar eşitleniyor…',
  synced: (at: number) => `Şablonlar eşitlendi · ${new Date(at).toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit' })}`,
  uploaded: (name: string) => `“${name}” buluta eşitlendi: web ve masaüstünde aynı; artık paylaşılabilir.`,
  notUploaded: (name: string) => `“${name}” şimdi eşitlenemedi; bağlantı gelince kendiliğinden gider.`,
  deleted: (name: string) => `“${name}” silindi; bulut hesabınızdan da kalkıyor. Ondan yapılmış paftalar kalır.`,
  notOwner: 'Yalnız şablonun sahibi paylaşır ve siler.',
  orgDeleted: (name: string, org: string) => `“${name}” silindi; “${org}” kurumunun şablonlarından da kalkıyor. Ondan yapılmış paftalar kalır.`,
  published: (name: string, org: string) => `“${name}” “${org}” kurumunda yayımlandı: kurumun etkin üyeleri görür ve kullanır.`,
  republished: (name: string, org: string) => `“${name}” “${org}” kurumundaki kopyasının yeni revizyonu olarak eşitleniyor.`,
} as const;

/** An organisation a template may be published to, and its copy there when it was published before. */
export interface PublishTarget {
  readonly tenantId: string;
  readonly name: string;
  /** The organisation's copy published from this template, the account its publisher or an administrator: its id, name and revision. */
  readonly copy?: { readonly id: string; readonly name: string; readonly revision: number };
}

const httpOf = (ctx: AppContext) => (ctx.cloud.api instanceof HttpCloudApi ? ctx.cloud.api : new HttpCloudApi());

/** The signed-in account, or null. */
export function accountOf(ctx: AppContext): { id: string; name: string } | null {
  const me = ctx.cloud.me.value;
  return me && ctx.cloud.auth.value === 'signedIn' ? { id: me.user.id, name: me.user.displayName } : null;
}

export class CloudLibrary {
  readonly sync: TemplateSync;
  private readonly ctx: AppContext;
  private readonly host: LibraryHost;
  private readonly api: TemplateCloudApi;
  private readonly d = new DisposableStore();

  constructor(ctx: AppContext, host: LibraryHost, api: TemplateCloudApi = new HttpTemplateApi(httpOf(ctx))) {
    this.ctx = ctx;
    this.host = host;
    this.api = api;
    this.sync = new TemplateSync(api, {
      engine: () => host.engine(),
      store: host.templates,
      account: () => accountOf(ctx),
      personal: () => ctx.cloud.me.value?.memberships.find((m) => m.tenantKind === 'personal')?.tenantId ?? null,
      say: (text, kind) => ctx.log[kind](text),
      changed: () => host.libraryChanged(),
      unreachable: () => void ctx.server.check(),
    });
    this.d.add(this.sync.status.subscribe(() => host.libraryState()));
    this.d.add(this.sync.busy.subscribe(() => host.libraryState()));
    // Signed in: run and hear events; signed out: stop, and the lists hide that account's copies.
    this.d.add(
      ctx.cloud.me.subscribe(() => {
        if (accountOf(ctx)) this.sync.reconnected();
        else {
          this.sync.stop();
          this.sync.status.set({ state: 'signedOut' });
        }
        host.libraryChanged();
      }),
    );
    this.d.add(ctx.server.state.subscribe((s) => s === 'online' && this.sync.reconnected()));
    if (accountOf(ctx)) this.sync.reconnected();
  }

  /** Why cloud actions cannot run now (not signed in, the server away); null when they can. */
  why(): string | null {
    if (!accountOf(this.ctx)) return LIBRARY_TEXTS.signIn;
    if (this.sync.status.value.state === 'offline' || this.ctx.server.state.value === 'offline') return LIBRARY_TEXTS.offline;
    return null;
  }

  /** The line the gallery shows for the library now, or '' when there is nothing to say. */
  line(): string {
    const s = this.sync.status.value;
    if (s.state === 'signedOut') return '';
    if (s.state === 'offline') return LIBRARY_TEXTS.offline;
    if (s.state === 'syncing') return LIBRARY_TEXTS.syncing;
    if (s.state === 'failed') return s.reason;
    return LIBRARY_TEXTS.synced(s.at);
  }

  /** “Buluta eşitle”: a template of this device goes up; the id the cloud gave it, or null when it could not go now. */
  async upload(id: string, name: string): Promise<string | null> {
    const me = accountOf(this.ctx);
    if (!me) return (this.ctx.log.warn(LIBRARY_TEXTS.signIn), null);
    const r = await this.host.templates.get(id);
    if (!r || r.status !== 'ok') return null;
    if (!r.record.cloud) await this.host.templates.save(id, r.record.template, { account: me.id, revision: 0, changed: true, role: 'owner' });
    this.host.libraryChanged();
    await this.sync.run();
    const made = this.sync.created.get(id) ?? (r.record.cloud ? id : null);
    if (made) this.ctx.log.success(LIBRARY_TEXTS.uploaded(name));
    else this.ctx.log.warn(LIBRARY_TEXTS.notUploaded(name));
    return made;
  }

  /**
   * A template of the library saved here (“Şablon olarak kaydet” → güncelle):
   * changed, synced as soon as it can be. A conflict's copy saved again is the
   * user's own template from now on (its “Çakışma” goes).
   */
  async saved(id: string, template: Template): Promise<void> {
    const r = await this.host.templates.get(id);
    const was = r?.status === 'ok' ? r.record.cloud : undefined;
    const cloud = was && (({ conflictOf: _kept, ...rest }) => rest)(was);
    // The cloud gives the revision: the copy keeps the one it is based on until then.
    await this.host.templates.save(id, cloud ? { ...template, meta: { ...template.meta, revision: Math.max(cloud.revision, 1) } } : template, cloud ? { ...cloud, changed: true } : undefined);
    this.host.libraryChanged();
    if (cloud) void this.sync.run();
  }

  /**
   * Deletes a template: one of this device at once; one of the library everywhere, when the cloud hears of
   * it. An organisation's is deleted by the one who published it and the organisation's administrators.
   */
  async remove(id: string, name: string): Promise<boolean> {
    const r = await this.host.templates.get(id);
    if (!r || r.status !== 'ok') return false;
    const cloud = r.record.cloud;
    if (cloud && cloud.role !== 'owner' && !(cloud.organization && cloud.role === 'admin')) return (this.ctx.log.warn(LIBRARY_TEXTS.notOwner), false);
    if (!cloud || cloud.revision === 0) await this.host.templates.remove(id);
    else await this.host.templates.save(id, r.record.template, { ...cloud, deleted: true });
    this.host.libraryChanged();
    if (cloud) {
      this.ctx.log.success(cloud.organization ? LIBRARY_TEXTS.orgDeleted(name, cloud.organization.name) : LIBRARY_TEXTS.deleted(name));
      void this.sync.run();
    }
    return true;
  }

  /** The organisations of the account's library (the last list), by name; none before one was read. */
  organizations(): readonly { readonly tenantId: string; readonly name: string; readonly canPublish: boolean }[] {
    return this.sync.organizations.value ?? [];
  }

  /** Where a template of the account's own may be published: the organisations it may publish to, each with the copy published from it there before. */
  async publishTargets(id: string): Promise<PublishTarget[]> {
    const me = accountOf(this.ctx);
    const copies = me ? (await this.host.templates.list()).flatMap((r) => (r.status === 'ok' && r.record.cloud?.account === me.id && !r.record.cloud.deleted && r.record.cloud.publishedFrom === id && (r.record.cloud.role === 'owner' || r.record.cloud.role === 'admin') ? [r.record] : [])) : [];
    return this.organizations()
      .filter((o) => o.canPublish)
      .map((o) => {
        const copy = copies.find((r) => r.cloud?.organization?.tenantId === o.tenantId);
        return { tenantId: o.tenantId, name: o.name, ...(copy && { copy: { id: copy.id, name: (copy.template as Template).meta.name, revision: copy.cloud!.revision } }) };
      });
  }

  /**
   * “Kuruma yayımla”: a copy of one's own template, as the cloud has it, becomes a template of the
   * organisation's library (a new id, revision 1); the sync brings it here. Its id there; thrown with the
   * server's words when it cannot.
   */
  async publish(id: string, tenantId: string): Promise<string> {
    const answer = await this.api.command(tenantId, { name: 'sheet.template.publish', input: { templateId: id, tenantId } }, uuidv7());
    await this.sync.run();
    const r = await this.host.templates.get(answer.templateId);
    const name = r?.status === 'ok' ? (r.record.template as Template).meta.name : '';
    const org = this.organizations().find((o) => o.tenantId === tenantId)?.name ?? '';
    this.ctx.log.success(LIBRARY_TEXTS.published(name, org));
    return answer.templateId;
  }

  /**
   * “Kurumdakini güncelle”: the organisation's copy published from this template takes its content (its id
   * and revision stay the copy's) and is synced: a new revision there, by the revision it is based on (a
   * change there in between is a conflict, as for any template).
   */
  async republish(id: string, copyId: string): Promise<void> {
    const [own, copy] = await Promise.all([this.host.templates.get(id), this.host.templates.get(copyId)]);
    if (own?.status !== 'ok' || copy?.status !== 'ok' || !copy.record.cloud) throw new Error('Şablonun kurumdaki kopyası bu cihazda bulunamadı; önce eşitleyin.');
    const content = own.record.template as Template;
    const there = copy.record.template as Template;
    await this.host.templates.save(copyId, { ...content, meta: { ...content.meta, id: copyId, revision: there.meta.revision } }, { ...copy.record.cloud, changed: true });
    this.host.libraryChanged();
    this.ctx.log.info(LIBRARY_TEXTS.republished(content.meta.name, copy.record.cloud.organization?.name ?? ''));
    await this.sync.run();
  }

  access(id: string, signal?: AbortSignal): Promise<SheetTemplateAccess> {
    return this.api.access(id, signal);
  }

  candidates(id: string, query: string, signal?: AbortSignal): Promise<SheetTemplateCandidates> {
    return this.api.candidates(id, query, signal);
  }

  /** Shares with a person or changes their role; the list follows at the next sync (the cloud tells the owner too). */
  share(id: string, userId: string, role: TemplateGrantRole): Promise<SheetTemplateChanged> {
    return this.command({ name: 'sheet.template.share', input: { templateId: id, userId, role } });
  }

  unshare(id: string, userId: string): Promise<SheetTemplateChanged> {
    return this.command({ name: 'sheet.template.unshare', input: { templateId: id, userId } });
  }

  private async command(c: Parameters<TemplateCloudApi['command']>[1]): Promise<SheetTemplateChanged> {
    const personal = this.ctx.cloud.me.value?.memberships.find((m) => m.tenantKind === 'personal')?.tenantId;
    if (!personal) throw new Error('Hesabın kişisel alanı bulunamadı; sunucu bu sürümle uyuşmuyor olabilir.');
    const answer = await this.api.command(personal, c, uuidv7());
    void this.sync.run();
    return answer;
  }

  dispose(): void {
    this.sync.stop();
    this.d.dispose();
  }
}
