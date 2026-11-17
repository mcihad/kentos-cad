// The cloud projects of a place (docs/adr/9000 §15): an organisation's, or a professional's own; each opens its page
// with its drawing and its attachments.
import { portal } from '../api/portal';
import type { ProjectSummary } from '../api/types';
import { orgBySlug } from '../app/state';
import { h } from '../lib/dom';
import { moment } from '../lib/format';
import { icon } from '../ui/icons';
import { empty, pageHeader, table, twoLine } from '../ui/kit';
import type { PageFn } from './page';

export function projectHref(p: ProjectSummary): string {
  return `#/proje/${p.tenantId}/${p.id}`;
}

export function projectsTable(projects: ProjectSummary[], none: HTMLElement): HTMLElement {
  return table(
    [
      {
        label: 'Proje',
        cell: (p) =>
          h('div', { class: 'with-icon' }, h('span', { class: 'file-icon is-project' }, icon('project', 18)), twoLine(p.name, p.description || (p.storage === 'file' ? 'Dosya projesi' : 'Veritabanı projesi'))),
      },
      { label: 'Çalışma alanı', cell: (p) => (p.tenantKind === 'personal' ? 'Kişisel' : p.tenantName) },
      { label: 'Saklama', cell: (p) => (p.storage === 'file' ? 'Dosya' : 'Veritabanı'), class: 'narrow' },
      { label: 'Son değişiklik', cell: (p) => h('span', { class: 'muted num' }, moment(p.updatedAt)), class: 'narrow' },
    ],
    projects,
    { empty: none, href: projectHref },
  );
}

export const orgProjects: PageFn = async ({ params }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Bu kurumun üyesi değilsiniz.');
  const page = await portal.projects('organization', org.tenantId);
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Projeler', lead: 'Kurumun buluttaki projeleri. Bir projeyi açınca çizimini, eklerini ve revizyonlarını görürsünüz; masaüstü ve mobil aynı dosyalara bakar.' }),
    projectsTable(page.projects, empty('folder', 'Bulutta proje yok', 'KentOS masaüstünden buluta kaydedilen ya da bir başvurudan açılan projeler burada görünür.')),
  );
  return { crumbs: [{ label: org.tenantName, href: `#/kurum/${org.tenantSlug}` }, { label: 'Projeler' }], body };
};
