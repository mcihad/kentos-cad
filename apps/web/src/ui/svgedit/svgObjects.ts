import type { SvgShape } from '../../style/svg/svgModel';
import { h } from '../dom';
import { icon } from '../icons';

/**
 * The SVG editor's shape list (front on top): the eye hides a shape, the
 * lock keeps it from being picked or moved on the canvas, a double click
 * on the name renames it, and dragging a row moves the shape in the stack.
 */

export interface ListHost {
  doc: { shapes: SvgShape[] };
  readonly selection: ReadonlySet<string>;
  change(key: string, fn: () => void): void;
  select(ids: string[]): void;
  refresh(): void;
}

const KIND: Record<SvgShape['kind'], string> = { rect: 'Dikdörtgen', ellipse: 'Elips', path: 'Yol', text: 'Yazı' };

export const shapeName = (s: SvgShape) => s.name ?? (s.kind === 'text' ? `Yazı “${s.text}”` : KIND[s.kind]);

export function objectRows(host: ListHost, list: HTMLElement): HTMLElement[] {
  const flip = (id: string, key: 'hidden' | 'locked') =>
    host.change(`${key}:${id}`, () => (host.doc.shapes = host.doc.shapes.map((x) => (x.id === id ? { ...x, [key]: !x[key] || undefined } : x))));
  return [...host.doc.shapes].reverse().map((s) => {
    const eye = h('button', { class: 'ibtn', type: 'button', 'aria-label': s.hidden ? 'Göster' : 'Gizle', title: s.hidden ? 'Göster' : 'Gizle' }, icon(s.hidden ? 'eyeOff' : 'eye', 14));
    eye.addEventListener('click', (e) => {
      e.stopPropagation();
      flip(s.id, 'hidden');
    });
    const lock = h('button', { class: `ibtn svge__lockbtn${s.locked ? ' svge__lockbtn--on' : ''}`, type: 'button', 'aria-pressed': String(!!s.locked), 'aria-label': s.locked ? 'Kilidi aç' : 'Kilitle', title: s.locked ? 'Kilidi aç' : 'Kilitle: tuvalde seçilmez ve taşınmaz' }, icon(s.locked ? 'lock' : 'unlock', 13));
    lock.addEventListener('click', (e) => {
      e.stopPropagation();
      flip(s.id, 'locked');
    });
    const name = h('span', { class: 'svge__name', title: 'Çift tıklayınca adı değişir; sürükleyince sırası' }, shapeName(s));
    const r = h('div', { class: `svge__row${s.hidden ? ' svge__row--hidden' : ''}`, 'aria-selected': String(host.selection.has(s.id)), 'data-shape': s.id }, eye, lock, name, s.group ? h('span', { class: 'svge__grp', title: 'Grupta' }, '▣') : null);
    r.addEventListener('click', (e) => {
      if (dragged) return;
      if ((e as MouseEvent).shiftKey) {
        const next = new Set(host.selection);
        next.has(s.id) ? next.delete(s.id) : next.add(s.id);
        host.select([...next]);
      } else host.select([s.id]);
    });
    name.addEventListener('dblclick', (e) => {
      e.stopPropagation();
      rename(host, s, name);
    });
    r.addEventListener('pointerdown', (e) => startDrag(host, list, s.id, r, e));
    return r;
  });
}

function rename(host: ListHost, s: SvgShape, name: HTMLElement): void {
  const input = h('input', { class: 'field svge__rename', value: s.name ?? '', placeholder: shapeName({ ...s, name: undefined }), 'aria-label': 'Şeklin adı', spellcheck: 'false' });
  let done = false;
  const finish = (keep: boolean) => {
    if (done) return;
    done = true;
    const v = input.value.trim();
    if (keep && v !== (s.name ?? '')) host.change(`name:${s.id}`, () => (host.doc.shapes = host.doc.shapes.map((x) => (x.id === s.id ? { ...x, name: v || undefined } : x))));
    host.refresh();
  };
  input.addEventListener('keydown', (e) => {
    e.stopPropagation();
    if (e.key === 'Enter') finish(true);
    if (e.key === 'Escape') {
      e.preventDefault();
      finish(false);
    }
  });
  input.addEventListener('blur', () => finish(true));
  input.addEventListener('click', (e) => e.stopPropagation());
  input.addEventListener('pointerdown', (e) => e.stopPropagation());
  name.replaceChildren(input);
  input.focus();
  input.select();
}

/** Set while a row was just dragged, so its click does not select. */
let dragged = false;

function startDrag(host: ListHost, list: HTMLElement, id: string, row: HTMLElement, e: PointerEvent): void {
  if (e.button !== 0 || (e.target as Element).closest('button, input')) return;
  const y0 = e.clientY;
  let active = false;
  let drop: number | null = null;
  const line = h('div', { class: 'svge__dropline' });
  dragged = false;
  const rows = () => [...list.querySelectorAll<HTMLElement>('.svge__row')];
  const move = (ev: PointerEvent) => {
    if (!active && Math.abs(ev.clientY - y0) < 4) return;
    if (!active) {
      active = true;
      row.classList.add('svge__row--drag');
      list.append(line);
    }
    // The gap nearest the pointer, 0 = above the first (front) row.
    const rs = rows();
    let k = rs.length;
    for (let i = 0; i < rs.length; i++) {
      const b = rs[i].getBoundingClientRect();
      if (ev.clientY < b.top + b.height / 2) {
        k = i;
        break;
      }
    }
    drop = k;
    const ref = rs[Math.min(k, rs.length - 1)].getBoundingClientRect();
    const top = (k < rs.length ? ref.top : ref.bottom) - list.getBoundingClientRect().top + list.scrollTop;
    line.style.top = `${top - 1}px`;
  };
  const up = () => {
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    line.remove();
    row.classList.remove('svge__row--drag');
    if (!active || drop === null) return;
    dragged = true;
    setTimeout(() => (dragged = false), 0);
    const shapes = host.doc.shapes;
    const n = shapes.length;
    const from = shapes.findIndex((s) => s.id === id);
    // Rows run front to back: gap k sits above row k, i.e. over the shape at n − 1 − k.
    let to = n - drop;
    if (from < to) to--;
    if (from < 0 || to === from) return;
    host.change(`order:${id}`, () => {
      const list2 = [...shapes];
      const [s] = list2.splice(from, 1);
      list2.splice(Math.max(0, Math.min(list2.length, to)), 0, s);
      host.doc.shapes = list2;
    });
    host.refresh();
  };
  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', up);
}
