import type { AppContext } from '../../app/context';
import type { AccessLost } from '../../app/cloud/session';
import { confirmDialog } from '../widgets/confirm';

/**
 * The notice when this account's access to the open cloud project is taken
 * away (TODOS.md CLOUD-13, CLAUDE.md §21): what happened, what stays (the
 * drawing on screen and its unsent edits in this device's draft) and what
 * can be done now: save a local copy, or ask for the project to be shared
 * again (opening it then brings the edits back). The server already refuses
 * whatever this browser would still send.
 */
export function openAccessLostNotice(ctx: AppContext, lost: AccessLost): void {
  const why = lost.reason || 'paylaşımınız kaldırıldı ya da süresi doldu';
  void confirmDialog({
    title: 'Projeye erişiminiz kaldırıldı',
    message: `“${lost.name}” projesine artık erişemiyorsunuz: ${why.replace(/\.$/, '')}. Değişiklikleriniz bundan sonra buluta kaydedilmez.`,
    details: [
      lost.unsent
        ? `Çizim ekranda kalıyor; gönderilmemiş ${lost.unsent} değişiklik bu cihazda saklanıyor.`
        : lost.keeps
          ? 'Çizim ekranda kalıyor; bundan sonraki değişiklikleriniz bu cihazda saklanır.'
          : 'Çizim ekranda kalıyor.',
      'Çizimi saklamak için yerel bir .kcad dosyasına kaydedin.',
      'Erişime yeniden ihtiyacınız varsa proje sahibine ya da yöneticisine başvurun; proje yeniden paylaşılıp açıldığında bu cihazdaki değişiklikler geri gelir.',
    ],
    answers: [
      { value: 'close', label: 'Tamam' },
      { value: 'save', label: 'Yerel kopya kaydet…', kind: 'primary' },
    ],
    cancel: 'close',
  }).then((answer) => answer === 'save' && ctx.commands.execute('file.saveAs'));
}
