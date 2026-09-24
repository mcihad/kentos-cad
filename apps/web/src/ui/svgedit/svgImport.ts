import { svgText } from '../../style/svg/exportSvg';
import { importSummary, mapColors, type ColorTarget } from '../../style/svg/importSvg';
import { isNearBlack } from '../../style/svg/svgValues';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { readSvg } from './readSvg';
import type { SvgFiles } from './svgFile';

/**
 * The import window (Inkscape's SVG import dialog): the file's drawing as
 * it will come in, what was kept, flattened or left out, whether it opens
 * as a new drawing or is added to this one (fitted to the canvas), and
 * which of its colours become the symbol's colour and the second colour.
 */

const n = (v: number) => String(Math.round(v * 100) / 100);

export function openImportDialog(files: SvgFiles, text: string, name: string): void {
  const host = files.host;
  const raw = readSvg(text, { symbolColor: null });
  if ('error' in raw) return host.status(`“${name}”: ${raw.error}`, 'warn');
  const { report, colors } = raw;
  let symbol: ColorTarget = report.symbolPaint ? null : 'black';
  let second: string | null = null;
  let mode: 'new' | 'add' = host.doc.shapes.length ? 'add' : 'new';
  const dominant = colors[0]?.color ?? null;
  const isSymbol = (c: string) => (symbol === 'black' ? isNearBlack(c) : symbol === 'dominant' ? c === dominant : symbol === c);

  const preview = h('img', { class: 'svgf__img', alt: 'İçe alınacak çizim' });
  const size = h('div', { class: 'sdf__hint num' });
  const modeHost = h('div');
  const symbolRow = h('div', { class: 'svgf__chips' });
  const secondRow = h('div', { class: 'svgf__chips' });
  const primary = h('button', { class: 'btn btn--primary', type: 'button' });
  const { done, lost } = importSummary(report);
  const summary = h(
    'div',
    { class: 'svgf__summary' },
    h('div', { class: 'svgf__done' }, icon('success', 14), `${done}.`),
    lost.map((l) => h('div', { class: 'svgf__lost' }, icon('warning', 14), l)),
    raw.skipped.length ? h('div', { class: 'svgf__lost' }, icon('warning', 14), `Tanınmayan öğeler atlandı: ${raw.skipped.map((t) => `<${t}>`).join(', ')}`) : null,
  );

  const chip = (label: string, on: boolean, run: () => void, color?: string, title?: string) => {
    const b = h('button', { class: 'svgf__chip', type: 'button', 'aria-pressed': String(on), title: title ?? label }, color ? h('span', { class: 'svgf__swatch', style: `background:${color}` }) : null, label);
    b.addEventListener('click', run);
    return b;
  };
  const render = () => {
    const mapped = mapColors(raw.doc, symbol, second);
    preview.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgText(mapped, { colors: { ink: host.options.ink, second: host.options.second } }))}`;
    preview.style.background = host.options.paper;
    size.textContent = `${n(raw.doc.width)} × ${n(raw.doc.height)} birim${raw.doc.sizeMm ? ` · ${n(raw.doc.sizeMm)} mm genişlik` : ''}`;
    replaceChildren(
      modeHost,
      segmented<'new' | 'add'>({
        label: 'Nasıl alınsın',
        options: [
          { value: 'new', label: 'Yeni çizim olarak aç', hint: 'Tuval dosyanınki olur; kaydedince yeni çizim yazılır' },
          { value: 'add', label: 'Bu çizime ekle', hint: 'Şekiller bu tuvale sığdırılarak eklenir' },
        ],
        value: mode,
        onChange: (v) => ((mode = v), render()),
      }),
    );
    const fixed = colors.slice(0, 12);
    replaceChildren(
      symbolRow,
      chip('Siyah', symbol === 'black', () => ((symbol = 'black'), render()), '#000000', 'Siyah ve siyaha yakın renkler sembol rengi olur'),
      dominant ? chip('Baskın renk', symbol === 'dominant', () => ((symbol = 'dominant'), render()), dominant, 'En çok alanı boyayan renk sembol rengi olur') : null,
      chip('Hiçbiri', symbol === null, () => ((symbol = null), render()), undefined, 'Renkler sabit kalır'),
      fixed.map((c) => chip(c.color, isSymbol(c.color), () => ((symbol = c.color), second === c.color && (second = null), render()), c.color, `${c.color}: ${c.count} kez`)),
    );
    replaceChildren(
      secondRow,
      chip('Hiçbiri', second === null, () => ((second = null), render())),
      fixed.filter((c) => !isSymbol(c.color)).map((c) => chip(c.color, second === c.color, () => ((second = c.color), render()), c.color, `${c.color}: ${c.count} kez`)),
    );
    replaceChildren(primary, icon('check', 16), mode === 'new' ? 'Yeni çizim olarak aç' : 'Bu çizime ekle');
  };

  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: `SVG içe al: ${name}`,
    width: 820,
    className: 'dialog--svgfile',
    content: [
      h(
        'div',
        { class: 'svgf__cols' },
        h('div', { class: 'svgf__preview' }, preview, size),
        h(
          'div',
          { class: 'svgf__form' },
          h('div', { class: 'svgf__field' }, h('span', { class: 'sdf__label' }, 'Nasıl alınsın'), modeHost),
          h('div', { class: 'svgf__field' }, h('span', { class: 'sdf__label' }, 'Sembol rengi olacak renk'), symbolRow, h('p', { class: 'sdf__hint' }, 'Haritada sembol hangi rengi verirse bu renkle boyanan yerler o renge döner (currentColor).')),
          h('div', { class: 'svgf__field' }, h('span', { class: 'sdf__label' }, 'İkinci renk olacak renk'), secondRow, h('p', { class: 'sdf__hint' }, 'Sembolün ikinci rengi (param(stroke)); öteki renkler sabit kalır.')),
          summary,
        ),
      ),
    ],
    footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, primary],
    stack: true,
  });
  cancel.addEventListener('click', () => dialog.close());
  primary.addEventListener('click', () => {
    const mapped = mapColors(raw.doc, symbol, second);
    dialog.close();
    const tail = lost.length ? `; ${lost.join(', ')}` : '';
    if (mode === 'new')
      files.confirmReplace(() => {
        host.open(mapped, null, name);
        files.reference.restore(null);
        host.status(`${done}: yeni çizim olarak açıldı${tail}.`, lost.length ? 'warn' : 'ok');
      });
    else {
      files.addShapes(mapped);
      host.status(`${done} ve çizime eklendi${tail}.`, lost.length ? 'warn' : 'ok');
    }
  });
  render();
  queueMicrotask(() => primary.focus());
}
