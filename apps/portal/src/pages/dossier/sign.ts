// E-signing a dossier's files from the browser (docs/adr/9004): the server signs a request naming the files and the
// digests of the bytes it keeps; KentOS İmza on this computer checks that request, asks for the card and the PIN in its
// own window and signs; the page hands the result back to the server, which checks it against its own record. The
// page never sees the PIN or the private key, and nothing the page says about the files is believed by the server.
import { signing as api } from '../../api/portal';
import type { DossierDetail, DossierFileView, DossierSignatureView, SigningVerdict } from '../../api/types';
import { ImzaError, imza } from '../../app/imza';
import { type Child, h } from '../../lib/dom';
import { bytes, moment } from '../../lib/format';
import { openDialog } from '../../ui/dialog';
import { checkbox } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { badge, button, facts, notice } from '../../ui/kit';
import { toast } from '../../ui/toast';

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** The certificate holder's name, from its subject (`CN=…`). */
export function holder(subject: string): string {
  return subject.split(',').map((p) => p.trim()).find((p) => p.startsWith('CN='))?.slice(3) ?? subject;
}

/** A file's signatures as a small mark; its details open on a click. */
export function signatureMark(f: DossierFileView, sigs: DossierSignatureView[]): HTMLElement | null {
  if (!sigs.length) return null;
  const broken = sigs.some((s) => !s.stillMatches);
  const names = [...new Set(sigs.map((s) => holder(s.subject)))].join(', ');
  const mark = h(
    'button',
    { class: `sign-mark ${broken ? 'is-broken' : ''}`, type: 'button', title: `E-imzalı: ${names}` },
    icon('signature', 14),
    broken ? 'imzadan sonra değişti' : `e-imzalı: ${names}`,
  );
  mark.addEventListener('click', () => signaturesDialog(f, sigs));
  return mark;
}

function signaturesDialog(f: DossierFileView, sigs: DossierSignatureView[]): void {
  const dlg = openDialog({
    title: `“${f.name}” e-imzaları`,
    lead: 'Sunucu her imzayı kendi sakladığı baytlara ve sertifikanın kendi anahtarına karşı doğruladı.',
    wide: true,
    body: h(
      'div',
      { class: 'stack' },
      ...sigs.map((s) =>
        h(
          'div',
          { class: 'sign-card' },
          h('div', { class: 'party' }, h('span', { class: 'kind-icon' }, icon('signature', 18)), h('div', { class: 'party-text' }, h('span', { class: 'party-name' }, holder(s.subject)), h('span', { class: 'party-sub' }, `${s.side === 'municipality' ? 'Belediye' : 'Meslek mensubu'} tarafı; ${s.signerName} hesabıyla`)), s.stillMatches ? badge('success', 'Belge aynı') : badge('danger', 'Belge imzadan sonra değişti')),
          facts([
            ['İmzalandı', moment(s.signedAt)],
            ['Doğrulandı', moment(s.verifiedAt)],
            ['Sertifika', h('span', { class: 'fee-code' }, s.subject)],
            ['Yayıncı', h('span', { class: 'fee-code' }, s.issuer)],
            ['Seri no', h('span', { class: 'num' }, s.serialNumber)],
            ['Parmak izi (SHA-256)', h('span', { class: 'fee-code' }, s.fingerprintSha256)],
            ['Algoritma', `${s.signatureAlgorithm}, özet ${s.digestAlgorithm}`],
          ]),
        ),
      ),
      notice('info', 'Yasal geçerlilik henüz denetlenmiyor.', 'Kanıtlanan, bu sertifikanın anahtarının bu baytları imzaladığıdır. Sertifikanın kök sertifikalara zinciri ve iptal durumu (OCSP / SİL) bu sürümde denetlenmez.'),
    ),
    footer: [button('Kapat', { kind: 'ghost', onClick: () => dlg.close() })],
  });
}

function verdictView(v: SigningVerdict, names: Map<string, string>): HTMLElement {
  const tone = v.status === 'completed' ? 'success' : v.status === 'partially' ? 'warning' : 'danger';
  const title = v.status === 'completed' ? `${v.signed} belge e-imzalandı.` : v.status === 'partially' ? `${v.required} belgeden ${v.signed} tanesi e-imzalandı.` : 'Hiçbir imza kabul edilmedi.';
  return h(
    'div',
    { class: 'stack' },
    notice(tone, title, v.signer ? `İmzalayan: ${holder(v.signer.subject)}` : (v.reason ?? null)),
    h('ul', { class: 'checklist' }, ...v.files.map((f) => h('li', { class: f.verified ? 'is-present' : '' }, h('span', { class: 'mark', 'aria-hidden': 'true' }, f.verified ? icon('check', 12) : null), h('span', null, names.get(f.fileId) ?? f.fileId, f.error ? h('span', { class: 'field-hint' }, ` (${f.error})`) : null)))),
  );
}

/** The signing of `files` (all checked to begin with), step by step in one window. */
export function signDialog(d: DossierDetail, files: DossierFileView[], reload: () => void): void {
  const names = new Map(files.map((f) => [f.id, f.name]));
  const status = h('div', { class: 'stack' });
  const list = h('div', { class: 'checks sign-files' }, ...files.map((f) => checkbox(h('span', null, f.name, h('span', { class: 'muted' }, ` · ${bytes(f.size)}`)), 'file', true, f.id)));
  let busy = false;
  let cancelled = false;
  let batchId: string | null = null;
  let transaction: string | null = null;
  const go = button('İmzala', { kind: 'primary', icon: 'signature' });
  const stop = button('Vazgeç', { kind: 'ghost' });
  const say = (...children: Child[]) => status.replaceChildren(...children.filter((c): c is HTMLElement => c instanceof HTMLElement));
  const step = (text: string) => say(h('div', { class: 'sign-step' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), text));

  const run = async () => {
    const chosen = [...list.querySelectorAll<HTMLInputElement>('input[name=file]:checked')].map((i) => i.value);
    if (!chosen.length) {
      say(notice('warning', 'İmzalanacak belge seçin.'));
      return;
    }
    busy = true;
    go.setAttribute('disabled', '');
    list.querySelectorAll('input').forEach((i) => ((i as HTMLInputElement).disabled = true));
    try {
      step('KentOS İmza aranıyor…');
      const health = await imza.health();
      if (!health.interactiveAgent) throw new ImzaError('KentOS İmza’nın PIN penceresi çalışmıyor: bilgisayarınızda KentOS İmza uygulamasını başlatın.');
      step('Sunucu imza isteğini hazırlıyor…');
      const req = await api.request(d.summary.id, chosen);
      transaction = req.transactionId;
      const batch = await imza.create(req.signedRequest);
      batchId = batch.id;
      await imza.open(batch.id);
      await imza.sign(batch.id);
      say(notice('info', 'Kartınızı takın ve PIN’inizi KentOS İmza penceresine girin.', 'PIN bu sayfaya hiç gelmez; pencere 3 dakika bekler.'));
      let state = await imza.get(batch.id);
      while (!['DELIVERING', 'FAILED', 'CANCELLED', 'EXPIRED'].includes(state.status) && !cancelled) {
        await sleep(600);
        state = await imza.get(batch.id);
      }
      if (cancelled) return;
      if (state.status !== 'DELIVERING') {
        throw new ImzaError(state.lastError?.userMessage ?? (state.status === 'EXPIRED' ? 'PIN zamanında girilmedi; yeniden deneyin.' : 'İmza tamamlanamadı.'));
      }
      step('Sunucu imzaları doğruluyor…');
      const result = await imza.result(batch.id);
      const verdict = await api.deliver(req.transactionId, result);
      await imza.acknowledge(batch.id, verdict.accepted, verdict.reason).catch(() => undefined);
      say(verdictView(verdict, names));
      stop.querySelector('span')!.textContent = 'Kapat';
      if (verdict.signed) toast(`${verdict.signed} belge e-imzalandı.`);
      busy = false;
      reload();
    } catch (e) {
      busy = false;
      go.removeAttribute('disabled');
      list.querySelectorAll('input').forEach((i) => ((i as HTMLInputElement).disabled = false));
      if (batchId) await imza.cancel(batchId).catch(() => undefined);
      if (transaction) await api.cancel(transaction).catch(() => undefined);
      batchId = null;
      transaction = null;
      say(notice('danger', e instanceof Error ? e.message : String(e), e instanceof ImzaError && e.code === 'unreachable' ? h('a', { href: '#/hesabim?bolum=e-imza' }, 'E-imza ayarları') : null));
    }
  };
  go.addEventListener('click', () => void run());
  stop.addEventListener('click', () => {
    if (busy) {
      cancelled = true;
      if (batchId) void imza.cancel(batchId).catch(() => undefined);
      if (transaction) void api.cancel(transaction).catch(() => undefined);
    }
    dlg.close();
  });
  const dlg = openDialog({
    title: 'Belgeleri e-imzala',
    lead: 'Nitelikli elektronik sertifikanızla; imza sunucuda belgenin kendisine karşı doğrulanır.',
    body: h('div', { class: 'stack' }, list, status),
    footer: [stop, go],
  });
}
