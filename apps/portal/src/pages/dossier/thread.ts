// A dossier's conversation and timeline on its page (docs/adr/9000 §6): the two sides' messages (a deficiency list and a
// decision's reason marked as such) with their attachments, the box to write one, and every step in a sentence.
import { dossier as api } from '../../api/portal';
import type { DossierDetail, DossierFileView } from '../../api/types';
import { h, replaceChildren } from '../../lib/dom';
import { bytes, moment } from '../../lib/format';
import { icon } from '../../ui/icons';
import { button, empty, panel } from '../../ui/kit';
import { toast } from '../../ui/toast';

const KIND: Record<string, string> = { deficiency: 'Eksik ve yanlışlar', decision: 'Karar gerekçesi' };

function attachmentChip(d: DossierDetail, f: DossierFileView): HTMLElement {
  return h('a', { class: 'chip', href: api.fileUrl(d.summary.id, f.id), download: f.name, title: `${f.name}, ${bytes(f.size)}` }, icon('paperclip', 14), f.name);
}

function composer(d: DossierDetail, reload: () => void): HTMLElement {
  const area = h('textarea', { class: 'input', rows: 3, placeholder: d.side === 'municipality' ? 'Meslek mensubuna yazın…' : 'Belediyeye yazın…', 'aria-label': 'İleti' });
  const files: DossierFileView[] = [];
  const chips = h('div', { class: 'msg-files' });
  const input = h('input', { type: 'file', multiple: true, hidden: true });
  const drawChips = () =>
    replaceChildren(
      chips,
      ...files.map((f) => {
        const x = h('button', { class: 'link-btn', type: 'button', 'aria-label': `${f.name} ekini çıkar` }, icon('close', 12));
        x.addEventListener('click', async () => {
          await api.removeFile(d.summary.id, f.id).catch(() => undefined);
          files.splice(files.indexOf(f), 1);
          drawChips();
        });
        return h('span', { class: 'chip' }, icon('paperclip', 14), f.name, x);
      }),
    );
  input.addEventListener('change', async () => {
    for (const file of Array.from(input.files ?? [])) {
      try {
        files.push(await api.upload(d.summary.id, file, null, 'attachment'));
        drawChips();
      } catch (e) {
        toast(`${file.name}: ${e instanceof Error ? e.message : String(e)}`, 'danger');
      }
    }
    input.value = '';
  });
  const send = button('Gönder', {
    kind: 'primary',
    icon: 'send',
    onClick: async () => {
      if (!area.value.trim()) {
        area.focus();
        return;
      }
      send.setAttribute('disabled', '');
      try {
        await api.message(d.summary.id, area.value, files.map((f) => f.id));
        reload();
      } catch (e) {
        toast(e instanceof Error ? e.message : String(e), 'danger');
        send.removeAttribute('disabled');
      }
    },
  });
  return h(
    'div',
    { class: 'composer' },
    area,
    chips,
    h('div', { class: 'composer-row' }, button('Dosya ekle', { kind: 'ghost', icon: 'paperclip', onClick: () => input.click() }), h('span', { class: 'toolbar-fill' }), send),
    input,
  );
}

export function threadPanel(d: DossierDetail, reload: () => void): HTMLElement {
  const mine = d.side;
  const messages = d.messages.length
    ? h(
        'div',
        { class: 'thread' },
        ...d.messages.map((m) =>
          h(
            'article',
            { class: `msg kind-${m.kind} ${m.side === mine ? 'is-mine' : ''}` },
            h('div', { class: 'msg-head' }, h('strong', null, m.authorName), h('span', null, m.side === 'municipality' ? d.organization.name : 'Meslek mensubu'), h('span', null, moment(m.at))),
            KIND[m.kind] ? h('span', { class: 'msg-kind' }, m.kind === 'deficiency' ? icon('alert', 14) : icon('stamp', 14), KIND[m.kind]) : null,
            h('div', { class: 'msg-body' }, m.body),
            m.attachments.length ? h('div', { class: 'msg-files' }, ...m.attachments.map((f) => attachmentChip(d, f))) : null,
          ),
        ),
      )
    : h('div', { class: 'panel-body' }, empty('message', 'Henüz ileti yok', d.canMessage ? 'Karşı tarafa soru sorun ya da bilgi verin; iletiler dosyanın kaydında kalır.' : 'Dosya gönderildikten sonra iki taraf burada yazışır.'));
  return panel('Yazışma', h('div', null, messages, d.canMessage ? composer(d, reload) : null), {
    class: 'panel-flush',
    note: d.canMessage ? null : d.summary.status === 'draft' ? 'Gönderilince açılır.' : 'Dosya kapandı.',
  });
}

export function timelinePanel(d: DossierDetail): HTMLElement {
  const events = [...d.events].reverse();
  return panel(
    'Geçmiş',
    events.length
      ? h(
          'ol',
          { class: 'timeline' },
          ...events.map((e) => h('li', null, h('span', { class: `dot ${e.side === 'professional' ? 'is-professional' : ''}`, 'aria-hidden': 'true' }), h('div', null, h('div', null, e.text), h('div', { class: 'when' }, moment(e.at))))),
        )
      : empty('clock', 'Kayıt yok', 'Dosyanın her adımı burada tarihiyle görünür.'),
    { class: 'panel-flush' },
  );
}
