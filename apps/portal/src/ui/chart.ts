// A grouped column chart (the portal's figures, docs/adr/9000 §22), drawn as the data-viz rules ask: one axis from a
// single baseline, columns at most 24 px with a 4 px rounded end and square at the baseline, a 2 px gap between
// touching columns, recessive grid, a legend for two or more series, the largest value of each series labelled, and
// a tooltip on hover and keyboard focus for every month (values first). The colours are the validated categorical
// slots as tokens (`--viz-1`, `--viz-2`, light and dark); text never wears them. The page shows the same numbers in
// a table, so nothing is reachable only by hovering.
import { h } from '../lib/dom';

export type Series = { name: string; token: string };
export type Group = { label: string; title: string; values: number[] };

const NS = 'http://www.w3.org/2000/svg';

function svg<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number>): SVGElementTagNameMap[K] {
  const el = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

/** 0, then a round step a little above the largest value: at most five lines. */
function ticks(max: number): number[] {
  if (max <= 0) return [0, 1];
  const raw = max / 4;
  const pow = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * pow).find((s) => s >= raw) ?? raw;
  const out: number[] = [];
  for (let v = 0; v <= max + step * 0.001; v += step) out.push(v);
  if (out[out.length - 1] < max) out.push(out[out.length - 1] + step);
  return out;
}

/** A column with its top corners rounded by `r` and its foot square on the baseline. */
function column(x: number, y: number, w: number, height: number, r: number): string {
  const rr = Math.min(r, w / 2, height);
  const base = y + height;
  return `M${x},${base}V${y + rr}Q${x},${y} ${x + rr},${y}H${x + w - rr}Q${x + w},${y} ${x + w},${y + rr}V${base}Z`;
}

/** The chart drawn at its box's own width (text stays at its size on a wide screen and on a phone). */
export function groupedColumns(groups: Group[], series: Series[], opts: { height?: number; label?: string } = {}): HTMLElement {
  const box = h('div', { class: 'viz-box' });
  let drawn = 0;
  const draw = (width: number) => {
    const w = Math.max(280, Math.round(width));
    if (w === drawn) return;
    drawn = w;
    box.replaceChildren(drawColumns(groups, series, w, opts));
  };
  new ResizeObserver((entries) => draw(entries[0]?.contentRect.width ?? 760)).observe(box);
  draw(760);
  return box;
}

function drawColumns(groups: Group[], series: Series[], W: number, opts: { height?: number; label?: string }): HTMLElement {
  const H = opts.height ?? 240;
  const pad = { top: 22, right: 8, bottom: 26, left: 34 };
  // A narrow box shows every other month's name.
  const every = W / Math.max(groups.length, 1) < 34 ? 2 : 1;
  const plotW = W - pad.left - pad.right;
  const plotH = H - pad.top - pad.bottom;
  const max = Math.max(0, ...groups.flatMap((g) => g.values));
  const marks = ticks(max);
  const top = marks[marks.length - 1] || 1;
  const yOf = (v: number) => pad.top + plotH - (v / top) * plotH;
  const slot = plotW / Math.max(groups.length, 1);
  const gap = 2;
  const bar = Math.max(4, Math.min(24, (slot * 0.7 - gap * (series.length - 1)) / series.length));
  const groupW = bar * series.length + gap * (series.length - 1);

  const root = svg('svg', { viewBox: `0 0 ${W} ${H}`, width: W, height: H, class: 'viz-svg', role: 'img', 'aria-label': opts.label ?? 'Grafik' });
  for (const t of marks) {
    const y = yOf(t);
    root.append(svg('line', { x1: pad.left, x2: W - pad.right, y1: y, y2: y, class: t === 0 ? 'viz-baseline' : 'viz-grid' }));
    const label = svg('text', { x: pad.left - 6, y: y + 4, class: 'viz-tick', 'text-anchor': 'end' });
    label.textContent = t.toLocaleString('tr-TR');
    root.append(label);
  }
  // The largest value of each series is labelled on its column; the rest are in the tooltip and the table.
  const peaks = series.map((_, s) => Math.max(...groups.map((g) => g.values[s] ?? 0)));
  const labelled = series.map(() => false);
  const tooltip = h('div', { class: 'viz-tooltip', role: 'status', hidden: true });
  const wrap = h('div', { class: 'viz' }, root, tooltip);
  groups.forEach((g, i) => {
    const x0 = pad.left + i * slot + (slot - groupW) / 2;
    const cols: SVGPathElement[] = [];
    g.values.forEach((v, s) => {
      const x = x0 + s * (bar + gap);
      if (v > 0) {
        const y = yOf(v);
        const p = svg('path', { d: column(x, y, bar, pad.top + plotH - y, 4), class: 'viz-col', fill: `var(${series[s].token})` });
        cols.push(p);
        root.append(p);
        if (v === peaks[s] && !labelled[s]) {
          labelled[s] = true;
          const t = svg('text', { x: x + bar / 2, y: y - 5, class: 'viz-value', 'text-anchor': 'middle' });
          t.textContent = v.toLocaleString('tr-TR');
          root.append(t);
        }
      }
    });
    if (i % every === 0) {
      const name = svg('text', { x: pad.left + i * slot + slot / 2, y: H - 8, class: 'viz-tick', 'text-anchor': 'middle' });
      name.textContent = g.label;
      root.append(name);
    }
    // The whole slot is the hit target, larger than the columns.
    const hit = svg('rect', { x: pad.left + i * slot, y: pad.top, width: slot, height: plotH, class: 'viz-hit', tabindex: 0 });
    hit.setAttribute('aria-label', `${g.title}: ${series.map((s, k) => `${s.name} ${g.values[k] ?? 0}`).join(', ')}`);
    const show = () => {
      cols.forEach((c) => c.classList.add('is-on'));
      tooltip.replaceChildren(
        h('div', { class: 'viz-tooltip-title' }, g.title),
        ...series.map((s, k) =>
          h('div', { class: 'viz-tooltip-row' }, h('span', { class: 'viz-key', style: `background: var(${s.token})` }), h('strong', null, (g.values[k] ?? 0).toLocaleString('tr-TR')), h('span', null, s.name)),
        ),
      );
      tooltip.hidden = false;
      const share = (pad.left + i * slot + slot / 2) / W;
      tooltip.style.left = `${Math.min(Math.max(share * 100, 12), 88)}%`;
    };
    const hide = () => {
      cols.forEach((c) => c.classList.remove('is-on'));
      tooltip.hidden = true;
    };
    hit.addEventListener('pointerenter', show);
    hit.addEventListener('pointerleave', hide);
    hit.addEventListener('focus', show);
    hit.addEventListener('blur', hide);
    root.append(hit);
  });
  const legend = h(
    'div',
    { class: 'viz-legend' },
    ...series.map((s) => h('span', { class: 'viz-legend-item' }, h('span', { class: 'viz-swatch', style: `background: var(${s.token})` }), s.name)),
  );
  return h('div', { class: 'stack', style: 'gap: 8px' }, series.length > 1 ? legend : null, wrap);
}
