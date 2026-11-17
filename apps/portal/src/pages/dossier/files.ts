// A dossier's packages on its page (docs/adr/9000 §7): the one being prepared (files dropped or chosen, a cloud
// project's present state added), each package as it was sent, and a KentOS project file opened as a project in the
// reader's own workspace or their organisation's.
import { sendFile } from '../../api/files';
import { dossier as api, portal } from '../../api/portal';
import type { DossierDetail, DossierFileView, DossierRound, ProjectSummary } from '../../api/types';
import { catalog } from '../../app/state';
import { type Child, h, replaceChildren } from '../../lib/dom';
import { bytes, moment } from '../../lib/format';
import { openDialog } from '../../ui/dialog';
import { form, formError, selectField, values } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { button, empty, notice, panel } from '../../ui/kit';
import { toast } from '../../ui/toast';
import { scanBadge, uploadRow } from '../../ui/uploads';

function fileIcon(f: DossierFileView): HTMLElement {
  const pdf = f.mediaType === 'application/pdf' || f.name.toLowerCase().endsWith('.pdf');
  return h('span', { class: `file-icon ${f.isProject ? 'is-project' : ''}` }, icon(f.isProject ? 'project' : pdf ? 'pdf' : 'file', 18));
}

/** Opens a KentOS project file as a new project of the reader. */
function importFile(d: DossierDetail, f: DossierFileView): void {
  const member = d.side === 'municipality';
  const where = member ? `${d.organization.name} kurumunun projeleri` : 'kişisel çalışma alanınız';
  const go = button('Proje olarak aç', {
    kind: 'primary',
    onClick: async () => {
      go.setAttribute('disabled', '');
      try {
        const made = await api.importFile(d.summary.id, f.id, member ? d.organization.tenantId : null);
        dlg.close();
        toast(`“${made.name}” projesi açıldı; KentOS'ta bulut projeleri arasında.`);
      } catch (e) {
        go.removeAttribute('disabled');
        replaceChildren(slot, notice('danger', e instanceof Error ? e.message : String(e)));
      }
    },
  });
  const slot = h('div');
  const dlg = openDialog({
    title: 'Proje olarak aç',
    body: h(
      'div',
      { class: 'form' },
      h('p', { class: 'dialog-text' }, `“${f.name}” dosyası ${where} içinde yeni bir dosya projesi olarak açılacak. Dosyadaki çizim olduğu gibi kalır; projede yaptığınız değişiklik bu dosyayı değiştirmez.`),
      slot,
    ),
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), go],
  });
}

function fileRow(d: DossierDetail, f: DossierFileView, reload: () => void): HTMLElement {
  const actions: Child[] = [h('a', { class: 'icon-btn', href: api.fileUrl(d.summary.id, f.id), title: 'İndir', 'aria-label': `${f.name} dosyasını indir`, download: f.name }, icon('download', 18))];
  if (f.isProject && f.state === 'final' && d.canImport) {
    const open = h('button', { class: 'icon-btn', type: 'button', title: 'Proje olarak aç', 'aria-label': 'Proje olarak aç' }, icon('open', 18));
    open.addEventListener('click', () => importFile(d, f));
    actions.unshift(open);
  }
  if (f.state === 'pending') {
    const remove = h('button', { class: 'icon-btn', type: 'button', title: 'Paketten çıkar', 'aria-label': `${f.name} dosyasını paketten çıkar` }, icon('trash', 18));
    remove.addEventListener('click', async () => {
      try {
        await api.removeFile(d.summary.id, f.id);
        reload();
      } catch (e) {
        toast(e instanceof Error ? e.message : String(e), 'danger');
      }
    });
    actions.push(remove);
  }
  const source = f.source ? ` Projeden alındı: ${f.source.projectName}.` : '';
  return h(
    'div',
    { class: 'file' },
    fileIcon(f),
    h(
      'div',
      { class: 'file-text' },
      h('div', { class: 'file-name', title: `${f.name} (SHA-256 ${f.sha256})` }, f.name),
      h('div', { class: 'file-meta' }, `${f.docTypeLabel}, ${bytes(f.size)}. ${f.addedByName} ekledi.${source}`, scanBadge(f.scan, f.scanNote)),
    ),
    h('div', { class: 'file-actions' }, ...actions),
  );
}

/** What a round is called on its page. */
function roundTitle(d: DossierDetail, r: DossierRound): string {
  if (d.summary.direction === 'application') return r.round === 1 ? 'İlk başvuru' : `${r.round}. sürüm (düzeltilmiş)`;
  return r.round === 1 ? 'Belediyenin gönderdiği paket' : `${r.round - 1}. teslim`;
}

/** Adds a cloud project's present state to the package. */
function fromProject(d: DossierDetail, reload: () => void): void {
  const member = d.side === 'municipality';
  const list = h('div', { class: 'pick-list' }, h('div', { class: 'loading' }, 'Projeler yükleniyor…'));
  const f = form(
    [
      h('p', { class: 'dialog-text' }, member ? `${d.organization.name} kurumunun projelerinden birini seçin. Projenin şimdiki hâli bir KCAD dosyası olarak pakete eklenir.` : 'Projelerinizden birini seçin. Projenin şimdiki hâli bir KCAD dosyası olarak pakete eklenir; projede sonra yaptığınız değişiklik pakete geçmez.'),
      list,
      selectField({ label: 'Belge türü', name: 'docType', value: 'kentos_project', options: (catalog.value?.docTypes ?? []).map((t) => ({ value: t.code, label: t.label })) }),
      formError(),
    ],
    async (el) => {
      const picked = el.querySelector<HTMLInputElement>('input[name=project]:checked');
      if (!picked) throw new Error('Bir proje seçin.');
      const [tenantId, projectId] = picked.value.split('/');
      await api.fromProject(d.summary.id, tenantId, projectId, values(el).docType);
      dlg.close();
      toast('Proje pakete eklendi.');
      reload();
    },
  );
  const dlg = openDialog({ title: 'Projeden ekle', wide: true, body: f, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Pakete ekle', { kind: 'primary', onClick: () => f.requestSubmit() })] });
  void portal
    .projects(member ? 'organization' : 'mine', member ? d.organization.tenantId : undefined)
    .then((page) => {
      const rows = page.projects.filter((p: ProjectSummary) => p.state === 'active');
      if (!rows.length) {
        replaceChildren(list, h('div', { class: 'notif-empty' }, member ? 'Kurumun projesi yok. KentOS masaüstünden ya da web uygulamasından buluta proje kaydedin.' : 'Bulutta projeniz yok. KentOS masaüstünden projenizi buluta kaydedin; burada görünür.'));
        return;
      }
      replaceChildren(
        list,
        ...rows.map((p) =>
          h(
            'label',
            { class: 'pick' },
            h('input', { type: 'radio', name: 'project', value: `${p.tenantId}/${p.id}` }),
            h('span', { class: 'file-icon is-project' }, icon('project', 18)),
            h('span', { class: 'party-text' }, h('span', { class: 'party-name' }, p.name), h('span', { class: 'party-sub' }, `${p.storage === 'file' ? 'Dosya projesi' : 'Veritabanı projesi'}, son değişiklik ${moment(p.updatedAt)}`)),
          ),
        ),
      );
    })
    .catch((e: Error) => replaceChildren(list, h('div', { class: 'notif-empty' }, e.message)));
}

function dropZone(d: DossierDetail, reload: () => void): HTMLElement {
  const docTypes = catalog.value?.docTypes ?? [];
  const wanted = d.checklist?.items.filter((i) => !i.present).map((i) => i.docType) ?? [];
  const typeSelect = h(
    'select',
    { class: 'input', 'aria-label': 'Eklenecek dosyaların belge türü' },
    h('option', { value: '' }, 'Belge türünü seçin (isteğe bağlı)'),
    ...docTypes.map((t) => h('option', { value: t.code }, wanted.includes(t.code) ? `${t.label} (eksik)` : t.label)),
  );
  const input = h('input', { type: 'file', multiple: true, hidden: true });
  const status = h('div', { class: 'field-hint' }, 'En çok 200 MB. KentOS proje dosyası (KCAD), PDF, çizim, resim ve ofis belgeleri; program, betik ve makrolu belge alınmaz. Bağlantı kesilirse yükleme kaldığı yerden sürer.');
  const progress = h('div', { class: 'uploads' });
  const send = async (files: File[]) => {
    let added = 0;
    for (const file of files) {
      const r = uploadRow(file);
      progress.append(r.row);
      try {
        const up = await sendFile(file, r.show);
        await api.fromUpload(d.summary.id, up.id, typeSelect.value || null);
        r.done();
        added += 1;
      } catch (e) {
        r.fail(e);
      }
    }
    if (added) setTimeout(reload, 900);
  };
  input.addEventListener('change', () => void send(Array.from(input.files ?? [])));
  const zone = h(
    'div',
    { class: 'drop' },
    icon('upload', 22),
    h('div', null, h('strong', null, 'Dosyaları buraya bırakın'), ' ya da seçin'),
    h('div', { class: 'drop-row' }, typeSelect, button('Dosya seç', { icon: 'paperclip', onClick: () => input.click() }), button('Projeden ekle', { icon: 'project', onClick: () => fromProject(d, reload) })),
    status,
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

/** The packages panel: the one being prepared (when the reader prepares it), then every sent one, newest first. */
export function packagesPanel(d: DossierDetail, reload: () => void): HTMLElement {
  const parts: Child[] = [];
  if (d.canAddFiles) {
    const title = d.summary.direction === 'application' ? (d.summary.round ? `Hazırlanan ${d.pendingRound}. sürüm` : 'Hazırlanan başvuru paketi') : d.side === 'municipality' ? 'Gönderilecek paket' : `Hazırlanan ${d.pendingRound - 1}. teslim`;
    parts.push(
      h(
        'div',
        { class: 'round is-pending' },
        h('div', { class: 'round-head' }, icon('upload', 16), h('strong', null, title), h('span', { class: 'muted' }, 'Gönderene kadar yalnız siz görürsünüz; gönderilen dosya değiştirilmez.')),
        d.pending.length ? h('div', { class: 'files' }, ...d.pending.map((f) => fileRow(d, f, reload))) : null,
        dropZone(d, reload),
      ),
    );
  }
  for (const r of [...d.rounds].reverse()) {
    parts.push(
      h(
        'div',
        { class: 'round' },
        h('div', { class: 'round-head' }, icon('folder', 16), h('strong', null, roundTitle(d, r)), h('span', { class: 'muted' }, r.sentAt ? `${moment(r.sentAt)} gönderildi, ${r.files.length} dosya` : `${r.files.length} dosya`)),
        h('div', { class: 'files' }, ...r.files.map((f) => fileRow(d, f, reload))),
      ),
    );
  }
  if (!parts.length) {
    parts.push(empty('folder', 'Henüz paket yok', d.side === 'professional' && d.summary.direction === 'transfer' ? 'Belediye işi dosyasız gönderdi; ayrıntı açıklamada ve yazışmada.' : 'Gönderilen paketler burada sürüm sürüm görünür.'));
  }
  return panel('Paketler ve belgeler', h('div', null, ...parts), { class: 'panel-flush' });
}
