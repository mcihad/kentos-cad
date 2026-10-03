import type { AppContext } from '../../app/context';
import { effectiveWorkspace } from '../../app/workspaces';
import type { Workspace } from '../../model/projectSettings';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { workspacePicker } from './workspacePicker';

/**
 * The type of a project opened without one (docs/adr/0165 §1): written
 * before types, or under the former Hibrit mode. Asked once per open, with
 * the CAD and CBS cards (the app's default for new projects chosen). Seç
 * writes it to the project: an edit, not an undo step, as any project
 * setting (a cloud project sends it). Sonra, Esc or × leave it unasked: the
 * project shows as CBS meanwhile and the next open asks again. Another
 * drawing put on screen closes the question unanswered.
 */
export function openProjectTypeDialog(ctx: AppContext): void {
  let choice: Workspace = effectiveWorkspace(ctx.prefs.defaultWorkspace.value).id;
  let answered = false;
  const pick = h('button', { class: 'btn btn--primary', type: 'button' }, 'Seç');
  const later = h('button', { class: 'btn', type: 'button' }, 'Sonra');
  const name = ctx.doc.name.value;
  const dialog = new Dialog({
    title: 'Proje türü',
    width: 640,
    className: 'dialog--projtype',
    content: [
      h(
        'p',
        { class: 'dialog__lead' },
        `“${name}” projesinin türü henüz seçilmedi. Sahne, eksenler ve şerit türe göredir; çizimin verisi değişmez. Seçtiğiniz tür projeye yazılır; Proje ayarları’ndan değiştirilebilir.`,
      ),
      workspacePicker({ value: choice, onChange: (id) => (choice = id), soon: false }),
    ],
    footer: [h('div', { class: 'dialog__spacer' }), later, pick],
    onClose: () => {
      off();
      if (!answered) ctx.log.info(`“${name}” projesinin türü seçilmedi; CBS olarak gösteriliyor. Durum çubuğundaki türden ya da Proje ayarları’ndan seçilir.`);
    },
  });
  // Another drawing on screen: the question was about the one before.
  const off = ctx.doc.events.on('reset', () => {
    answered = true;
    dialog.close();
  });
  later.addEventListener('click', () => dialog.close());
  pick.addEventListener('click', () => {
    answered = true;
    ctx.commands.execute(`workspace.${choice}`);
    dialog.close();
  });
  pick.focus();
}
