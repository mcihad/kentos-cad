// A cloud project's page (docs/adr/9000 §15): its drawing as the server draws it, the files kept beside it and, for a
// file project, its revisions; for a database project its records as the caller's role reads them, and the caller's
// access (docs/adr/9194); what a browser or a phone shows of a project without a CAD view.
import { sendFile } from '../api/files';
import { cloud } from '../api/portal';
import type { AttachmentView, FileRevisions, ProjectDetails } from '../api/types';
import { me } from '../app/state';
import { type Child, h, replaceChildren } from '../lib/dom';
import { bytes, day, moment } from '../lib/format';
import { confirm } from '../ui/dialog';
import { type IconName, icon } from '../ui/icons';
import { button, empty, facts, pageHeader, panel, table, twoLine } from '../ui/kit';
import { toast } from '../ui/toast';
import { scanBadge, uploadRow } from '../ui/uploads';
import type { PageFn } from './page';
import { accessView, recordsPanel, throughForms } from './project-records';

function kindIcon(media: string, name: string): IconName {
  if (media === 'application/pdf') return 'pdf';
  if (media.startsWith('image/') || media.startsWith('video/')) return 'image';
  if (name.toLowerCase().endsWith('.kcad')) return 'project';
  return 'file';
}

/** The server's own words in a refusal's body (an `ApiError`), when it gives them. */
async function refusal(res: Response): Promise<string | null> {
  try {
    const body: unknown = await res.json();
    const message = (body as { message?: unknown } | null)?.message;
    return typeof message === 'string' ? message : null;
  } catch {
    return null;
  }
}

/** The drawing: fetched with the session, its counts read from the answer, shown as a picture. */
function drawing(tenant: string, project: string): HTMLElement {
  const frame = h('div', { class: 'preview-frame is-loading' }, h('div', { class: 'spinner' }));
  const caption = h('div', { class: 'preview-caption' });
  const box = h('div', { class: 'preview' }, frame, caption);
  void (async () => {
    try {
      const res = await fetch(cloud.previewUrl(tenant, project, 1600), { credentials: 'same-origin' });
      if (!res.ok) {
        // A refusal says why in the server's words (a Veri girişi reads the records, not the drawing); a missing
        // drawing and a server that cannot draw now are said apart.
        const refused = res.status === 403 ? await refusal(res) : null;
        const why = refused ?? (res.status === 404 ? 'Projede henüz kaydedilmiş bir çizim yok.' : 'Çizim şu an gösterilemiyor.');
        frame.classList.remove('is-loading');
        replaceChildren(frame, empty('image', refused ? 'Çizim size kapalı' : 'Çizim yok', why));
        return;
      }
      const drawn = Number(res.headers.get('x-kentos-drawn') ?? 0);
      const skipped = Number(res.headers.get('x-kentos-skipped') ?? 0);
      const url = URL.createObjectURL(await res.blob());
      const img = h('img', { src: url, alt: 'Projenin çizimi', class: 'preview-image' });
      frame.classList.remove('is-loading');
      replaceChildren(frame, h('a', { href: url, target: '_blank', rel: 'noopener', title: 'Tam boy aç' }, img));
      caption.textContent = skipped
        ? `${drawn} nesne çizildi; ${skipped} nesne (ölçü, tablo, resim…) önizlemede gösterilmez.`
        : `${drawn} nesne çizildi.`;
    } catch {
      frame.classList.remove('is-loading');
      replaceChildren(frame, empty('image', 'Çizim yok', 'Sunucuya ulaşılamadı.'));
    }
  })();
  return box;
}

function attachmentsPanel(d: ProjectDetails, list: AttachmentView[], reload: () => void): HTMLElement {
  const p = d.project;
  const canWrite = p.access.permissions.includes('feature.write') && p.state === 'active';
  const canDownload = p.access.permissions.includes('project.download');
  const rows = table(
    [
      {
        label: 'Dosya',
        cell: (a) =>
          h(
            'div',
            { class: 'with-icon' },
            h('span', { class: 'file-icon' }, icon(kindIcon(a.mediaType, a.name), 18)),
            twoLine(h('span', { title: `SHA-256 ${a.sha256}` }, a.name), a.note ?? `${a.createdByName} ekledi`),
          ),
      },
      { label: 'Boyut', cell: (a) => h('span', { class: 'num muted' }, bytes(a.size)), class: 'narrow' },
      { label: 'Eklendi', cell: (a) => h('span', { class: 'num muted' }, moment(a.createdAt)), class: 'narrow' },
      { label: '', cell: (a) => scanBadge(a.scan, a.scanNote), class: 'narrow' },
      {
        label: '',
        class: 'narrow',
        cell: (a) => {
          const out: Child[] = [];
          if (canDownload && a.viewable) {
            out.push(h('a', { class: 'icon-btn', href: cloud.attachmentUrl(p.tenantId, p.id, a.id, true), target: '_blank', rel: 'noopener', title: 'Aç', 'aria-label': `${a.name} dosyasını aç` }, icon('eye', 18)));
          }
          if (canDownload) {
            out.push(h('a', { class: 'icon-btn', href: cloud.attachmentUrl(p.tenantId, p.id, a.id), download: a.name, title: 'İndir', 'aria-label': `${a.name} dosyasını indir` }, icon('download', 18)));
          }
          if (canWrite) {
            const remove = h('button', { class: 'icon-btn', type: 'button', title: 'Kaldır', 'aria-label': `${a.name} dosyasını kaldır` }, icon('trash', 18));
            remove.addEventListener('click', async () => {
              if (!(await confirm({ title: 'Ek kaldırılsın mı?', text: `“${a.name}” projeden kaldırılır.`, confirm: 'Kaldır', danger: true }))) return;
              await cloud.removeAttachment(p.tenantId, p.id, a.id);
              toast('Ek kaldırıldı.');
              reload();
            });
            out.push(remove);
          }
          return h('div', { class: 'file-actions' }, ...out);
        },
      },
    ],
    list,
    { empty: empty('paperclip', 'Ek yok', 'Rapor, pafta, çizelge, fotoğraf ya da kaynak dosyaları projenin yanında burada saklanır.') },
  );
  return panel('Ekler', h('div', null, rows, canWrite ? attachZone(d, reload) : null), {
    note: canWrite ? null : 'Ek koymak için projede değişiklik yetkisi gerekir.',
  });
}

/** Files dropped or chosen go up in parts, each with its progress; the server's refusal is shown as it says it. */
function attachZone(d: ProjectDetails, reload: () => void): HTMLElement {
  const p = d.project;
  const input = h('input', { type: 'file', multiple: true, hidden: true });
  const note = h('input', { class: 'input', type: 'text', placeholder: 'Not (isteğe bağlı): “Kuzey cephe, 12 Kasım”', maxlength: 2000, 'aria-label': 'Eklerin notu' });
  const progress = h('div', { class: 'uploads' });
  const send = async (files: File[]) => {
    let added = 0;
    for (const file of files) {
      const r = uploadRow(file);
      progress.append(r.row);
      try {
        const up = await sendFile(file, r.show);
        await cloud.attach(p.tenantId, p.id, up.id, note.value.trim() || null);
        r.done();
        added += 1;
      } catch (e) {
        r.fail(e);
      }
    }
    if (added) {
      toast(added === 1 ? 'Dosya eklendi.' : `${added} dosya eklendi.`);
      note.value = '';
      setTimeout(reload, 900);
    }
  };
  input.addEventListener('change', () => void send(Array.from(input.files ?? [])));
  const zone = h(
    'div',
    { class: 'drop' },
    icon('upload', 22),
    h('div', null, h('strong', null, 'Dosyaları buraya bırakın'), ' ya da seçin'),
    h('div', { class: 'drop-row' }, note, button('Dosya seç', { icon: 'paperclip', onClick: () => input.click() })),
    h('div', { class: 'field-hint' }, 'En çok 512 MB. Bağlantı kesilirse yükleme kaldığı yerden sürer. Program, betik ve makrolu belge alınmaz.'),
    progress,
    input,
  );
  zone.addEventListener('dragover', (e) => {
    e.preventDefault();
    zone.classList.add('is-over');
  });
  zone.addEventListener('dragleave', () => zone.classList.remove('is-over'));
  zone.addEventListener('drop', (e) => {
    e.preventDefault();
    zone.classList.remove('is-over');
    void send(Array.from(e.dataTransfer?.files ?? []));
  });
  return zone;
}

async function revisionsPanel(d: ProjectDetails, r: FileRevisions | null): Promise<HTMLElement | null> {
  const p = d.project;
  if (!r) return null;
  const canDownload = p.access.permissions.includes('project.download');
  return panel(
    'Revizyonlar',
    table(
      [
        { label: 'No', cell: (v) => h('span', { class: 'num' }, `${v.revision}.`), class: 'narrow' },
        { label: 'Kaydeden', cell: (v) => twoLine(v.createdByName, h('span', { class: 'num' }, `${day(v.createdAt)}, ${bytes(v.size)}`)) },
        {
          label: '',
          class: 'narrow',
          cell: (v) =>
            canDownload
              ? h('a', { class: 'icon-btn', href: cloud.revisionUrl(p.tenantId, p.id, v.revision), download: `${p.name}-r${v.revision}.kcad`, title: 'KCAD olarak indir', 'aria-label': `${v.revision}. revizyonu indir` }, icon('download', 18))
              : null,
        },
      ],
      r.revisions.slice(0, 12),
      { empty: empty('project', 'Revizyon yok', 'Proje KentOS’tan buluta kaydedilince revizyonlar burada görünür.') },
    ),
    { note: r.revisions.length > 12 ? `Son 12 / ${r.revisions.length}` : null },
  );
}

/** The projects list a project is reached from: its organisation's, or the professional's own. */
function projectsOf(tenant: string): string | undefined {
  const org = me.value?.organizations.find((o) => o.tenantId === tenant);
  if (org) return `#/kurum/${org.tenantSlug}/projeler`;
  return me.value?.professional ? '#/mesleki/projeler' : undefined;
}

export const projectPage: PageFn = async ({ params, reload }) => {
  const { tenant, project } = params;
  const [d, attachments] = await Promise.all([cloud.details(tenant, project), cloud.attachments(tenant, project)]);
  const p = d.project;
  // A file project's objects are in its revisions, not in the database: its newest revision says how many.
  const r = p.storage === 'file' ? await cloud.revisions(p.tenantId, p.id) : null;
  const [revisions, records] = await Promise.all([revisionsPanel(d, r), recordsPanel(p)]);
  const objects = r ? (r.revisions[0]?.objects ?? '—') : String(d.featureCount);
  const where = p.tenantKind === 'personal' ? 'Kişisel alan' : p.tenantName;
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({
      title: p.name,
      lead: p.description || `${where}. ${p.storage === 'file' ? 'Dosya olarak saklanan proje.' : 'Nesneleri veritabanında saklanan proje.'}`,
      // One who reads the records through forms alone (a Veri girişi) is not given the drawing.
      actions: throughForms(p.access) ? [] : [button('Çizimi indir (SVG)', { icon: 'download', href: cloud.previewUrl(p.tenantId, p.id, 2400) })],
    }),
    h(
      'div',
      { class: 'project-grid' },
      h('div', { class: 'stack' }, panel('Çizim', drawing(p.tenantId, p.id)), records, attachmentsPanel(d, attachments, reload)),
      h(
        'div',
        { class: 'stack' },
        panel(
          'Künye',
          facts([
            ['Çalışma alanı', where],
            ['Saklama', p.storage === 'file' ? 'Dosya (KCAD revizyonları)' : 'Veritabanı'],
            ['Nesne', h('span', { class: 'num' }, objects)],
            ['Katman', h('span', { class: 'num' }, String(d.layerCount))],
            ['Koordinat sistemi', p.srid ? h('span', { class: 'num' }, `EPSG:${p.srid}`) : 'Yerel (atanmamış)'],
            ['Son değişiklik', moment(p.updatedAt)],
            ['Sahibi', p.ownerName],
            ['Erişiminiz', accessView(p.access)],
          ]),
        ),
        revisions,
      ),
    ),
  );
  return { crumbs: [{ label: 'Projeler', href: projectsOf(p.tenantId) }, { label: p.name }], body };
};
