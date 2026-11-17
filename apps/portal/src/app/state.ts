// What the whole portal shares (docs/adr/9000 §8): the signed-in person, the catalog the server words, the unread
// notifications, and where the person works now: the platform, one of their organisations, or their own practice.
import type { DossierDirection, PortalCatalog, PortalMe, PortalMembership, Profession, ProfessionalStatus } from '../api/types';
import { Signal } from '../lib/signal';

export const me = new Signal<PortalMe | null>(null);
export const catalog = new Signal<PortalCatalog | null>(null);
export const unread = new Signal(0);

/** Where the person works on the portal. */
export type Context = { kind: 'admin' } | { kind: 'org'; org: PortalMembership } | { kind: 'pro' };

/** Every context the person has: the platform's administration, their organisations, their practice. */
export function contexts(m: PortalMe): Context[] {
  const out: Context[] = [];
  if (m.platformAdmin) out.push({ kind: 'admin' });
  for (const org of m.organizations) out.push({ kind: 'org', org });
  if (m.professional) out.push({ kind: 'pro' });
  return out;
}

/** The address a context opens at. */
export function homeOf(c: Context): string {
  switch (c.kind) {
    case 'admin':
      return '#/sistem';
    case 'org':
      return `#/kurum/${c.org.tenantSlug}`;
    case 'pro':
      return '#/mesleki';
  }
}

export function orgBySlug(slug: string): PortalMembership | undefined {
  return me.value?.organizations.find((o) => o.tenantSlug === slug);
}

export type Tone = 'neutral' | 'info' | 'warning' | 'success' | 'danger';

export function statusTone(code: string): Tone {
  return (catalog.value?.statuses.find((s) => s.code === code)?.tone as Tone | undefined) ?? 'neutral';
}

export function professionalStatus(code: ProfessionalStatus): { label: string; tone: Tone } {
  const s = catalog.value?.professionalStatuses.find((p) => p.code === code);
  return { label: s?.label ?? code, tone: (s?.tone as Tone | undefined) ?? 'neutral' };
}

export function professionLabel(code: Profession | null | undefined): string {
  if (!code) return '';
  return catalog.value?.professions.find((p) => p.code === code)?.label ?? code;
}

export function chamberOf(code: Profession): string {
  return catalog.value?.professions.find((p) => p.code === code)?.chamber ?? '';
}

export function kinds(direction: DossierDirection) {
  const c = catalog.value;
  return (direction === 'application' ? c?.applicationKinds : c?.transferKinds) ?? [];
}

export function kindLabel(direction: DossierDirection, code: string): string {
  return kinds(direction).find((k) => k.code === code)?.label ?? code;
}

export function docTypeLabel(code: string): string {
  return catalog.value?.docTypes.find((d) => d.code === code)?.label ?? code;
}

/** A tenant role's name. */
export function roleLabel(role: string): string {
  return (
    { owner: 'Sahip', admin: 'Yönetici', project_manager: 'Proje yöneticisi', editor: 'Düzenleyici', viewer: 'Görüntüleyici' } as Record<
      string,
      string
    >
  )[role] ?? role;
}

export function categoryLabel(category: string): string {
  return ({ municipality: 'Belediye', office: 'Büro', other: 'Kurum' } as Record<string, string>)[category] ?? category;
}

/** The offices the person may share a dossier with or apply for (docs/adr/9000 §18): an editor and above, usable now. */
export function myOffices(): PortalMembership[] {
  const rank: Record<string, number> = { viewer: 1, editor: 2, project_manager: 3, admin: 4, owner: 5 };
  return (me.value?.organizations ?? []).filter((o) => o.category === 'office' && o.usable && (rank[o.role] ?? 0) >= 2);
}
