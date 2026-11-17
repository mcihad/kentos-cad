// A dossier's steps on its page (docs/adr/9000 §5): the buttons the server allows the reader now, the question each
// asks before it is taken (a refusal and a deficiency list need their words; a decision is said to be the portal's
// record, not the official letter), editing its particulars and removing a draft.
import { dossier as api } from '../../api/portal';
import type { DossierAction, DossierDetail, DossierEdit, KindOption } from '../../api/types';
import { go } from '../../app/router';
import { kinds } from '../../app/state';
import { h } from '../../lib/dom';
import { confirm, openDialog } from '../../ui/dialog';
import { form, formError, grid, orNull, selectField, textareaField, textField, values } from '../../ui/forms';
import { button, notice } from '../../ui/kit';
import { toast } from '../../ui/toast';

type Ask = { title: string; lead?: string; note?: 'required' | 'optional'; noteLabel?: string; noteHint?: string; caution?: string; confirm: string; danger?: boolean; done: string };

function labelOf(a: DossierAction, d: DossierDetail): string {
  const application = d.summary.direction === 'application';
  switch (a) {
    case 'send':
      return application ? (d.summary.round > 0 ? 'Düzeltilmiş başvuruyu gönder' : 'Başvuruyu gönder') : 'İşi gönder';
    case 'withdraw':
      return 'Geri çek';
    case 'start_review':
      return 'İncelemeye al';
    case 'assign':
      return 'Sorumlu ata';
    case 'request_changes':
      return 'Düzeltme iste';
    case 'approve':
      return 'Onayla';
    case 'reject':
      return 'Reddet';
    case 'accept':
      return 'İşi üstlen';
    case 'decline':
      return 'Üstlenme';
    case 'deliver':
      return 'Teslim et';
    case 'request_revision':
      return 'Düzeltme iste';
    case 'complete':
      return 'Teslimi onayla';
    case 'cancel':
      return 'İptal et';
  }
}

const DECISION_CAUTION = 'Bu karar portalda dosyanın kaydına geçer; ıslak ya da elektronik imzalı resmî yazının yerine geçmez.';

function askOf(a: DossierAction, d: DossierDetail): Ask | null {
  const application = d.summary.direction === 'application';
  switch (a) {
    case 'send':
      return application
        ? { title: labelOf(a, d), lead: `${d.organization.name} başvuruyu ve hazırladığınız paketi görecek. Gönderilen dosyalar değiştirilmez.`, note: 'optional', noteLabel: 'Belediyeye not', confirm: 'Gönder', done: 'Başvuru gönderildi.' }
        : { title: 'İşi gönder', lead: 'Meslek mensubu işi ve paketi görecek; üstlenip üstlenmeyeceğini bildirecek.', note: 'optional', noteLabel: 'Meslek mensubuna not', confirm: 'Gönder', done: 'İş gönderildi.' };
    case 'withdraw':
      return { title: 'Başvuruyu geri çek', lead: 'Geri çekilen başvuru kapanır; yeniden başvurmak için yeni bir başvuru açarsınız.', note: 'optional', noteLabel: 'Belediyeye not', confirm: 'Geri çek', danger: true, done: 'Başvuru geri çekildi.' };
    case 'start_review':
      return null;
    case 'assign':
      return null;
    case 'request_changes':
      return {
        title: 'Düzeltme iste',
        lead: 'Başvuru sahibi düzeltilmiş başvuruyu yeni bir sürüm olarak gönderir.',
        note: 'required',
        noteLabel: 'Eksik ve yanlışlar',
        noteHint: 'İmar Kanunu m.22: eksik ve yanlışlar bütün olarak, bir defada yazılır.',
        confirm: 'Düzeltme iste',
        done: 'Düzeltme istendi.',
      };
    case 'approve':
      return { title: 'Başvuruyu onayla', note: 'optional', noteLabel: 'Karar notu', caution: DECISION_CAUTION, confirm: 'Onayla', done: 'Başvuru onaylandı.' };
    case 'reject':
      return { title: 'Başvuruyu reddet', note: 'required', noteLabel: 'Ret gerekçesi', caution: DECISION_CAUTION, confirm: 'Reddet', danger: true, done: 'Başvuru reddedildi.' };
    case 'accept':
      return { title: 'İşi üstlen', lead: `${d.organization.name} işi üstlendiğinizi görecek. Teslim etmek için hazırladığınız dosyaları pakete ekleyip Teslim et'e basarsınız.`, note: 'optional', noteLabel: 'Belediyeye not', confirm: 'Üstlen', done: 'İşi üstlendiniz.' };
    case 'decline':
      return { title: 'İşi üstlenme', note: 'required', noteLabel: 'Neden', confirm: 'Üstlenme', danger: true, done: 'İşi üstlenmediğiniz bildirildi.' };
    case 'deliver':
      return { title: 'İşi teslim et', lead: 'Hazırladığınız paket teslim olarak gider; gönderilen dosyalar değiştirilmez.', note: 'optional', noteLabel: 'Teslim notu', confirm: 'Teslim et', done: 'İş teslim edildi.' };
    case 'request_revision':
      return { title: 'Teslimde düzeltme iste', note: 'required', noteLabel: 'Düzeltilecekler', confirm: 'Düzeltme iste', done: 'Düzeltme istendi.' };
    case 'complete':
      return { title: 'Teslimi onayla', lead: 'İş tamamlanır ve dosya kapanır.', note: 'optional', noteLabel: 'Karar notu', caution: DECISION_CAUTION, confirm: 'Onayla', done: 'Teslim onaylandı; iş tamamlandı.' };
    case 'cancel':
      return { title: 'Aktarımı iptal et', note: 'required', noteLabel: 'İptal nedeni', confirm: 'İptal et', danger: true, done: 'Aktarım iptal edildi.' };
  }
}

async function take(d: DossierDetail, action: DossierAction, extra: { note?: string | null; assigneeId?: string | null } = {}): Promise<void> {
  await api.act(d.summary.id, { action, expectedStatus: d.summary.status, note: extra.note ?? null, assigneeId: extra.assigneeId ?? null, attachments: [] });
}

function run(d: DossierDetail, action: DossierAction, reload: () => void): void {
  if (action === 'start_review') {
    void take(d, action)
      .then(() => {
        toast('Başvuru incelemeye alındı.');
        reload();
      })
      .catch((e: Error) => toast(e.message, 'danger'));
    return;
  }
  if (action === 'assign') {
    const f = form(
      [
        selectField({
          label: 'Sorumlu',
          name: 'assigneeId',
          required: true,
          placeholder: 'Seçin…',
          options: d.assignees.map((a) => ({ value: a.userId, label: a.displayName })),
          hint: 'Kurumda dosyalara bakabilen (en az düzenleyici) üyeler.',
        }),
        formError(),
      ],
      async (el) => {
        await take(d, 'assign', { assigneeId: values(el).assigneeId });
        dlg.close();
        toast('Sorumlu atandı.');
        reload();
      },
    );
    const dlg = openDialog({ title: 'Sorumlu ata', body: f, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Ata', { kind: 'primary', onClick: () => f.requestSubmit() })] });
    return;
  }
  const ask = askOf(action, d);
  if (!ask) return;
  const f = form(
    [
      ask.lead ? h('p', { class: 'dialog-text' }, ask.lead) : null,
      ask.note
        ? textareaField({ label: ask.noteLabel ?? 'Not', name: 'note', rows: ask.note === 'required' ? 6 : 3, required: ask.note === 'required', hint: ask.noteHint })
        : null,
      ask.caution ? notice('info', ask.caution) : null,
      formError(),
    ],
    async (el) => {
      await take(d, action, { note: orNull(values(el).note) });
      dlg.close();
      toast(ask.done);
      reload();
    },
  );
  const dlg = openDialog({
    title: ask.title,
    body: f,
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button(ask.confirm, { kind: ask.danger ? 'danger' : 'primary', onClick: () => f.requestSubmit() })],
  });
}

const FORWARD: DossierAction[] = ['send', 'start_review', 'accept', 'deliver', 'approve', 'complete'];
const STOPPING: DossierAction[] = ['reject', 'decline', 'cancel', 'withdraw'];

/** The step buttons: the first forward step filled, refusals in the danger style, the rest plain. */
export function actionButtons(d: DossierDetail, reload: () => void): HTMLElement[] {
  const primary = d.actions.find((a) => FORWARD.includes(a));
  const out = d.actions.map((a) =>
    button(labelOf(a, d), {
      kind: a === primary ? 'primary' : STOPPING.includes(a) ? 'danger' : 'secondary',
      onClick: () => run(d, a, reload),
    }),
  );
  // The primary step last, at the right edge, where the eye ends.
  return [...out.filter((b) => !b.classList.contains('btn-primary')), ...out.filter((b) => b.classList.contains('btn-primary'))];
}

function kindOptions(list: KindOption[]) {
  return list.map((k) => ({ value: k.code, label: k.label }));
}

/** Editing the dossier's particulars while its side prepares it. */
export function editDossier(d: DossierDetail, reload: () => void): void {
  const s = d.summary;
  const application = s.direction === 'application';
  const f = form(
    [
      grid(
        selectField({ label: 'Tür', name: 'kind', value: s.kind, options: kindOptions(kinds(s.direction)), required: true }),
        textField({ label: 'Başlık', name: 'title', value: s.title, required: true }),
        h('div', { class: 'span-2' }, textareaField({ label: 'Açıklama', name: 'description', value: d.description, rows: 4 })),
        textField({ label: 'Mahalle', name: 'neighbourhood', value: s.site.neighbourhood }),
        h('div', { class: 'form-grid' }, textField({ label: 'Ada', name: 'block', value: s.site.block }), textField({ label: 'Parsel', name: 'parcel', value: s.site.parcel })),
        textField({ label: 'İl', name: 'province', value: s.site.province }),
        textField({ label: 'İlçe', name: 'district', value: s.site.district }),
        h('div', { class: 'span-2' }, textField({ label: 'Adres', name: 'address', value: s.site.address })),
        application
          ? textField({ label: 'Yapı sahibi', name: 'ownerName', value: d.ownerName, hint: 'Yalnız ad soyad; kimlik numarası yazmayın.' })
          : textField({ label: 'İstenen teslim günü', name: 'dueOn', type: 'date', value: s.dueOn }),
      ),
      formError(),
    ],
    async (el) => {
      const v = values(el);
      const input: DossierEdit = {
        kind: v.kind,
        title: v.title,
        description: v.description,
        site: { province: orNull(v.province), district: orNull(v.district), neighbourhood: orNull(v.neighbourhood), block: orNull(v.block), parcel: orNull(v.parcel), address: orNull(v.address) },
        ownerName: application ? orNull(v.ownerName) : null,
        dueOn: application ? null : orNull(v.dueOn),
      };
      await api.edit(s.id, input);
      dlg.close();
      toast('Bilgiler kaydedildi.');
      reload();
    },
  );
  const dlg = openDialog({ title: 'Bilgileri düzenle', body: f, wide: true, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Kaydet', { kind: 'primary', onClick: () => f.requestSubmit() })] });
}

export async function deleteDraft(d: DossierDetail, back: string): Promise<void> {
  const yes = await confirm({ title: 'Taslağı sil', text: `“${d.summary.title}” taslağı ve eklediğiniz dosyalar silinecek. Bu geri alınmaz.`, confirm: 'Sil', danger: true });
  if (!yes) return;
  try {
    await api.remove(d.summary.id);
    toast('Taslak silindi.');
    go(back);
  } catch (e) {
    toast(e instanceof Error ? e.message : String(e), 'danger');
  }
}
