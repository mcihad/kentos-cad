import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { ProjectPage } from '../../contracts/generated/ProjectPage';
import type { ProjectState } from '../../contracts/generated/ProjectState';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import type { CatalogRequest, CloudApi } from './api';

/**
 * The project catalog as the interface says it (docs/adr/0028, TODOS.md
 * CLOUD-02..05): the words for project types, lists, orders and states,
 * the lifecycle commands' envelopes, and paging through a list. The server
 * searches, sorts, counts and pages after its access check; here a list is
 * only asked for and shown. Nothing here touches the DOM.
 */

/** Project types in the order the interface offers them. */
export const PROJECT_TYPES: readonly ProjectType[] = ['cad', 'gis', 'landReadjustment', 'zoningPlan', 'subdivision', 'road', 'architecture'];

export const TYPE_LABEL: Record<ProjectType, string> = {
  cad: 'Genel CAD',
  gis: 'CBS',
  landReadjustment: '18 uygulaması',
  zoningPlan: 'İmar planı',
  subdivision: 'İfraz / tevhit',
  road: 'Yol',
  architecture: 'Mimari',
};

/** What choosing a type means, and does not (said wherever one is chosen). */
export const TYPE_HINT =
  'Tür yalnız projeleri bulmak ve düzenlemek içindir: bir modül açmaz, mevzuata uygunluk ya da resmî onay anlamına gelmez, projenin nasıl saklandığını değiştirmez.';

export interface ViewDef {
  id: CatalogView;
  label: string;
  icon: string;
  /** The orders it offers; the first is its own. */
  sorts: readonly CatalogSort[];
  /** What an empty list says (without a search). */
  empty: string;
}

/** The lists, in the order the interface shows them. */
export const VIEWS: readonly ViewDef[] = [
  {
    id: 'recent',
    label: 'Son kullanılanlar',
    icon: 'history',
    sorts: ['opened', 'updated', 'name'],
    empty: 'Henüz açtığınız bir bulut projesi yok. Açtığınız ve oluşturduğunuz projeler burada, en yenisi üstte durur.',
  },
  {
    id: 'favorites',
    label: 'Favoriler',
    icon: 'star',
    sorts: ['updated', 'name', 'created'],
    empty: 'Favori projeniz yok. Bir projeyi yıldızına tıklayarak buraya ekleyin; favorileriniz yalnız size görünür.',
  },
  {
    id: 'mine',
    label: 'Projelerim',
    icon: 'folder',
    sorts: ['updated', 'name', 'created'],
    empty: 'Sahibi olduğunuz bir proje yok. Açık çizimi Dosya → Buluta yükle ile gönderebilirsiniz.',
  },
  {
    id: 'organization',
    label: 'Kurum projeleri',
    icon: 'layers',
    sorts: ['updated', 'name', 'created'],
    empty: 'Bu kurumda size açık bir proje yok: sizin açtıklarınız ve sizinle paylaşılanlar burada görünür.',
  },
  {
    id: 'shared',
    label: 'Benimle paylaşılanlar',
    icon: 'share',
    sorts: ['updated', 'name', 'created'],
    empty: 'Sizinle paylaşılmış bir proje yok. Biri bir projeyi sizinle paylaşınca burada, sahibinin adı ve rolünüzle görünür.',
  },
  {
    id: 'archived',
    label: 'Arşivlenmişler',
    icon: 'archive',
    sorts: ['updated', 'name', 'created'],
    empty: 'Arşivlenmiş bir proje yok. Arşivlenen proje salt okunur olur ve burada durur.',
  },
  {
    id: 'trash',
    label: 'Çöp kutusu',
    icon: 'trash',
    sorts: ['trashed', 'name'],
    empty: 'Çöp kutusu boş. Geri yükleyebileceğiniz (sahibi ya da kurum yöneticisi olduğunuz) silinmiş projeler burada durur.',
  },
];

export const viewDef = (id: CatalogView): ViewDef => VIEWS.find((v) => v.id === id)!;

export const SORT_LABEL: Record<CatalogSort, string> = {
  updated: 'Son değişiklik',
  name: 'Ad',
  created: 'Oluşturulma',
  opened: 'Son açılma',
  trashed: 'Çöpe taşınma',
};

export const STATE_LABEL: Record<ProjectState, string> = {
  active: 'Etkin',
  archived: 'Arşivde',
  trashed: 'Çöp kutusunda',
};

/** A date and time as the lists write them: 26.09.2026 14:05. */
export const when = (iso: string): string => new Date(iso).toLocaleString('tr-TR', { dateStyle: 'short', timeStyle: 'short' });

/** A date as the lists write it: 26.09.2026. */
export const day = (iso: string): string => new Date(iso).toLocaleDateString('tr-TR', { day: '2-digit', month: '2-digit', year: 'numeric' });

/** When a project in the trash goes for good, in one line. */
export function purgeText(p: ProjectSummary): string {
  return p.purgeAfter ? `${day(p.purgeAfter)} tarihinde kalıcı olarak silinir` : 'Elle silinene kadar çöp kutusunda kalır';
}

/** Tags as typed in one field: separated by commas; spaces around each dropped (the server also folds duplicates). */
export function parseTags(text: string): string[] {
  return text
    .split(',')
    .map((t) => t.trim().replace(/\s+/g, ' '))
    .filter(Boolean);
}

/** A project's command (docs/adr/0028) with a fresh request id and idempotency key. */
export function catalogEnvelope(commandName: string, tenantId: string, projectId: string, input: unknown, expectedVersions: Record<string, string> = {}): CommandEnvelope {
  return {
    commandName,
    version: 1,
    tenantId,
    projectId,
    requestId: `web-${crypto.randomUUID()}`,
    idempotencyKey: crypto.randomUUID(),
    expectedVersions,
    input: input as CommandEnvelope['input'],
  };
}

/** What can be asked of a list page by page. */
export type CatalogSource = Pick<CloudApi, 'catalog'>;

/**
 * One list, page by page: `reset` asks for the first page of a new
 * request (a view, a search, an order), `more` for the next one. An answer
 * that arrives after a newer request was made is thrown away, so a slow
 * search never replaces a newer one's list.
 */
export class CatalogPager {
  private readonly api: CatalogSource;
  private request: CatalogRequest | null = null;
  private generation = 0;
  private abort: AbortController | null = null;
  projects: ProjectSummary[] = [];
  total = 0;
  next: string | undefined;
  retentionDays = 0;

  constructor(api: CatalogSource) {
    this.api = api;
  }

  get hasMore(): boolean {
    return !!this.next;
  }

  /** The first page of `request`; false when a newer request overtook it. */
  async reset(request: CatalogRequest): Promise<boolean> {
    this.request = request;
    this.projects = [];
    this.total = 0;
    this.next = undefined;
    return this.load(request);
  }

  /** The next page of the current request, added after what is there. */
  async more(): Promise<boolean> {
    if (!this.request || !this.next) return true;
    return this.load({ ...this.request, after: this.next });
  }

  /** Stops whatever is on its way (the window closes). */
  cancel(): void {
    this.generation++;
    this.abort?.abort();
    this.abort = null;
  }

  private async load(request: CatalogRequest): Promise<boolean> {
    this.abort?.abort();
    const abort = new AbortController();
    this.abort = abort;
    const gen = ++this.generation;
    let page: ProjectPage;
    try {
      page = await this.api.catalog(request, abort.signal);
    } catch (e) {
      if (gen !== this.generation) return false;
      throw e;
    }
    if (gen !== this.generation) return false;
    this.projects = request.after ? [...this.projects, ...page.projects] : page.projects;
    this.total = page.total;
    this.next = page.next;
    this.retentionDays = page.trashRetentionDays;
    this.abort = null;
    return true;
  }
}
