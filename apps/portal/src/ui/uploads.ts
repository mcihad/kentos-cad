// A file on its way to the cloud, shown as a row with its progress (docs/api/README.md §6.1): ready, sent so far,
// checked on the server, then added, or the server's reason it was refused.
import { ApiError } from '../api/client';
import { type Phase, phaseText } from '../api/files';
import { type Child, h } from '../lib/dom';
import { badge } from './kit';

export type UploadRow = {
  row: HTMLElement;
  show: (done: number, total: number, phase: Phase) => void;
  done: (text?: string) => void;
  fail: (e: unknown) => void;
};

export function uploadRow(file: File): UploadRow {
  const fill = h('span', { style: 'width: 0%' });
  const text = h('span', { class: 'upload-text' }, 'hazırlanıyor');
  const row = h('div', { class: 'upload' }, h('span', { class: 'upload-name' }, file.name), h('div', { class: 'upload-bar' }, fill), text);
  return {
    row,
    show: (done, total, phase) => {
      fill.style.width = `${total ? Math.round((done / total) * 100) : 0}%`;
      text.textContent = phaseText(done, total, phase);
    },
    done: (t = 'eklendi') => {
      fill.style.width = '100%';
      row.classList.add('is-done');
      text.textContent = t;
    },
    fail: (e) => {
      row.classList.add('is-failed');
      text.textContent = e instanceof ApiError ? e.message : String(e);
    },
  };
}

/** How far a kept file was checked, told only when it says something: an antivirus passed it, or could not. */
export function scanBadge(scan: string, note: string | null): Child {
  if (scan === 'clean') return h('span', { class: 'scan', title: 'Antivirüs taramasından geçti' }, badge('success', 'Tarandı'));
  if (note) return h('span', { class: 'scan', title: note }, badge('warning', 'Taranamadı'));
  return null;
}
