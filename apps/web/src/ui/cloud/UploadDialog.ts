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
 * drawing does not get in, it stays local and the window says why, as the
 * desktop's does: refused for good, the empty project is moved to the trash
 * and a refused object is named and selected; with no answer the project
 * is kept, and trying again (the same upload's idempotency key) fills it.
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
  // The upload's idempotency key, kept while the same upload is tried again: a project kept after no answer is
  // found and filled, never a second one made. What is asked for changes, or the project went to the trash: a new key.
  let key = crypto.randomUUID();
  let asked = '';
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

  /**
   * The drawing did not get into the new project: the window stays open and
   * says why (and the log keeps it). Refused for good, the empty project is
   * in the trash, and a refused object is selected to be fixed; with no
   * answer the project waits on the server for the same upload to be tried
   * again.
   */
  const failed = (e: UploadFailed) => {
    const p = e.project;
    const refused = e.refused ? refusedObjectText(ctx, e.refused.index) : null;
    const why = (e.failure instanceof ApiFailure ? e.failure.message : e.message).replace(/\.?$/, '.');
    const text =
      e.left === 'trashed'
        ? refused
          ? `Sunucu çizimin ${refused} nesnesini almadı: ${why} Hiçbir nesne yazılmadı; “${p.name}” projesi çöp kutusuna taşındı. Reddedilen nesne çizimde seçildi; düzeltip yeniden yükleyebilirsiniz.`
          : `Çizim buluta aktarılamadı: ${why} “${p.name}” projesi çöp kutusuna taşındı; çiziminiz bu cihazda olduğu gibi duruyor.`
        : `“${p.name}” oluşturuldu ama çizim yüklenemedi: ${why} Proje sunucuda boş duruyor; yeniden denerseniz aynı proje kullanılır.`;
    say(text, 'error');
    ctx.log.warn(text);
    // In the trash, the project is not found again: the next try makes a new one.
    if (e.left === 'trashed') asked = '';
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
    const now = JSON.stringify([tenant.tenantId, name, storage, fields.read()]);
    if (now !== asked) {
      asked = now;
      key = crypto.randomUUID();
    }
    const onProgress = (done: number, total: number) => {
      progress.hidden = false;
      bar.style.width = `${total ? Math.round((done / total) * 100) : 100}%`;
      say(storage === 'file' || done > 1e4 ? `Yükleniyor: %${total ? Math.round((done / total) * 100) : 100} (${sizeText(done)} / ${sizeText(total)})` : `${done} / ${total} nesne`);
    };
    try {
      // False: the project was made and holds the drawing as it went up, but the drawing changed on its way and
      // stays local (the log says so). The window closes either way: another press would make a second project.
      if (storage === 'file') await cloud.uploadFile(tenant.tenantId, name, onProgress, fields.read(), stageSay, key);
      else await cloud.upload(tenant.tenantId, name, onProgress, fields.read(), stageSay, key);
      dialog.close();
    } catch (e) {
      lock(false);
      progress.hidden = true;
      if (e instanceof UploadFailed) return failed(e);
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
