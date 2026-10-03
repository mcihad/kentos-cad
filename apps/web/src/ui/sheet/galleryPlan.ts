import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { GallerySection, ProviderState, TemplateCard } from '../../product/sheet/templates';

/**
 * What the template gallery offers for a card and what it says when it
 * cannot (docs/sheet/design.md §12, §13; the style manager's “system items
 * and own items”, DESIGN.md §7.14): system templates are read-only (used,
 * copied, edited as a copy); the user's own are edited, synced, deleted and
 * published to an organisation; one only on this device cannot be shared or
 * published before it is synced; a shared one is used and copied, and its
 * owner decides the rest; an organisation's is used and copied by every
 * member, edited and deleted by the one who published it and the
 * organisation's administrators, and never shared one by one. Apart from the
 * DOM (TemplateGallery.ts).
 */

export type TemplateAction = 'use' | 'duplicate' | 'edit' | 'publish' | 'share' | 'sync' | 'delete';

export interface ActionView {
  /** Offered at all for this card (a system template has no Sil, Eşitle or Paylaş). */
  readonly shown: boolean;
  readonly label: string;
  /** Why it cannot run now; null when it can. */
  readonly reason: string | null;
}

/** What the gallery can do now beyond what each card is. */
export interface GalleryAbilities {
  /** Why the engine cannot work (use, copy, edit a template); null when it can. */
  readonly engine: string | null;
  /** Why templates cannot be synced and shared through the cloud now (signed out, the server away); null when they can. */
  readonly cloud: string | null;
  /** Why the account's library cannot be changed at all (signed out); null when it can, offline too (it waits for the connection). */
  readonly account: string | null;
  /** How many organisations the account may publish to (“Kuruma yayımla…”); none: 0. */
  readonly publishTo?: number;
}

export const SHARE_NEEDS_SYNC = 'Paylaşmak için önce buluta eşitleyin.';
export const PUBLISH_NEEDS_SYNC = 'Kuruma yayımlamak için önce buluta eşitleyin.';
export const PUBLISH_NO_ORGANISATION = 'Yayımlayabileceğiniz bir kurum yok: kurum sahibi, yöneticiler ve proje açabilen üyeler yayımlar.';
export const PUBLISH_WAIT_SYNC = 'Bu cihazdaki değişiklikler eşitlenince yayımlayın: kuruma bulutun son sürümü kopyalanır.';
export const ORGANISATION_NO_DELETE = 'Kurum şablonunu yalnız yayımlayan ve kurum yöneticileri siler.';

export function actionsOf(card: TemplateCard, can: GalleryAbilities): Record<TemplateAction, ActionView> {
  const system = card.source === 'system';
  const device = card.source === 'device';
  const cloud = card.source === 'cloud';
  const shared = card.source === 'shared';
  const org = card.source === 'org';
  const editor = shared && card.role === 'editor';
  // An organisation's template is written by the one who published it and the organisation's administrators.
  const orgWriter = org && (card.role === 'owner' || card.role === 'admin');
  const waiting = card.badges.includes('unsynced') || card.badges.includes('syncing');
  return {
    use: { shown: true, label: 'Kullan', reason: can.engine },
    // The style manager copies into “Kitaplığım” (Kopyala → Kitaplığıma); a template goes to the user's own: Şablonlarıma.
    duplicate: { shown: true, label: system || shared || org ? 'Şablonlarıma kopyala' : 'Çoğalt', reason: can.engine },
    edit: { shown: true, label: system || (shared && !editor) || (org && !orgWriter) ? 'Kopyasını düzenle' : 'Düzenle', reason: can.engine },
    publish: {
      shown: device || cloud,
      label: 'Kuruma yayımla…',
      reason: device ? PUBLISH_NEEDS_SYNC : (can.cloud ?? (!can.publishTo ? PUBLISH_NO_ORGANISATION : waiting ? PUBLISH_WAIT_SYNC : null)),
    },
    share: {
      shown: !system && !org,
      label: 'Paylaş…',
      reason: shared ? 'Yalnız şablonun sahibi paylaşır.' : device ? SHARE_NEEDS_SYNC : can.cloud,
    },
    sync: { shown: device || cloud || shared || org, label: device ? 'Buluta eşitle' : 'Şimdi eşitle', reason: can.cloud },
    delete: {
      shown: !system,
      label: 'Sil',
      reason: shared ? 'Benimle paylaşılan şablon silinmez; sahibi paylaşımı kaldırırsa listeden kalkar.' : org ? (orgWriter ? can.account : ORGANISATION_NO_DELETE) : cloud ? can.account : null,
    },
  };
}

/** The line under a section's name in the gallery's left list when its provider has nothing to give now. */
export function sectionNote(state: ProviderState): string | null {
  if (state.state === 'ready') return null;
  if (state.state === 'loading') return 'Yükleniyor…';
  return state.reason;
}

/** What an empty list says, by section, when its provider is ready but has no card (or a search found none). */
export function emptyText(section: GallerySection | 'all', searching: boolean, state: ProviderState): { title: string; text: string } {
  if (state.state === 'soon') return { title: 'Yakında', text: state.reason };
  // Kurumum with none to list is no failure: the account has no organisation, or is not signed in.
  if (state.state === 'unavailable') return { title: section === 'org' ? 'Kurum şablonu yok' : 'Şimdi listelenemiyor', text: state.reason };
  if (state.state === 'loading') return { title: 'Yükleniyor…', text: '' };
  if (searching) return { title: 'Eşleşen şablon yok', text: 'Aramayı ya da kâğıt ve tür süzgeçlerini değiştirin; başka kiplerin şablonları için “Bütün kiplerin şablonları”nı açın.' };
  switch (section) {
    case 'system':
      return { title: 'Sistem şablonu yok', text: 'Sistem şablonları uygulamayla gelir.' };
    case 'mine':
      return { title: 'Henüz şablonunuz yok', text: 'Bir paftayı şeridin Pafta sekmesinde “Şablon olarak kaydet” ile buraya alın; önce bu cihazda saklanır.' };
    case 'shared':
      return { title: 'Sizinle paylaşılan şablon yok', text: 'Biri bir şablonu sizinle paylaştığında burada görünür.' };
    case 'org':
      return { title: 'Kurum şablonu yok', text: 'Kurumunuzun yayımladığı şablonlar burada görünür.' };
    default:
      return { title: 'Şablon yok', text: '' };
  }
}

/** “A3 yatay”: a paper choice as the gallery writes it (`name` gives a paper's name: the engine's table). */
export const paperText = (p: PaperChoice, name: (paper: PaperChoice['paper']) => string) => `${name(p.paper)} ${p.orientation === 'landscape' ? 'yatay' : 'dikey'}`;

/** The papers Kullan offers for a template: the ones it suits (its recommended one first), each the way it suits. */
export function paperChoices(card: TemplateCard, name: (paper: PaperChoice['paper']) => string): { choice: PaperChoice; label: string }[] {
  return card.papers.map((p, i) => ({ choice: p, label: `${paperText(p, name)}${i === 0 ? ' (önerilen)' : ''}` }));
}
