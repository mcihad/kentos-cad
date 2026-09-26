import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { workspaceName } from '../../app/cloud/session';
import { STORAGE_TEXT } from '../../app/cloud/sharing';
import { sizeText } from '../../app/cloud/transfer';
import { UploadFailed, type FileStage } from '../../app/cloud/uploading';
import type { MembershipView } from '../../contracts/generated/MembershipView';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import { ENTITY_KIND_LABEL, type DrawingEntity } from '../../model/entities';
import { h } from '../dom';
import { confirmDialog } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';
import { catalogFields } from './ProjectForms';

/**
 * “Buluta yükle”: the open drawing as a new cloud project in a workspace
 * the account may open projects in (its personal space or an organisation),
 * with the catalog's type, description and tags (docs/adr/0028), kept as the
 * user chooses for good (docs/adr/0031, 0036, 0038):
 *
 * - **veritabanı**: the drawing's `.kcad` is uploaded and imported in one
 *   transaction; every change is then saved by itself;
 * - **dosya**: the drawing becomes revision 1 (“Buluta dosya olarak
 *   kaydet”); Kaydet writes the next one, nothing is saved by itself.
 *
 * The window says each stage (the project created, the drawing written,
 * the upload with how far, the server checking it, the import). If the
 * drawing does not get in, the new project stays empty and the drawing
 * local: the user deletes the empty project or keeps it, and a refused
 * object is named and selected.
 */

/** What the window starts with (a conflict's “Ayrı kopya olarak kaydet” asks for a file project named as a copy). */
export interface UploadPreset {
  storage?: ProjectStorage;
  name?: string;
  tenantId?: string;
}

const STAGE_TEXT: Record<FileStage, string> = {
  creating: 'Proje oluşturuluyor…',
  encoding: 'Çizim KCAD v2 olarak hazırlanıyor…',
  uploading: 'Yükleniyor…',
  verifying: 'Sunucu dosyayı doğruluyor…',
  importing: 'Çizim veritabanına aktarılıyor (tek işlemde)…',
};

/** The object an import refused, in words: its place in the file and what it is. */
export function refusedObjectText(ctx: AppContext, index: number): string {
  const e = ([...ctx.doc.all()] as DrawingEntity[])[index];
  if (!e) return `${index + 1}. nesne`;
  const layer = ctx.doc.layers.get(e.layerId)?.name ?? e.layerId;
  return `${index + 1}. nesne (${ENTITY_KIND_LABEL[e.kind].toLocaleLowerCase('tr')}, “${layer}” katmanı${e.label ? `, etiketi “${e.label}”` : ''})`;
}

export function openUploadDialog(ctx: AppContext, preset: UploadPreset = {}): void {
  const cloud = ctx.cloud;
  const tenants = (cloud.me.value?.memberships ?? []).filter((m) => m.active && m.seat);
  if (!tenants.length) {
    ctx.log.warn('Hiçbir çalışma alanında etkin üyeliğiniz ya da koltuğunuz yok; kurum yöneticinize başvurun.');
    return;
  }
  const creates = (t: MembershipView) => t.capabilities.includes('project.create');
  const near = preset.tenantId ?? cloud.project.value?.tenantId;
  let tenant: MembershipView = tenants.find((t) => t.tenantId === near && creates(t)) ?? tenants.find(creates) ?? tenants[0];
  let storage: ProjectStorage = preset.storage ?? 'database';
  const label = (t: MembershipView) => workspaceName(t.tenantKind, t.tenantName, true);
  const tenantSelect = h(
    'select',
    { class: 'field', 'aria-label': 'Çalışma alanı', disabled: tenants.length < 2 },
    tenants.map((t) => h('option', { value: t.tenantId, selected: t.tenantId === tenant.tenantId }, label(t))),
  );
  const nameField = h('input', { class: 'field', value: preset.name ?? ctx.doc.name.value, 'aria-label': 'Proje adı', spellcheck: 'false', maxlength: '200' });
  const fields = catalogFields();
  // The storage mode, for good (docs/adr/0031): each option says what it means.
  const storageOptions = (['database', 'file'] as const).map((s) => {
    const input = h('input', { type: 'radio', name: 'cloud-storage', value: s, checked: s === storage });
    input.addEventListener('change', () => {
      if (input.checked) {
        storage = s;
        refresh();
      }
    });
    return h('label', { class: 'cloud-storage__option', dataset: { storage: s } }, input, h('span', { class: 'cloud-storage__text' }, h('b', null, STORAGE_TEXT[s].title), h('span', null, STORAGE_TEXT[s].detail)));
  });
  const hint = h('p', { class: 'cloud-hint' });
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const bar = h('span');
  const progress = h('div', { class: 'cloud-progress', hidden: true }, bar);
  const primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Buluta yükle');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Buluta yükle',
    width: 600,
    className: 'dialog--cloud',
    content: [
      h('label', { class: 'cloud-field' }, h('span', null, 'Çalışma alanı'), tenantSelect),
      h('label', { class: 'cloud-field' }, h('span', null, 'Proje adı'), nameField),
      h('fieldset', { class: 'cloud-storage' }, h('legend', null, 'Saklama biçimi (sonradan değişmez)'), storageOptions),
      ...fields.elements,
      hint,
      progress,
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, primary],
  });
  const say = (text: string, kind: 'info' | 'error' = 'info') => {
    status.textContent = text;
    status.dataset.kind = kind;
  };
  const canCreate = () => creates(tenant);
  let running = false;
  const refresh = () => {
    primary.textContent = storage === 'file' ? 'Buluta dosya olarak kaydet' : 'Buluta yükle';
    hint.textContent =
      storage === 'file'
        ? `${ctx.doc.size} nesne, katman ağacı, proje ayarları ve proje stilleri tek bir .kcad dosyası olarak yüklenir ve projenin 1. revizyonu olur. Proje sizin olur; başkaları paylaşımla eklenir. Sonra Kaydet (Ctrl+S) yeni bir revizyon yazar; kendiliğinden kaydedilmez.`
        : `${ctx.doc.size} nesne, katman ağacı, proje ayarları ve proje stilleri yüklenir ve veritabanına tek işlemde aktarılır. Proje sizin olur; başkaları paylaşımla eklenir. Sonra her değişiklik kendiliğinden kaydedilir.`;
    primary.disabled = running || !canCreate() || !nameField.value.trim();
    if (!running && !canCreate())
      say(`“${label(tenant)}” çalışma alanında proje açma yetkiniz yok (project.create); başka bir çalışma alanı seçin ya da kurum yöneticinize başvurun.`, 'error');
    else if (!running) say('');
  };
  const stageSay = (stage: FileStage) => {
    progress.hidden = false;
    if (stage !== 'uploading') say(STAGE_TEXT[stage]);
    if (stage === 'creating' || stage === 'encoding') bar.style.width = '0%';
    if (stage === 'verifying' || stage === 'importing') bar.style.width = '100%';
  };
  const lock = (on: boolean) => {
    running = on;
    tenantSelect.disabled = on || tenants.length < 2;
    nameField.disabled = on;
    for (const o of storageOptions) o.querySelector('input')!.disabled = on;
    refresh();
  };

  /** The drawing did not get into the new project: it stays empty; the user deletes it or keeps it, and a refused object is shown. */
  const failed = async (e: UploadFailed) => {
    const p = e.project;
    const refused = e.refused ? refusedObjectText(ctx, e.refused.index) : null;
    const why = e.failure instanceof ApiFailure ? e.failure.message : e.message;
    say(`“${p.name}” oluşturuldu ama çizim yüklenemedi: ${why}`, 'error');
    const answer = await confirmDialog<'delete' | 'keep'>({
      title: 'Çizim buluta aktarılamadı',
      message: refused
        ? `Sunucu çizimin ${refused} nesnesini almadı: ${why} Hiçbir nesne yazılmadı.`
        : `“${p.name}” projesi oluşturuldu, ama çizim içine yüklenemedi: ${why}`,
      details: [
        `“${p.name}” sunucuda boş duruyor; çiziminiz bu cihazda olduğu gibi, kaydedilmemiş değişiklikleriyle.`,
        'Boş projeyi sil: proje kalıcı olarak silinir.',
        refused
          ? 'Çizimi yerelde tut: proje boş kalır; reddedilen nesne seçilir, düzeltip yeniden yükleyebilirsiniz.'
          : 'Çizimi yerelde tut: proje boş kalır; bağlantı ya da neden düzelince yeniden yükleyebilirsiniz.',
      ],
      answers: [
        { value: 'delete', label: 'Boş projeyi sil', kind: 'danger', aside: true },
        { value: 'keep', label: 'Çizimi yerelde tut', kind: 'primary' },
      ],
      cancel: 'keep',
    });
    if (answer === 'delete') {
      const ref = { tenantId: p.tenantId, projectId: p.id, name: p.name };
      try {
        await cloud.lifecycle.trash(ref);
        await cloud.lifecycle.purge(ref);
        ctx.log.info(`Boş “${p.name}” projesi silindi; çizim bu cihazda duruyor.`);
      } catch (x) {
        ctx.log.warn(`Boş “${p.name}” projesi silinemedi (${x instanceof Error ? x.message : String(x)}); Bulut projeleri’nden silebilirsiniz.`);
      }
    } else ctx.log.info(`“${p.name}” bulut projesi boş kaldı; çizim bu cihazda duruyor.`);
    dialog.close();
    // The refused object, selected and shown, to be fixed.
    if (e.refused) {
      const entity = ([...ctx.doc.all()] as DrawingEntity[])[e.refused.index];
      if (entity) {
        ctx.selection.set([entity.id]);
        ctx.view.zoomToSelection();
      }
    }
  };

  const run = async () => {
    if (primary.disabled) return;
    lock(true);
    const name = nameField.value.trim();
    const onProgress = (done: number, total: number) => {
      progress.hidden = false;
      bar.style.width = `${total ? Math.round((done / total) * 100) : 100}%`;
      say(storage === 'file' || done > 1e4 ? `Yükleniyor: %${total ? Math.round((done / total) * 100) : 100} (${sizeText(done)} / ${sizeText(total)})` : `${done} / ${total} nesne`);
    };
    try {
      const ok = storage === 'file' ? await cloud.uploadFile(tenant.tenantId, name, onProgress, fields.read(), stageSay) : await cloud.upload(tenant.tenantId, name, onProgress, fields.read(), stageSay);
      if (ok) dialog.close();
      else lock(false);
    } catch (e) {
      lock(false);
      progress.hidden = true;
      if (e instanceof UploadFailed) return void failed(e);
      say(e instanceof ApiFailure || e instanceof Error ? e.message : 'Yükleme tamamlanamadı.', 'error');
    }
  };
  tenantSelect.addEventListener('change', () => {
    tenant = tenants.find((t) => t.tenantId === tenantSelect.value) ?? tenant;
    refresh();
  });
  nameField.addEventListener('input', refresh);
  primary.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  refresh();
  nameField.select();
}
