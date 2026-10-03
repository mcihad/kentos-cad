import { Signal } from '../../core/signal';
import { errorText, type SheetEngine } from '../../product/sheet/engine';
import { fromBase64, type AssetStore } from '../../product/sheet/store';
import type { DeviceTemplateStore, StoredTemplate } from '../../product/sheet/templateStore';
import { cardOf, type GallerySection, type ProviderState, type TemplateBadge, type TemplateCard, type TemplateProvider } from '../../product/sheet/templates';
import type { Template } from '../../contracts/generated/sheet/Template';

/**
 * The template gallery's sources (docs/sheet/design.md §13): the system's,
 * which the engine gives (its ten templates are kentos-sheet's data); the
 * templates kept on this device, each read and checked by the engine
 * (`validateTemplate`: an older one migrated, a damaged one refused with its
 * reason, never half read), split in three: “Benim” (the ones made here and
 * the account's own in the cloud), “Kurumum” (the cached copies of the
 * libraries of the organisations it is an active member of) and “Benimle
 * paylaşılanlar” (those shared with it). A copy of the cloud's library
 * belongs to an account: another
 * account signed in on this browser does not see it; with no answer from
 * the server (offline) the last account's copies are shown, so they can
 * be used; after a sign-out, none.
 */

export const PROVIDER_TEXTS = {
  engineFailed: (why: string) => `Şablonlar listelenemedi: ${why}`,
  deviceUnreadable: (why: string) => `Bu cihazın şablonları okunamadı: ${why}. Sayfayı yenileyip yeniden deneyin.`,
  deviceRefused: (name: string, why: string) => `Bu cihazdaki “${name}” şablonu okunamadı: ${why} Kayıt olduğu gibi korunuyor.`,
  orgSignIn: 'Kurum şablonları için bulut hesabınızla giriş yapın.',
  noOrganisation: 'Bir kurumun etkin üyesi değilsiniz: kurum şablonlarını kurumun etkin, koltuklu üyeleri görür.',
  orgOffline: 'Sunucuya ulaşılamıyor: kurum şablonları bağlantı gelince görünür.',
  sharedSignIn: 'Sizinle paylaşılan şablonlar için bulut hesabınızla giriş yapın.',
  sharedOffline: 'Sunucuya ulaşılamıyor: sizinle paylaşılan şablonlar bağlantı gelince görünür.',
} as const;

/** The program's templates, read-only (`sys:` ids), from the engine. */
export class SystemProvider implements TemplateProvider {
  readonly section: GallerySection = 'system';
  readonly state = new Signal<ProviderState>({ state: 'loading' });
  readonly cards = new Signal<readonly TemplateCard[]>([]);
  private readonly engine: () => Promise<SheetEngine>;

  constructor(engine: () => Promise<SheetEngine>) {
    this.engine = engine;
  }

  async refresh(): Promise<void> {
    try {
      const engine = await this.engine();
      if (!this.cards.value.length) this.cards.set(engine.systemTemplates().map((t) => cardOf(t, 'system', ['system'])));
      this.state.set({ state: 'ready' });
    } catch (e) {
      this.state.set({ state: 'unavailable', reason: PROVIDER_TEXTS.engineFailed(errorText(e)) });
    }
  }
}

/** Whose cloud copies the lists show: the signed-in account's (an id), the last one's (undefined: no answer yet), or none (null: signed out). */
export type LibraryViewer = () => string | null | undefined;

/** What the lists read of the sync: the templates it is busy with. */
export type LibraryBusy = () => ReadonlySet<string>;

/**
 * A record's badges: on this device only, or its place in the cloud and the account's role; an
 * organisation's template its organisation first, then the account's part in it (published it,
 * administers it; a plain member none), then how it stands with the cloud.
 */
export function libraryBadges(r: StoredTemplate, busy: ReadonlySet<string>): TemplateBadge[] {
  const c = r.cloud;
  if (!c) return ['device'];
  const out: TemplateBadge[] = [];
  if (c.organization) {
    out.push('org');
    if (c.role === 'owner') out.push('published');
    else if (c.role === 'admin') out.push('admin');
  } else if (c.role === 'viewer' || c.role === 'editor') out.push(c.role);
  out.push(busy.has(r.id) ? 'syncing' : c.changed || c.revision === 0 ? 'unsynced' : 'synced');
  if (c.conflictOf) out.push('conflict');
  if (c.role === 'owner' && c.shared && !c.organization) out.push('shared');
  return out;
}

/** The organisations the account's library has (the last list the cloud gave): null when none was read yet. */
export type LibraryOrganizations = () => readonly { readonly tenantId: string; readonly name: string; readonly canPublish: boolean }[] | null;

/** The gallery section a record belongs to. */
export function sectionOfRecord(r: StoredTemplate): 'mine' | 'org' | 'shared' {
  if (r.cloud?.organization) return 'org';
  return r.cloud?.role === 'viewer' || r.cloud?.role === 'editor' ? 'shared' : 'mine';
}

/** The records a viewer sees: this device's own, and the copies of one account's library (deleted ones wait unseen). */
export function visibleRecords(records: readonly StoredTemplate[], viewer: string | null | undefined): StoredTemplate[] {
  const last = viewer === undefined ? [...records].filter((r) => r.cloud).sort((a, b) => b.saved - a.saved)[0]?.cloud?.account : viewer;
  return records.filter((r) => !r.cloud || (!!last && r.cloud.account === last && !r.cloud.deleted));
}

/**
 * The templates kept on this device for one gallery section: “Benim”
 * (made here, or the account's own in the cloud) or “Benimle paylaşılanlar”.
 * Each is the engine's `Template` with its pictures' bytes; the engine reads
 * it, and a picture's bytes go to the asset store so a sheet made from it
 * shows them. One the engine refuses is left as it is and listed with its
 * reason (`refused`), never dropped.
 */
export class LibraryProvider implements TemplateProvider {
  readonly section: GallerySection;
  readonly state = new Signal<ProviderState>({ state: 'loading' });
  readonly cards = new Signal<readonly TemplateCard[]>([]);
  /** Templates the engine refused: their id and reason (the gallery lists them as such). */
  readonly refused = new Signal<readonly { id: string; reason: string }[]>([]);
  private readonly store: DeviceTemplateStore;
  private readonly assets: AssetStore;
  private readonly engine: () => Promise<SheetEngine>;
  private readonly viewer: LibraryViewer;
  private readonly busy: LibraryBusy;
  private readonly organizations: LibraryOrganizations;
  private read: { record: StoredTemplate; template: Template }[] = [];

  constructor(section: 'mine' | 'org' | 'shared', store: DeviceTemplateStore, assets: AssetStore, engine: () => Promise<SheetEngine>, viewer: LibraryViewer, busy: LibraryBusy, organizations: LibraryOrganizations = () => null) {
    this.section = section;
    this.store = store;
    this.assets = assets;
    this.engine = engine;
    this.viewer = viewer;
    this.busy = busy;
    this.organizations = organizations;
  }

  private ours(r: StoredTemplate): boolean {
    return sectionOfRecord(r) === this.section;
  }

  /** Why a section with nothing to show has nothing: nobody signed in, no answer from the server yet, no organisation. */
  private emptyWhy(viewer: string | null | undefined): string | null {
    if (this.section === 'shared') return viewer === null ? PROVIDER_TEXTS.sharedSignIn : viewer === undefined ? PROVIDER_TEXTS.sharedOffline : null;
    if (this.section === 'org') {
      if (viewer === null) return PROVIDER_TEXTS.orgSignIn;
      if (viewer === undefined) return PROVIDER_TEXTS.orgOffline;
      const orgs = this.organizations();
      return orgs && !orgs.length ? PROVIDER_TEXTS.noOrganisation : null;
    }
    return null;
  }

  async refresh(): Promise<void> {
    let engine: SheetEngine;
    try {
      engine = await this.engine();
    } catch (e) {
      this.state.set({ state: 'unavailable', reason: PROVIDER_TEXTS.engineFailed(errorText(e)) });
      return;
    }
    try {
      const kept = await this.store.list();
      const ok = kept.flatMap((r) => (r.status === 'ok' ? [r.record] : []));
      const read: { record: StoredTemplate; template: Template }[] = [];
      const refused: { id: string; reason: string }[] = this.section === 'mine' ? kept.flatMap((r) => (r.status === 'unreadable' ? [{ id: r.key, reason: r.reason }] : [])) : [];
      for (const record of visibleRecords(ok, this.viewer()).filter((r) => this.ours(r))) {
        try {
          const t = engine.validateTemplate(JSON.stringify(record.template));
          read.push({ record, template: t });
          for (const a of t.assets) await this.assets.put(a.meta, fromBase64(a.data)).catch(() => {});
        } catch (e) {
          refused.push({ id: record.id, reason: errorText(e) });
        }
      }
      this.read = read;
      this.refused.set(refused);
      this.relabel();
      // “Kurumum” and “Benimle paylaşılanlar” with nothing to show say why.
      const why = read.length ? null : this.emptyWhy(this.viewer());
      this.state.set(why ? { state: 'unavailable', reason: why } : { state: 'ready' });
    } catch (e) {
      this.cards.set([]);
      this.state.set({ state: 'unavailable', reason: PROVIDER_TEXTS.deviceUnreadable((e as Error).message) });
    }
  }

  /** The cards again from what was read, their badges as the sync stands now (cheap: nothing is read). */
  relabel(): void {
    const busy = this.busy();
    this.cards.set(
      this.read.map(({ record: r, template }) =>
        cardOf(template, !r.cloud ? 'device' : r.cloud.organization ? 'org' : r.cloud.role === 'owner' ? 'cloud' : 'shared', libraryBadges(r, busy), {
          ...(r.cloud?.role && { role: r.cloud.role }),
          ...(r.cloud?.ownerName && { sharedBy: r.cloud.ownerName }),
          ...(r.cloud?.formerIds && { formerIds: r.cloud.formerIds }),
          ...(r.cloud?.formerRevision !== undefined && { formerRevision: r.cloud.formerRevision }),
          ...(r.cloud?.conflictOf && { conflictOf: r.cloud.conflictOf }),
          ...(r.cloud?.organization && { organization: r.cloud.organization }),
          ...(r.cloud?.publishedFrom && { publishedFrom: r.cloud.publishedFrom }),
        }),
      ),
    );
  }
}
