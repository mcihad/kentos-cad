import type { AppContext } from '../../app/context';
import { when } from '../../app/cloud/catalog';
import { pointText } from '../../app/cloud/history';
import { workspaceName } from '../../app/cloud/session';
import type { Checkpoint } from '../../contracts/generated/Checkpoint';
import type { FileRevision } from '../../contracts/generated/FileRevision';
import type { FileRevisions } from '../../contracts/generated/FileRevisions';
import type { ProjectDuplicated } from '../../contracts/generated/ProjectDuplicated';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { reason } from './ProjectActions';
import { creatableWorkspaces } from './ProjectForms';

/**
 * The history's forms (docs/adr/0034, 0038; TODOS.md SYNC-11, CLOUD-07):
 * naming a checkpoint — a database project's present state, or a file
 * project's revision (its newest by default) — and restoring a point of the
 * history (a checkpoint, or a file project's revision) as a new project.
 * Restoring never changes the source: the new project is made like a copy
 * and is the account's own. The server checks every value and right.
 */

/** The project a form is about: where it is, its name and how it is kept. */
export interface HistoryTarget {
  tenantId: string;
  projectId: string;
  name: string;
  storage: ProjectStorage;
}

const NAME_MAX = 120;
const NOTE_MAX = 2000;

/** “Kontrol noktası oluştur”: a name, a note and, in a file project, the revision it names. */
export function openCheckpointDialog(ctx: AppContext, t: HistoryTarget, revisions: FileRevisions | null, done?: (c: Checkpoint) => void): void {
  const name = h('input', { class: 'field', 'aria-label': 'Kontrol noktasının adı', placeholder: 'Örn. Belediyeye teslim', spellcheck: 'false', maxlength: String(NAME_MAX) });
  const note = h('textarea', { class: 'field cloud-textarea', rows: '3', maxlength: String(NOTE_MAX), 'aria-label': 'Not', placeholder: 'İsteğe bağlı: neden, kime, hangi aşama' });
  const list = revisions?.revisions ?? [];
  const revision =
    t.storage === 'file'
      ? h(
          'select',
          { class: 'field', 'aria-label': 'Adlandırılacak revizyon' },
          list.map((r, i) => h('option', { value: r.revision, selected: i === 0 }, `Revizyon ${r.revision}${i === 0 ? ' (en yeni)' : ''} · ${r.createdByName || 'görünmüyor'} · ${when(r.createdAt)}`)),
        )
      : null;
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const make = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, 'Kontrol noktası oluştur');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Kontrol noktası oluştur',
    width: 520,
    className: 'dialog--cloud',
    stack: true,
    content: [
      h(
        'p',
        { class: 'cloud-hint' },
        t.storage === 'file'
          ? `“${t.name}” projesinin seçilen revizyonuna ad verilir; dosya kopyalanmaz. Revizyon, kontrol noktası silinse de kalır.`
          : `“${t.name}” projesinin şimdiki hâli tek bir .kcad olarak saklanır; bundan sonraki değişiklikler onda olmaz. Açık projede gönderilmeyi bekleyen değişiklikler önce gönderilir.`,
      ),
      h('label', { class: 'cloud-field' }, h('span', null, 'Ad'), name),
      h('label', { class: 'cloud-field' }, h('span', null, 'Not'), note),
      revision ? h('label', { class: 'cloud-field' }, h('span', null, 'Revizyon'), revision) : null,
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, make],
  });
  const refresh = () => {
    const n = name.value.trim();
    make.disabled = !n || n.length > NAME_MAX;
  };
  const run = async () => {
    if (make.disabled) return;
    make.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = t.storage === 'file' ? 'Oluşturuluyor…' : 'Projenin görüntüsü alınıyor…';
    try {
      const made = await ctx.cloud.lifecycle.createCheckpoint(t, { name: name.value, note: note.value, ...(revision ? { fileRevision: revision.value } : {}) });
      dialog.close();
      ctx.log.success(`“${made.checkpoint.name}” kontrol noktası oluşturuldu: ${pointText(made.checkpoint)}.`);
      done?.(made.checkpoint);
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = reason(e);
      refresh();
    }
  };
  name.addEventListener('input', refresh);
  name.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      void run();
    }
  });
  make.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  name.focus();
}

/** A point of the history: a checkpoint, or a file project's revision. */
export type HistoryPoint = { checkpoint: Checkpoint } | { revision: FileRevision };

/**
 * “Yeni proje olarak geri yükle”: the point as a new project, with an
 * optional name and a workspace the account may open projects in. The
 * source stays as it is. `done` gets the new project and the mode it is
 * kept as (a file project's point is a file project; a database
 * checkpoint, a database project).
 */
export function openRestoreDialog(ctx: AppContext, t: HistoryTarget, point: HistoryPoint, done?: (made: ProjectDuplicated) => void): void {
  const places = creatableWorkspaces(ctx, t.tenantId);
  const label = 'checkpoint' in point ? point.checkpoint.name : `r${point.revision.revision}`;
  const storage: ProjectStorage = 'checkpoint' in point ? (point.checkpoint.kind === 'revision' ? 'file' : 'database') : 'file';
  const what = 'checkpoint' in point ? `“${point.checkpoint.name}” kontrol noktası (${pointText(point.checkpoint)})` : `revizyon ${point.revision.revision}`;
  const name = h('input', { class: 'field', value: '', placeholder: `${t.name} (${label})`, 'aria-label': 'Yeni projenin adı', spellcheck: 'false', maxlength: '200' });
  const place = h(
    'select',
    { class: 'field', 'aria-label': 'Yeni projenin çalışma alanı', disabled: places.length < 2 },
    places.map((m) => h('option', { value: m.tenantId }, workspaceName(m.tenantKind, m.tenantName, true))),
  );
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const make = h('button', { class: 'btn btn--primary', type: 'button', disabled: !places.length }, 'Yeni proje olarak geri yükle');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Yeni proje olarak geri yükle',
    width: 540,
    className: 'dialog--cloud',
    stack: true,
    content: [
      h('p', null, `“${t.name}” projesinin ${what} yeni bir proje olarak açılır.`),
      h(
        'ul',
        { class: 'cloud-consequences' },
        h('li', null, `“${t.name}” olduğu gibi kalır; kimsenin güncel işi ezilmez.`),
        h('li', null, storage === 'file' ? 'Yeni proje bir dosya projesidir; 1. revizyonu bu noktadır.' : 'Yeni proje bir veritabanı projesidir; nesneler kalıcı kimlikleriyle gelir.'),
        h('li', null, 'Yeni proje sizin olur; geçmiş, paylaşım ve favoriler gelmez. Açıklama, tür ve etiketler gelir.'),
        h('li', null, 'Oluşturulunca açılır.'),
      ),
      h('label', { class: 'cloud-field' }, h('span', null, 'Yeni projenin adı (isteğe bağlı)'), name),
      h('label', { class: 'cloud-field' }, h('span', null, 'Çalışma alanı'), place),
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, make],
  });
  if (!places.length) {
    status.dataset.kind = 'error';
    status.textContent = 'Proje açabileceğiniz bir çalışma alanınız yok (project.create); kurum yöneticinize başvurun.';
  }
  const run = async () => {
    if (make.disabled) return;
    make.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = 'Geri yükleniyor…';
    try {
      const made = await ctx.cloud.lifecycle.restoreCheckpoint(t, 'checkpoint' in point ? { checkpointId: point.checkpoint.id } : { fileRevision: point.revision.revision }, { name: name.value, tenantId: place.value });
      dialog.close();
      ctx.log.success(`“${made.project.name}” oluşturuldu: ${what} geri yüklendi (${Number(made.objects).toLocaleString('tr-TR')} nesne).`);
      done?.(made);
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = reason(e);
      make.disabled = false;
    }
  };
  name.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      void run();
    }
  });
  make.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  name.focus();
}
