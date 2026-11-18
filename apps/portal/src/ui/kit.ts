// The portal's building blocks (DESIGN.md §7, §13): buttons, a status badge that never speaks by colour alone, a page
// header, panels, a data table, figures, empty states and tabs. One accent-filled button per view, at most.
import type { LegalClock } from '../api/types';
import { statusTone, type Tone } from '../app/state';
import { daysLeft } from '../lib/format';
import { type Child, h } from '../lib/dom';
import { type IconName, icon } from './icons';

export type ButtonKind = 'primary' | 'secondary' | 'ghost' | 'danger';

export function button(
  label: string,
  opts: { kind?: ButtonKind; icon?: IconName; onClick?: (e: MouseEvent) => void; disabled?: boolean; title?: string; href?: string; type?: 'button' | 'submit' } = {},
): HTMLElement {
  const kind = opts.kind ?? 'secondary';
  const children: Child[] = [opts.icon ? icon(opts.icon, 16) : null, h('span', null, label)];
  if (opts.href && !opts.disabled) {
    return h('a', { class: `btn btn-${kind}`, href: opts.href, title: opts.title ?? null }, ...children);
  }
  const b = h('button', { class: `btn btn-${kind}`, type: opts.type ?? 'button', title: opts.title ?? null, disabled: opts.disabled ?? false }, ...children);
  if (opts.onClick) b.addEventListener('click', opts.onClick);
  return b;
}

export function iconButton(name: IconName, label: string, onClick: (e: MouseEvent) => void, opts: { badge?: number } = {}): HTMLButtonElement {
  const b = h(
    'button',
    { class: 'icon-btn', type: 'button', 'aria-label': label, title: label },
    icon(name, 18),
    opts.badge ? h('span', { class: 'icon-btn-badge num' }, opts.badge > 99 ? '99+' : String(opts.badge)) : null,
  );
  b.addEventListener('click', onClick);
  return b;
}

const TONE_MARK: Record<Tone, string> = { neutral: '•', info: '•', warning: '⚠', success: '✓', danger: '✕' };

/** A tone's badge: its sign and its words (a status is never told by colour alone, DESIGN.md §11). */
export function badge(tone: Tone, label: string): HTMLElement {
  return h('span', { class: `badge tone-${tone}` }, h('span', { class: 'badge-mark', 'aria-hidden': 'true' }, TONE_MARK[tone]), label);
}

export function statusBadge(code: string, label: string): HTMLElement {
  return badge(statusTone(code), label);
}

/** A legal or requested deadline: what is due, when, and how long is left. */
export function clockChip(clock: LegalClock): HTMLElement {
  const tone: Tone = clock.daysLeft < 0 ? 'danger' : clock.daysLeft <= 3 ? 'warning' : 'neutral';
  return h(
    'span',
    { class: `clock tone-${tone}`, title: clock.basis },
    icon(tone === 'neutral' ? 'clock' : 'alert', 14),
    h('span', null, `${clock.label}: ${daysLeft(clock.daysLeft)}`),
  );
}

export function pageHeader(opts: { title: string; lead?: Child; back?: { label: string; href: string }; actions?: Child[]; aside?: Child }): HTMLElement {
  return h(
    'header',
    { class: 'page-head' },
    opts.back ? h('a', { class: 'page-back', href: opts.back.href }, icon('arrowLeft', 16), opts.back.label) : null,
    h(
      'div',
      { class: 'page-head-row' },
      h('div', { class: 'page-head-text' }, h('h1', null, opts.title), opts.lead ? h('p', { class: 'page-lead' }, opts.lead) : null),
      opts.actions?.length ? h('div', { class: 'page-actions' }, ...opts.actions) : null,
    ),
    opts.aside ?? null,
  );
}

/** A bordered panel with its heading (no shadow: panels lie flat, DESIGN.md §5.4). */
export function panel(title: string | null, body: Child, opts: { actions?: Child[]; class?: string; note?: Child } = {}): HTMLElement {
  return h(
    'section',
    { class: `panel ${opts.class ?? ''}` },
    title || opts.actions?.length
      ? h(
          'div',
          { class: 'panel-head' },
          title ? h('h2', null, title) : h('span'),
          opts.note ? h('span', { class: 'panel-note' }, opts.note) : null,
          opts.actions?.length ? h('div', { class: 'panel-actions' }, ...opts.actions) : null,
        )
      : null,
    h('div', { class: 'panel-body' }, body),
  );
}

export type Column<T> = { label: string; cell: (row: T) => Child; class?: string };

/** A data table; a row with `href` opens its page. */
export function table<T>(columns: Column<T>[], rows: T[], opts: { empty?: Child; href?: (row: T) => string | null; class?: string } = {}): HTMLElement {
  if (!rows.length && opts.empty) return h('div', { class: 'table-empty' }, opts.empty);
  return h(
    'div',
    { class: `table-wrap ${opts.class ?? ''}` },
    h(
      'table',
      { class: 'table' },
      h('thead', null, h('tr', null, ...columns.map((c) => h('th', { class: c.class ?? null }, c.label)))),
      h(
        'tbody',
        null,
        ...rows.map((row) => {
          const href = opts.href?.(row) ?? null;
          const tr = h('tr', { class: href ? 'row-link' : null }, ...columns.map((c) => h('td', { class: c.class ?? null }, c.cell(row))));
          if (href) {
            tr.tabIndex = 0;
            tr.addEventListener('click', (e) => {
              if ((e.target as HTMLElement).closest('a,button,input,select,label')) return;
              location.hash = href;
            });
            tr.addEventListener('keydown', (e) => {
              if (e.key === 'Enter') location.hash = href;
            });
          }
          return tr;
        }),
      ),
    ),
  );
}

/** Two lines in a cell: the name and what it is. */
export function twoLine(main: Child, sub: Child): HTMLElement {
  return h('div', { class: 'two-line' }, h('div', { class: 'two-line-main' }, main), sub ? h('div', { class: 'two-line-sub' }, sub) : null);
}

export function empty(name: IconName, title: string, text: Child, action?: Child): HTMLElement {
  return h('div', { class: 'empty' }, h('div', { class: 'empty-icon' }, icon(name, 22)), h('h3', null, title), h('p', null, text), action ?? null);
}

/** A figure of a dashboard: its value large, what it counts under it. */
export function stat(opts: { label: string; value: string | number; hint?: Child; href?: string; tone?: Tone; icon?: IconName }): HTMLElement {
  const body: Child[] = [
    h('div', { class: 'stat-label' }, opts.icon ? icon(opts.icon, 16) : null, opts.label),
    h('div', { class: `stat-value num ${opts.tone ? `tone-text-${opts.tone}` : ''}` }, String(opts.value)),
    opts.hint ? h('div', { class: 'stat-hint' }, opts.hint) : null,
  ];
  return opts.href ? h('a', { class: 'stat stat-link', href: opts.href }, ...body) : h('div', { class: 'stat' }, ...body);
}

export type Tab = { key: string; label: string; count?: number; href: string };

export function tabs(items: Tab[], active: string): HTMLElement {
  return h(
    'nav',
    { class: 'tabs', role: 'tablist' },
    ...items.map((t) =>
      h(
        'a',
        { class: `tab ${t.key === active ? 'is-active' : ''}`, href: t.href, role: 'tab', 'aria-selected': t.key === active ? 'true' : 'false' },
        t.label,
        t.count !== undefined ? h('span', { class: 'tab-count num' }, String(t.count)) : null,
      ),
    ),
  );
}

export function avatar(name: string, initials: string, size: 'sm' | 'md' = 'md'): HTMLElement {
  return h('span', { class: `avatar avatar-${size}`, title: name, 'aria-hidden': 'true' }, initials);
}

/** A list of labelled values. */
export function facts(rows: [string, Child][]): HTMLElement {
  return h('dl', { class: 'facts' }, ...rows.flatMap(([k, v]) => [h('dt', null, k), h('dd', null, v ?? '—')]));
}

/** A note across the page: information, a warning (with its sign) or a refusal. */
export function notice(tone: Tone, title: string, text?: Child, action?: Child): HTMLElement {
  const sign: IconName = tone === 'warning' || tone === 'danger' ? 'alert' : tone === 'success' ? 'check' : 'clock';
  return h(
    'div',
    { class: `notice tone-${tone}`, role: tone === 'danger' ? 'alert' : 'status' },
    icon(sign, 18),
    h('div', { class: 'notice-text' }, h('strong', null, title), text ? h('div', null, text) : null),
    action ?? null,
  );
}

export function loading(): HTMLElement {
  return h('div', { class: 'loading', role: 'status' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), 'Yükleniyor…');
}
