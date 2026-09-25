import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';

/**
 * Renaming and deleting a cloud project, for the open one (Dosya menu,
 * account menu) or any project of the list (Bulut projesi aç). Both stack
 * over the list, which reads itself again afterwards. Deleting asks first
 * and says what it does: the project is gone for everyone who has access to
 * it, though the server keeps it so the operator can bring it back. The
 * safe button (Vazgeç) has the focus; the deleting one is marked, not amber.
 */

export interface ProjectTarget {
  tenantId: string;
  tenantName: string;
  projectId: string;
  name: string;
}

const reason = (e: unknown): string => {
  if (e instanceof ApiFailure && e.code === 'conflict') return 'Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.';
  return e instanceof Error ? e.message : 'İşlem tamamlanamadı.';
};

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
    footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, save],
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

export function openDeleteDialog(ctx: AppContext, target: ProjectTarget, done?: () => void): void {
  const remove = h('button', { class: 'btn btn--danger', type: 'button' }, 'Projeyi sil');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const isOpen = ctx.cloud.project.value?.projectId === target.projectId;
  const dialog = new Dialog({
    title: 'Bulut projesini sil',
    width: 500,
    className: 'dialog--cloud',
    stack: true,
    content: [
      h('p', null, `“${target.name}” projesi (${target.tenantName}) erişimi olan herkes için silinsin mi?`),
      h(
        'ul',
        { class: 'cloud-consequences' },
        h('li', null, 'Proje listeden kalkar; kimse açamaz ve değiştiremez.'),
        h('li', null, 'Projeyi şu anda açık tutanların kaydı durur; gönderilmemiş değişiklikleri kendi cihazlarında kalır.'),
        h('li', null, 'Nesneler hemen silinmez: yanlışlıkla silinen projeyi sunucu yöneticisi geri getirebilir.'),
        isOpen ? h('li', null, 'Proje şu anda sizde açık: çizim ekranda kalır, dilerseniz yerel bir dosyaya kaydedin.') : null,
      ),
      status,
    ],
    footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, remove],
  });
  const run = async () => {
    remove.disabled = cancel.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = 'Siliniyor…';
    try {
      await ctx.cloud.deleteProject(target.tenantId, target.projectId);
      dialog.close();
      // The open project's own message (the drawing stays on screen) comes from the session.
      if (!isOpen) ctx.log.success(`“${target.name}” bulut projesi silindi.`);
      done?.();
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = reason(e);
      remove.disabled = cancel.disabled = false;
    }
  };
  remove.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  cancel.focus();
}
