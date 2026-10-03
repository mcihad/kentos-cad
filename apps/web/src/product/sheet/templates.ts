import type { ProjectType } from '../../contracts/generated/ProjectType';
import type { Capabilities } from '../../contracts/generated/sheet/Capabilities';
import type { CloudOrganization } from '../../contracts/generated/sheet/CloudOrganization';
import type { Paper } from '../../contracts/generated/sheet/Paper';
import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { RankedTemplate } from '../../contracts/generated/sheet/RankedTemplate';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { TemplateFit } from '../../contracts/generated/sheet/TemplateFit';
import type { TemplateMeta } from '../../contracts/generated/sheet/TemplateMeta';
import type { TemplateRole } from '../../contracts/generated/sheet/TemplateRole';
import type { Workspace } from '../../contracts/generated/Workspace';
import { foldTurkish } from '../../core/text';
import type { ReadonlySignal } from '../../core/signal';

/**
 * The template library as the gallery shows it (docs/sheet/design.md §11,
 * §11a, §12, §13): cards gathered from several providers (the system's, from
 * the engine; this device's, from IndexedDB, read by the engine; the
 * cloud's and those shared with me, Part C; the organisations', Part F),
 * the gallery's sections and its filters. The order a project sees them in
 * is the engine's (`rankTemplates`: the project's type first, then its
 * mode, then the common ones, other modes' last with their badge); this
 * module only filters by the gallery's query and keeps that order.
 */

/** Where a template comes from. */
export type TemplateSource = 'system' | 'device' | 'cloud' | 'shared' | 'org';

/** The gallery's sections, in order: Sistem · Benim · Kurumum · Benimle paylaşılanlar. */
export type GallerySection = 'system' | 'mine' | 'org' | 'shared';

export const SECTIONS: readonly { readonly id: GallerySection; readonly label: string; readonly icon: string }[] = [
  { id: 'system', label: 'Sistem', icon: 'lock' },
  { id: 'mine', label: 'Benim', icon: 'styles' },
  { id: 'org', label: 'Kurumum', icon: 'layers' },
  { id: 'shared', label: 'Benimle paylaşılanlar', icon: 'share' },
];

export const sectionOf = (source: TemplateSource): GallerySection => (source === 'system' ? 'system' : source === 'org' ? 'org' : source === 'shared' ? 'shared' : 'mine');

/**
 * A card's badges (design §11, §13): where it is and whether it is in step
 * with the cloud (only on this device; synced; going now; changed here and
 * not yet synced; kept by a conflict), whether its owner shared it, the
 * account's role in one shared with it, and an organisation's template's
 * organisation (“Kurum: <ad>”) with the account's part in it (published it,
 * administers it; a plain member has no badge).
 */
export type TemplateBadge = 'system' | 'device' | 'synced' | 'syncing' | 'unsynced' | 'conflict' | 'shared' | 'viewer' | 'editor' | 'org' | 'published' | 'admin' | 'newer';

export const BADGE_LABEL: Record<TemplateBadge, string> = {
  system: 'Sistem',
  device: 'Bu cihazda',
  synced: 'Eşitlendi',
  syncing: 'Eşitleniyor',
  unsynced: 'Değişti, eşitlenmedi',
  conflict: 'Çakışma',
  shared: 'Paylaşıldı',
  viewer: 'Görüntüleyebilir',
  editor: 'Düzenleyebilir',
  org: 'Kurum',
  published: 'Yayımladınız',
  admin: 'Yöneticisiniz',
  newer: 'Yeni sürüm var',
};

/** What a badge means, for its tooltip. */
export const BADGE_TIP: Record<TemplateBadge, string> = {
  system: 'Uygulamayla gelir; salt okunurdur.',
  device: 'Yalnız bu cihazda; bulutta kopyası yok. Paylaşmak için önce buluta eşitleyin.',
  synced: 'Bulut hesabınızdakiyle aynı; web ve masaüstünde kullanılır.',
  syncing: 'Bulutla şimdi eşitleniyor.',
  unsynced: 'Bu cihazda değişti; bağlantı gelince buluta gider.',
  conflict: 'Siz değiştirirken bulutta da değişmişti: bu, sizin sürümünüz; bulutunki ayrıca duruyor.',
  shared: 'Başkalarıyla paylaştınız.',
  viewer: 'Sizinle paylaşıldı: kullanır ve kopyalarsınız.',
  editor: 'Sizinle paylaşıldı: yeni sürümünü de kaydedebilirsiniz.',
  org: 'Kurumunuzun şablonu: kurumun etkin üyeleri görür ve kullanır; yayımlayan ve kurum yöneticileri düzenler ve siler.',
  published: 'Bu kurum şablonunu siz yayımladınız: düzenler ve silersiniz.',
  admin: 'Kurum yöneticisi olarak düzenler ve silersiniz.',
  newer: 'Bu şablonun daha yeni bir sürümü var.',
};

/** A badge as a card writes it: an organisation's carries its name (“Kurum: Örnek Harita Bürosu”). */
export const badgeLabel = (k: TemplateBadge, card: Pick<TemplateCard, 'organization'>): string => (k === 'org' && card.organization ? `Kurum: ${card.organization.name}` : BADGE_LABEL[k]);

/** A template as a card: what the gallery lists, filters and shows beside it. */
export interface TemplateCard {
  readonly id: string;
  readonly revision: number;
  readonly name: string;
  readonly description: string;
  /** Its kind (genel, kadastro, imar, rapor …): the gallery's kind filter. */
  readonly category: string;
  readonly tags: readonly string[];
  /** The papers it suits, the recommended one first; it is laid out on any paper by its constraints (design §3.2). */
  readonly papers: readonly PaperChoice[];
  /** The work modes it is made for (both cad and gis: common) and the project types (design §11a). */
  readonly workspaces: readonly Workspace[];
  readonly projectTypes: readonly ProjectType[];
  readonly source: TemplateSource;
  readonly badges: readonly TemplateBadge[];
  readonly author?: string;
  /** When it last changed, RFC 3339. */
  readonly updated?: string;
  /** Who shared it, for “Benimle paylaşılanlar”. */
  readonly sharedBy?: string;
  /** The account's role in one of its cloud library (its own: owner). */
  readonly role?: TemplateRole;
  /** Ids it had before the cloud gave it its own: sheets made from it may name one. */
  readonly formerIds?: readonly string[];
  /** Its revision under those ids when the cloud gave it its own (the cloud counts from 1 again). */
  readonly formerRevision?: number;
  /** A copy a conflict kept: the template it is a copy of (design §13). */
  readonly conflictOf?: string;
  /** The organisation whose library it is in (“Kurumum”). */
  readonly organization?: CloudOrganization;
  /** The template it was published from (“Kuruma yayımla…”). */
  readonly publishedFrom?: string;
  /** The template itself, as the engine read it: drawn on the card, used, copied. */
  readonly template: Template;
}

/** What a card says beyond its template: who shared it, the account's role, its old ids. */
export type CardExtra = Pick<TemplateCard, 'sharedBy' | 'role' | 'formerIds' | 'formerRevision' | 'conflictOf' | 'organization' | 'publishedFrom'>;

/** The card of a template the engine read. */
export function cardOf(template: Template, source: TemplateSource, badges: readonly TemplateBadge[], extra: CardExtra = {}): TemplateCard {
  const m = template.meta;
  return {
    ...extra,
    id: m.id,
    revision: m.revision,
    name: m.name,
    description: m.description,
    category: m.category,
    tags: m.tags,
    papers: m.papers,
    workspaces: m.workspaces,
    projectTypes: m.projectTypes,
    source,
    badges,
    author: m.author || undefined,
    updated: m.updated || undefined,
    template,
  };
}

/** Whether a provider has its cards, and why not. */
export type ProviderState =
  | { readonly state: 'ready' }
  | { readonly state: 'loading' }
  /** It cannot list now (the engine could not be fetched, the browser keeps no data): the reason, said in the list. */
  | { readonly state: 'unavailable'; readonly reason: string }
  /** It is announced and comes later: said in the list as “yakında”. */
  | { readonly state: 'soon'; readonly reason: string };

/** One source of cards. The gallery lists what each has and says why one has none. */
export interface TemplateProvider {
  readonly section: GallerySection;
  readonly state: ReadonlySignal<ProviderState>;
  readonly cards: ReadonlySignal<readonly TemplateCard[]>;
  /** Asks again (the gallery opens, a template was saved). */
  refresh(): Promise<void>;
}

/** The open project as the gallery sorts and checks templates for it: its work mode, its type and what it offers. */
export interface ProjectTraits {
  readonly workspace: Workspace;
  readonly projectType: ProjectType | null;
  readonly capabilities: Capabilities;
}

/** How a card stands with the project (the engine's `TemplateFit`), as the gallery's headings say it. */
export const FIT_LABEL: Record<TemplateFit, string> = {
  projectType: 'İşin türüne uygun',
  workspace: 'Proje türüne uygun',
  common: 'Ortak: her proje türünde',
  other: 'Başka bir proje türü için',
};

export interface GalleryQuery {
  readonly section: GallerySection | 'all';
  /** Words in the name, description, category or tags; Turkish letters fold (“imar” finds “İmar”). */
  readonly text: string;
  /** A paper (`a3`), or null for any. */
  readonly paper: Paper | null;
  /** A category, or null for any. */
  readonly kind: string | null;
  /** “Bütün türlerin şablonları”: other types' templates too, with their type's badge. */
  readonly allModes: boolean;
}

export interface ArrangedCard {
  readonly card: TemplateCard;
  readonly fit: TemplateFit;
  /** Another mode's badge (“CBS şablonu”), from the engine. */
  readonly badge?: string;
}

/** The metas the engine ranks, one per card. */
export const metasOf = (cards: readonly TemplateCard[]): TemplateMeta[] => cards.map((c) => c.template.meta);

function matchesQuery(card: TemplateCard, q: GalleryQuery, words: readonly string[]): boolean {
  if (q.section !== 'all' && sectionOf(card.source) !== q.section) return false;
  if (q.paper && !card.papers.some((p) => p.paper === q.paper)) return false;
  if (q.kind && card.category !== q.kind) return false;
  if (words.length) {
    const hay = foldTurkish([card.name, card.description, card.category, ...card.tags].join(' '));
    if (!words.every((w) => hay.includes(w))) return false;
  }
  return true;
}

/**
 * The cards the gallery lists for a query, in the engine's order (`ranked`,
 * `rankTemplates` over every card's meta): another mode's only with
 * `allModes`, with its badge. A card the engine did not rank is left out.
 */
export function arrangeTemplates(cards: readonly TemplateCard[], ranked: readonly RankedTemplate[], q: GalleryQuery): ArrangedCard[] {
  const words = foldTurkish(q.text).split(/\s+/).filter(Boolean);
  const byId = new Map<string, TemplateCard[]>();
  for (const c of cards) byId.set(c.id, [...(byId.get(c.id) ?? []), c]);
  const out: ArrangedCard[] = [];
  const seen = new Set<string>();
  for (const r of ranked) {
    if (seen.has(r.id)) continue;
    seen.add(r.id);
    if (!r.matches && !q.allModes) continue;
    for (const card of byId.get(r.id) ?? []) if (matchesQuery(card, q, words)) out.push({ card, fit: r.fit, badge: r.badge ?? undefined });
  }
  return out;
}

/** How many cards of a section the work mode keeps out now (said under the list: “4 şablon başka türlerin”). */
export function hiddenByMode(cards: readonly TemplateCard[], ranked: readonly RankedTemplate[], section: GallerySection | 'all'): number {
  const out = new Set(ranked.filter((r) => !r.matches).map((r) => r.id));
  return cards.filter((c) => (section === 'all' || sectionOf(c.source) === section) && out.has(c.id)).length;
}

const collator = new Intl.Collator('tr', { sensitivity: 'base', numeric: true });

/** The papers and kinds the cards use, for the filters (papers in the series' order, kinds by name). */
export function facetsOf(cards: readonly TemplateCard[]): { papers: Paper[]; kinds: string[] } {
  const papers = new Set<Paper>();
  const kinds = new Set<string>();
  for (const c of cards) {
    c.papers.forEach((p) => papers.add(p.paper));
    if (c.category) kinds.add(c.category);
  }
  const series = (p: Paper) => {
    const m = /^([ab])(\d)$/.exec(p);
    return m ? (m[1] === 'a' ? 0 : 100) + Number(m[2]) : 1000;
  };
  return { papers: [...papers].sort((a, b) => series(a) - series(b)), kinds: [...kinds].sort(collator.compare) };
}

/** A category as the gallery writes it: “kadastro” → “Kadastro”. */
export const categoryLabel = (c: string): string => c.charAt(0).toLocaleUpperCase('tr-TR') + c.slice(1);
