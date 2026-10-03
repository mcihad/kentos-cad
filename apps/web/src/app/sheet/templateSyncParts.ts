import type { SheetTemplateSummary } from '../../contracts/generated/sheet/SheetTemplateSummary';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { StoredTemplate, TemplateCloudState } from '../../product/sheet/templateStore';
import { ApiFailure } from '../cloud/api';

/**
 * The template sync's words and small parts (templateSync.ts): what it says
 * to the user, how it reads a failure, the keys of its commands, and a
 * device copy's cloud state from the cloud's summary of it.
 */

export const SYNC_TEXTS = {
  noSpace: 'Hesabın kişisel alanı bulunamadı; sunucu bu sürümle uyuşmuyor olabilir.',
  listFailed: (why: string) => `Bulut şablonları okunamadı: ${why}`,
  unshared: (name: string) => `“${name}” artık sizinle paylaşılmıyor; listenizden kalktı. Ondan yapılmış paftalar kalır.`,
  removed: (name: string) => `“${name}” bulutta silindi; bu cihazdaki kopyası da kalktı. Ondan yapılmış paftalar kalır.`,
  /** An organisation's template the list no longer has (deleted there, or the account left the organisation). */
  orgGone: (name: string, org: string) => `“${name}” artık “${org}” kurumunun şablonları arasında görünmüyor; bu cihazdaki kopyası da kalktı. Ondan yapılmış paftalar kalır.`,
  conflict: (name: string, copy: string) => `“${name}” siz değiştirirken bulutta da değişmiş: ikisi de kaldı. Bulutun sürümü indirildi; sizin değişiklikleriniz “${copy}” adıyla ayrı bir şablon oldu.`,
  readOnly: (name: string, copy: string) => `“${name}” için düzenleme izniniz kalmamış: değişiklikleriniz “${copy}” adıyla bu cihazda ayrı bir şablon olarak kaldı.`,
  actionFailed: (name: string, why: string) => `“${name}” eşitlenemedi: ${why}`,
} as const;

export const nameOf = (t: unknown): string => ((t as Template | null)?.meta?.name as string | undefined) ?? '';
export const transient = (e: unknown) => e instanceof ApiFailure && e.transient;
export const status = (e: unknown) => (e instanceof ApiFailure ? e.status : -1);

/** Hex SHA-256 of text, for the keys of the commands that send it. */
export async function digest(text: string): Promise<string> {
  const bytes = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text)));
  return [...bytes.subarray(0, 12)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** The stored template's content, its id the record's. */
export function templateOf(r: StoredTemplate, id: string): Template {
  const t = r.template as Template;
  return t.meta.id === id ? t : { ...t, meta: { ...t.meta, id } };
}

/** The content with the id and revision the cloud gave it (the cloud writes them into its own copy too). */
export function withMeta(t: Template, id: string, revision: number): Template {
  return { ...t, meta: { ...t.meta, id, revision } };
}

/**
 * A copy's cloud state from the cloud's summary of it, keeping what only this device knows (its old ids, a
 * conflict): its role, its owner (an organisation's template: who published it), its organisation and source.
 */
export function cloudOf(account: string, s: SheetTemplateSummary, prev: TemplateCloudState | undefined): TemplateCloudState {
  return {
    account,
    revision: s.revision,
    changed: false,
    role: s.role,
    ...(s.role !== 'owner' && s.ownerName && { ownerName: s.ownerName }),
    ...(s.shared && { shared: true }),
    ...(s.organization && { organization: { tenantId: s.organization.tenantId, name: s.organization.name } }),
    ...(s.publishedFrom && { publishedFrom: s.publishedFrom }),
    ...(prev?.conflictOf && { conflictOf: prev.conflictOf }),
    ...(prev?.formerIds?.length && { formerIds: prev.formerIds }),
    ...(prev?.formerRevision !== undefined && { formerRevision: prev.formerRevision }),
  };
}

/** Whether what the list says of a copy (its role, owner, sharing, organisation) differs from what it keeps. */
export function listChanged(next: TemplateCloudState, c: TemplateCloudState): boolean {
  return next.role !== c.role || next.ownerName !== c.ownerName || !!next.shared !== !!c.shared || next.organization?.name !== c.organization?.name || next.organization?.tenantId !== c.organization?.tenantId || next.publishedFrom !== c.publishedFrom;
}
