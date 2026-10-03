import { Signal, type ReadonlySignal } from '../../core/signal';
import type { SheetEngine } from '../../product/sheet/engine';
import type { AssetStore } from '../../product/sheet/store';
import type { DeviceTemplateStore } from '../../product/sheet/templateStore';
import type { TemplateCard, TemplateProvider } from '../../product/sheet/templates';
import type { LibraryLine } from '../../ui/sheet/host';
import type { AppContext } from '../context';
import type { CloudLibrary, LibraryHost } from './cloudLibrary';
import { LibraryProvider, SystemProvider } from './providers';

/**
 * The gallery's sources as the service keeps them (docs/sheet/design.md
 * §13): the system's, “Benim”, “Kurumum” and “Benimle paylaşılanlar”, and
 * the account's cloud library once it is there (it
 * comes with the engine: cloudLibrary.ts). Says which account's copies the
 * lists show, what the sync is busy with, the line the gallery shows for
 * the library and why a cloud action cannot run now; and answers a
 * sheet's template by its id, an old one included (a template the cloud
 * gave a new id keeps its sheets' “Yeni sürüm var”).
 */

const NONE: ReadonlySet<string> = new Set();

export const SHELF_TEXTS = {
  signIn: 'Bulut şablonları için önce bulut hesabınızla giriş yapın.',
  noLibrary: 'Bulut şablonları pafta motoruyla birlikte yüklenir.',
} as const;

export class TemplateShelf implements LibraryHost {
  readonly system: SystemProvider;
  /** “Benim”: made on this device, and the account's own in the cloud. */
  readonly mine: LibraryProvider;
  /** “Kurumum”: the libraries of the organisations the account is an active member of. */
  readonly org: LibraryProvider;
  /** “Benimle paylaşılanlar”. */
  readonly shared: LibraryProvider;
  readonly providers: readonly TemplateProvider[];
  readonly templates: DeviceTemplateStore;
  /** What the gallery and the status bar say of the library now (synced, offline …). */
  readonly line: ReadonlySignal<LibraryLine>;
  private readonly lineSig = new Signal<LibraryLine>({ state: 'none', text: '' }, (a, b) => a.state === b.state && a.text === b.text);
  private readonly ctx: AppContext;
  private readonly engineNow: () => SheetEngine | null;
  private lib: CloudLibrary | null = null;

  constructor(ctx: AppContext, templates: DeviceTemplateStore, assets: AssetStore, engine: () => Promise<SheetEngine>, engineNow: () => SheetEngine | null) {
    this.ctx = ctx;
    this.templates = templates;
    this.engineNow = engineNow;
    this.line = this.lineSig;
    const viewer = () => {
      const me = ctx.cloud.me.value;
      if (me && ctx.cloud.auth.value === 'signedIn') return me.user.id;
      return ctx.cloud.auth.value === 'signedOut' ? null : undefined;
    };
    const busy = () => this.lib?.sync.busy.value ?? NONE;
    this.system = new SystemProvider(engine);
    this.mine = new LibraryProvider('mine', templates, assets, engine, viewer, busy);
    this.org = new LibraryProvider('org', templates, assets, engine, viewer, busy, () => this.organizations());
    this.shared = new LibraryProvider('shared', templates, assets, engine, viewer, busy);
    this.providers = [this.system, this.mine, this.org, this.shared];
  }

  /** The cloud library, there once the engine is. */
  get library(): CloudLibrary | null {
    return this.lib;
  }

  attach(lib: CloudLibrary): void {
    this.lib = lib;
    // The organisations come with the cloud's list: “Kurumum” says what it has (or why it has none) after each.
    lib.sync.organizations.subscribe(() => void this.org.refresh());
    this.libraryState();
  }

  /** The organisations of the account's library (the last list the cloud gave); null when none was read yet. */
  organizations(): readonly { readonly tenantId: string; readonly name: string; readonly canPublish: boolean }[] | null {
    return this.lib?.sync.organizations.value ?? null;
  }

  engine(): SheetEngine | null {
    return this.engineNow();
  }

  libraryChanged(): void {
    void this.mine.refresh();
    void this.org.refresh();
    void this.shared.refresh();
  }

  libraryState(): void {
    this.mine.relabel();
    this.org.relabel();
    this.shared.relabel();
    this.lineSig.set(this.lib ? { state: this.lib.sync.status.value.state, text: this.lib.line() } : { state: 'none', text: '' });
  }

  /** Why cloud actions cannot run now; null when they can. */
  why(): string | null {
    if (this.ctx.cloud.auth.value !== 'signedIn') return SHELF_TEXTS.signIn;
    return this.lib ? this.lib.why() : SHELF_TEXTS.noLibrary;
  }

  /** Every card the gallery has. */
  cards(): TemplateCard[] {
    return this.providers.flatMap((p) => p.cards.value);
  }

  /** A template by its id or an id it had before the cloud gave it its own. */
  card(id: string): TemplateCard | undefined {
    const all = this.cards();
    return all.find((c) => c.id === id) ?? all.find((c) => c.formerIds?.includes(id));
  }

  /**
   * The name and revision of every template a sheet may name, for “Yeni sürüm
   * var”; an old id with the revision counted as it was counted under it
   * (the cloud's first revision is the one it had here then).
   */
  known(): Map<string, { name: string; revision: number }> {
    const out = new Map<string, { name: string; revision: number }>();
    for (const c of this.cards()) {
      out.set(c.id, { name: c.name, revision: c.revision });
      for (const old of c.formerIds ?? []) if (!out.has(old)) out.set(old, { name: c.name, revision: (c.formerRevision ?? 1) + c.revision - 1 });
    }
    return out;
  }
}
