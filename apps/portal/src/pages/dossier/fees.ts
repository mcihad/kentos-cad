// A dossier's fees and charges (docs/adr/9003), the same tab on both sides: what was assessed, on what basis, whether
// it is paid. The municipality's editors issue an assessment from the tariff in force (its inputs asked here, the lines
// shown as they are typed) and record the payment's receipt; its project managers cancel one with a reason.
import { fees as api } from '../../api/portal';
import type { DossierDetail, FeeAssessment, FeeParameter, FeePreview } from '../../api/types';
import { orgBySlug } from '../../app/state';
import { type Child, h } from '../../lib/dom';
import { day, money, moment, parseDecimal } from '../../lib/format';
import { openDialog } from '../../ui/dialog';
import { form, formError, grid, textField, textareaField, values } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { badge, button, empty, facts, notice, panel } from '../../ui/kit';
import { toast } from '../../ui/toast';
import { linesTable } from '../org/fees';

const STATE: Record<string, { label: string; tone: 'warning' | 'success' | 'neutral' }> = {
  issued: { label: 'Ödenmedi', tone: 'warning' },
  paid: { label: 'Ödendi', tone: 'success' },
  cancelled: { label: 'İptal edildi', tone: 'neutral' },
};

function recordPayment(d: DossierDetail, fee: FeeAssessment, reload: () => void): void {
  const today = new Date().toISOString().slice(0, 10);
  const f = form(
    [
      grid(
        textField({ label: 'Makbuz numarası', name: 'receiptNo', required: true, maxlength: 60 }),
        textField({ label: 'Ödeme günü', name: 'paidOn', type: 'date', value: today }),
      ),
      formError(),
    ],
    async (el) => {
      const v = values(el);
      await api.paid(d.summary.id, fee.id, { receiptNo: v.receiptNo, paidOn: v.paidOn || null });
      dialog.close();
      toast('Ödeme kaydedildi; meslek mensubuna bildirildi.');
      reload();
    },
  );
  const dialog = openDialog({
    title: 'Ödemeyi kaydet',
    lead: `${money(fee.total)}, ${fee.scheduleName}`,
    body: f,
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dialog.close() }), button('Kaydet', { kind: 'primary', icon: 'check', onClick: () => f.requestSubmit() })],
  });
}

function cancelFee(d: DossierDetail, fee: FeeAssessment, reload: () => void): void {
  const f = form(
    [
      textareaField({ label: 'Gerekçe', name: 'reason', required: true, rows: 3, hint: fee.state === 'paid' ? 'Ödenmiş tahakkuk iptal edilir; iadesi veznenin işidir.' : 'Dosyanın geçmişine yazılır.' }),
      formError(),
    ],
    async (el) => {
      await api.cancel(d.summary.id, fee.id, values(el).reason);
      dialog.close();
      toast('Tahakkuk iptal edildi.');
      reload();
    },
  );
  const dialog = openDialog({
    title: 'Tahakkuku iptal et',
    lead: `${money(fee.total)}, ${fee.scheduleName}`,
    body: f,
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dialog.close() }), button('İptal et', { kind: 'danger', onClick: () => f.requestSubmit() })],
  });
}

function assessmentPanel(d: DossierDetail, fee: FeeAssessment, reload: () => void): HTMLElement {
  const st = STATE[fee.state] ?? STATE.issued;
  const actions: Child[] = [];
  if (d.side === 'municipality' && fee.state === 'issued' && (d.canAssessFees || d.canCancelFees)) {
    actions.push(button('Ödemeyi kaydet', { kind: 'secondary', icon: 'check', onClick: () => recordPayment(d, fee, reload) }));
  }
  if (d.canCancelFees && fee.state !== 'cancelled') actions.push(button('İptal et', { kind: 'ghost', icon: 'close', onClick: () => cancelFee(d, fee, reload) }));
  const rows: [string, Child][] = [['Tahakkuk', `${moment(fee.issuedAt)}, ${fee.issuedByName}`]];
  if (fee.dueOn) rows.push(['Son ödeme', day(fee.dueOn)]);
  if (fee.state === 'paid') rows.push(['Ödeme', `${day(fee.paidAt)}, makbuz ${fee.receiptNo ?? '—'}${fee.paidRecordedByName ? ` (${fee.paidRecordedByName} kaydetti)` : ''}`]);
  if (fee.state === 'cancelled') rows.push(['İptal gerekçesi', fee.cancelReason ?? '—']);
  return panel(
    fee.scheduleName,
    h(
      'div',
      { class: 'stack', style: 'gap: 12px' },
      linesTable(fee.lines, fee.total),
      facts(rows),
      fee.state === 'issued' && fee.paymentNote ? notice('info', d.side === 'professional' ? 'Nasıl ödenir' : 'Ödeme notu', fee.paymentNote) : null,
    ),
    { note: badge(st.tone, st.label), actions },
  );
}

/** One input of the tariff as a control; `onInput` gets its value as the server reads it. */
function inputControl(p: FeeParameter, onInput: (v: number | string | boolean | null) => void): HTMLElement {
  const id = `fee-${p.key}`;
  let control: HTMLElement;
  if (p.kind === 'choice') {
    const sel = h('select', { class: 'input', id, name: p.key }, h('option', { value: '' }, 'Seçin'), ...p.choices.map((c) => h('option', { value: c.value }, c.label)));
    sel.addEventListener('change', () => onInput(sel.value || null));
    control = sel;
  } else if (p.kind === 'flag') {
    const box = h('input', { type: 'checkbox', id, name: p.key });
    box.addEventListener('change', () => onInput(box.checked));
    control = h('label', { class: 'check' }, box, ' Evet');
  } else {
    const input = h('input', { class: 'input num', id, name: p.key, inputmode: 'decimal', autocomplete: 'off' });
    input.addEventListener('input', () => onInput(input.value.trim() ? (parseDecimal(input.value) ?? input.value) : null));
    control = p.unit ? h('div', { class: 'fee-range' }, input, h('span', { class: 'muted' }, p.unit)) : input;
  }
  return h(
    'div',
    { class: 'field', dataset: { field: `inputs.${p.key}` } },
    h('label', { class: 'field-label', for: id }, p.label, p.required ? h('span', { class: 'field-req', 'aria-hidden': 'true' }, ' *') : null),
    control,
    p.help ? h('div', { class: 'field-hint' }, p.help) : null,
    h('div', { class: 'field-error', role: 'alert' }),
  );
}

/** The municipality's new assessment: the tariff's inputs for this kind, the lines as they are typed, then issued. */
function issuePanel(d: DossierDetail, first: FeePreview, reload: () => void): HTMLElement {
  const inputs: Record<string, number | string | boolean> = {};
  const lines = h('div', { class: 'stack' });
  // What is still missing is said once something was typed; an empty form only waits.
  const show = (p: FeePreview) =>
    lines.replaceChildren(
      linesTable(p.lines, p.total),
      ...(Object.keys(inputs).length
        ? Object.values(p.missing).map((m) => h('div', { class: 'field-hint' }, icon('alert', 14), ' ', m))
        : [h('div', { class: 'field-hint' }, 'Satırlar girdileri doldurdukça hesaplanır.')]),
    );
  show(first);
  let seq = 0;
  const refresh = async () => {
    const mine = ++seq;
    try {
      const p = await api.preview(d.summary.id, { inputs, dueOn: null });
      if (mine === seq) show(p);
    } catch (e) {
      if (mine === seq) lines.replaceChildren(notice('warning', 'Hesaplanamadı.', e instanceof Error ? e.message : String(e)));
    }
  };
  const f = form(
    [
      h(
        'div',
        { class: 'form-grid' },
        ...first.parameters.map((p) =>
          inputControl(p, (v) => {
            if (v == null) delete inputs[p.key];
            else inputs[p.key] = v;
            void refresh();
          }),
        ),
        textField({ label: 'Son ödeme günü', name: 'dueOn', type: 'date', hint: 'Boş bırakılabilir.' }),
      ),
      lines,
      formError(),
      h('div', { class: 'form-actions' }, button('Tahakkuk et', { kind: 'primary', icon: 'receipt', type: 'submit' })),
    ],
    async (el) => {
      const dueOn = values(el).dueOn || null;
      const made = await api.issue(d.summary.id, { inputs, dueOn });
      toast(`${money(made.total)} tahakkuk etti; meslek mensubuna bildirildi.`);
      reload();
    },
  );
  return panel('Yeni tahakkuk', h('div', { class: 'stack' }, h('div', { class: 'field-hint' }, `${first.scheduleName} ile.`), f));
}

export async function feesTab(d: DossierDetail, slug: string | undefined, reload: () => void): Promise<HTMLElement> {
  const list = await api.ofDossier(d.summary.id);
  const parts: Child[] = [];
  const open = list.filter((f) => f.state === 'issued');
  if (d.side === 'professional' && open.length) {
    parts.push(notice('warning', `Ödenecek: ${money(open.reduce((n, f) => n + f.total, 0))}`, 'Ödeme belediyeye kaydedilince bildirim alırsınız.'));
  }
  // The assessments first; a new one opens by itself only when none is in force.
  const shown = [...list].reverse().map((f) => assessmentPanel(d, f, reload));
  if (d.canAssessFees) {
    try {
      const first = await api.preview(d.summary.id, { inputs: {}, dueOn: null });
      const formPanel = issuePanel(d, first, reload);
      if (list.some((f) => f.state !== 'cancelled')) {
        formPanel.hidden = true;
        const more = h(
          'div',
          null,
          button('Ek tahakkuk', {
            icon: 'plus',
            onClick: () => {
              formPanel.hidden = false;
              more.hidden = true;
              formPanel.querySelector<HTMLElement>('input,select')?.focus();
            },
          }),
        );
        shown.push(more, formPanel);
      } else shown.unshift(formPanel);
    } catch (e) {
      const org = slug ? orgBySlug(slug) : undefined;
      shown.unshift(
        notice(
          'warning',
          'Harç tahakkuk edilemiyor.',
          e instanceof Error ? e.message : String(e),
          org ? button('Harç ve ücretler', { kind: 'ghost', icon: 'receipt', href: `#/kurum/${org.tenantSlug}/harclar` }) : undefined,
        ),
      );
    }
  }
  parts.push(...shown);
  if (!list.length && !d.canAssessFees) {
    parts.push(empty('receipt', 'Tahakkuk yok', d.side === 'professional' ? 'Belediye harç ve ücret tahakkuk ettirdiğinde burada ve bildirimlerinizde görünür.' : 'Bu dosyaya harç tahakkuk edilmedi.'));
  }
  return h('div', { class: 'stack' }, ...parts);
}
