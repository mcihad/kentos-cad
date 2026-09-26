import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { day } from '../../app/cloud/catalog';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { h } from '../dom';
import { askRemove, confirmDialog } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';
import { placeOf } from './catalogRows';

/**
 * Renaming a cloud project, and the questions before the lifecycle's
 * actions (docs/adr/0028): moving to the trash, removing for good and
 * archiving, for the open project (Dosya menu, account menu) or any project
 * of the catalog. Each question says what happens to everyone who has the
 * project, what stays and for how long; the safe answer has the focus, and
 * a removing answer is marked, not amber (DESIGN.md §7.9.1). The server
 * decides every request; a refusal is said in its own words.
 */

export interface ProjectTarget {
  tenantId: string;
  tenantName: string;
  projectId: string;
  name: string;
}

/** A catalog entry as the actions name it. */
export function targetOf(ctx: AppContext, p: ProjectSummary): ProjectTarget {
  return { tenantId: p.tenantId, tenantName: placeOf(ctx, p), projectId: p.id, name: p.name };
}

/** A failed request as a sentence: the server's words, or what to do when it gave none. */
export const reason = (e: unknown): string => {
  if (e instanceof ApiFailure && e.code === 'conflict') return 'Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.';
  if (e instanceof ApiFailure && e.code === 'network') return 'Sunucuya ulaşılamadı; bağlantınızı denetleyip yeniden deneyin.';
  return e instanceof Error ? e.message : 'İşlem tamamlanamadı.';
};

const ref = (t: ProjectTarget) => ({ tenantId: t.tenantId, projectId: t.projectId, name: t.name });

export function openRenameDialog(ctx: AppContext, target: ProjectTarget, done?: (name: string) => void): void {
  const field = h('input', { class: 'field', value: target.name, 'aria-label': 'Yeni ad', spellcheck: 'false', maxlength: '200' });
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const save = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, 'Yeniden adlandır');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Bulut projesini yeniden adlandır',
    width: 460,
    className: 'dialog--cloud',
    stack: true,
    content: [
      h('label', { class: 'cloud-field' }, h('span', null, 'Yeni ad'), field),
      h('p', { class: 'cloud-hint' }, 'Projeye erişimi olan herkes yeni adı görür.'),
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
  });
  const refresh = () => {
    const v = field.value.trim();
    save.disabled = !v || v === target.name;
  };
  const run = async () => {
    if (save.disabled) return;
    save.disabled = true;
    const name = field.value.trim();
    status.dataset.kind = 'info';
    status.textContent = 'Kaydediliyor…';
    try {
      const saved = await ctx.cloud.rename(target.tenantId, target.projectId, name);
      dialog.close();
      if (saved) ctx.log.success(`Proje “${name}” olarak yeniden adlandırıldı.`);
      else ctx.log.warn(`Yeni ad (“${name}”) bu cihazda bekliyor; sunucuya ulaşılınca kaydedilir.`);
      done?.(name);
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = reason(e);
      refresh();
    }
  };
  field.addEventListener('input', refresh);
  field.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      void run();
    }
  });
  save.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  field.focus();
  field.select();
}

/**
 * Asks, then moves the project to the trash for everyone (`project.trash`).
 * `retentionDays`: how long the trash keeps it, when the caller knows.
 * Resolves true when it went; a refusal is logged with the server's reason.
 */
export async function trashProject(ctx: AppContext, target: ProjectTarget, retentionDays?: number): Promise<boolean> {
  const isOpen = ctx.cloud.project.value?.projectId === target.projectId;
  const ok = await askRemove({
    title: 'Çöp kutusuna taşı',
    message: `“${target.name}” projesi (${target.tenantName}) erişimi olan herkes için çöp kutusuna taşınsın mı?`,
    details: [
      'Proje listelerden kalkar; kimse açamaz ve değiştiremez.',
      'Projeyi şu anda açık tutanların kaydı durur; gönderilmemiş değişiklikleri kendi cihazlarında kalır.',
      `Hiçbir şey silinmez: proje sahibi ya da kurum yöneticisi ${retentionDays ? `${retentionDays} gün` : 'saklama süresi dolana kadar'} içinde Çöp kutusu’ndan geri yükleyebilir; sonra proje kalıcı olarak silinir.`,
      isOpen ? 'Proje şu anda sizde açık: çizim ekranda kalır, dilerseniz yerel bir dosyaya kaydedin.' : null,
    ].filter((x): x is string => !!x),
    action: 'Çöpe taşı',
  });
  if (!ok) return false;
  try {
    const done = await ctx.cloud.lifecycle.trash(ref(target));
    // The open project's own message (the drawing stays on screen) comes from the session.
    if (!isOpen) ctx.log.success(`“${target.name}” çöp kutusuna taşındı${done.project.purgeAfter ? `; ${day(done.project.purgeAfter)} tarihine kadar geri yüklenebilir` : ''}.`);
    return true;
  } catch (e) {
    ctx.log.error(`“${target.name}” çöp kutusuna taşınamadı: ${reason(e)}`);
    return false;
  }
}

/** The account menu's and the Dosya menu's delete (the open project): the same question and command. */
export function openDeleteDialog(ctx: AppContext, target: ProjectTarget, done?: () => void): void {
  void trashProject(ctx, target).then((went) => went && done?.());
}

/** Asks, then removes a project in the trash for good (`project.purge`, naming it). */
export async function purgeProject(ctx: AppContext, target: ProjectTarget): Promise<boolean> {
  const ok = await askRemove({
    title: 'Kalıcı olarak sil',
    message: `“${target.name}” projesi kalıcı olarak silinsin mi? Bu işlem geri alınamaz.`,
    details: [
      'Nesneler, katmanlar, paylaşımlar, komut günlüğü ve olaylar silinir; yalnız kimin ne zaman sildiğini söyleyen denetim kaydı kalır.',
      'Önceden indirilmiş kopyalar ve sunucu yedekleri bu işlemle silinmez.',
    ],
    action: 'Kalıcı olarak sil',
  });
  if (!ok) return false;
  try {
    const gone = await ctx.cloud.lifecycle.purge(ref(target));
    ctx.log.success(`“${gone.name}” kalıcı olarak silindi (${gone.objects} nesne).`);
    return true;
  } catch (e) {
    ctx.log.error(`“${target.name}” kalıcı olarak silinemedi: ${reason(e)}`);
    return false;
  }
}

/** Asks, then archives the project (`project.archive`): read-only for everyone until it is unarchived. */
export async function archiveProject(ctx: AppContext, target: ProjectTarget): Promise<boolean> {
  const answer = await confirmDialog({
    title: 'Projeyi arşivle',
    message: `“${target.name}” projesi arşivlensin mi?`,
    details: [
      'Proje salt okunur olur: nesneleri, adı ve bilgileri değişmez; açılabilir, paylaşımı değiştirilebilir, kopyası oluşturulabilir.',
      'Projeyi şu anda açık tutanların kaydı durur; gönderilmemiş değişiklikleri kendi cihazlarında kalır.',
      'Arşivlenmişler listesinde durur; proje sahibi ya da yöneticisi arşivden çıkarabilir.',
    ],
    answers: [
      { value: 'stay', label: 'Vazgeç' },
      { value: 'archive', label: 'Arşivle', kind: 'primary' },
    ],
    cancel: 'stay',
  });
  if (answer !== 'archive') return false;
  try {
    await ctx.cloud.lifecycle.archive(ref(target));
    if (ctx.cloud.project.value?.projectId !== target.projectId) ctx.log.success(`“${target.name}” arşivlendi.`);
    return true;
  } catch (e) {
    ctx.log.error(`“${target.name}” arşivlenemedi: ${reason(e)}`);
    return false;
  }
}
