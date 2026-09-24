import { SNAP_KINDS } from '../../style/svg/snapping';
import { h } from '../dom';
import { icon } from '../icons';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import type { EditActions, PathOpId, Tab } from './svgActions';
import { svgIcon } from './svgIcons';
import type { ActionName } from './svgProps';
import type { CanvasOptions } from './svgView';

/**
 * The SVG editor's menus above the canvas: Yol (path operations), Nesne
 * (arranging, the Hizala, Dönüştür and Dizi tabs, stacking order), Seç
 * (selection helpers), and the snapping and ruler switches with the list
 * of snap kinds. Every entry says its key.
 */

export interface MenuHost {
  readonly edit: EditActions;
  readonly options: CanvasOptions;
  action(name: ActionName): void;
  setOption(patch: Partial<CanvasOptions>): void;
  showTab(tab: Tab): void;
  select(ids: string[]): void;
  /** Keys go back to the canvas after a menu choice (shortcuts keep working). */
  focusCanvas(): void;
}

/** Opens a menu under its button; after a choice the canvas has the keys again. */
const openAt = (host: MenuHost, b: HTMLElement, items: MenuItem[]) => {
  const r = b.getBoundingClientRect();
  const back = items.map((m) => (m.run ? { ...m, run: () => (m.run!(), host.focusCanvas()) } : m));
  PopupMenu.open(back, { x: r.left, y: r.bottom + 4 }, { owner: b, minWidth: 250 });
};

function menuButton(host: MenuHost, label: string, iconName: string, title: string, items: () => MenuItem[]): HTMLButtonElement {
  const b = h('button', { class: 'btn btn--small', type: 'button', title, 'aria-haspopup': 'menu' }, svgIcon(iconName, 14), label, icon('chevronDown', 12));
  b.addEventListener('click', () => openAt(host, b, items()));
  return b;
}

export function editMenus(host: MenuHost): HTMLElement[] {
  const e = host.edit;
  const p = (op: PathOpId, label: string, shortcut?: string, detail?: string): MenuItem => ({ label, shortcut, detail, run: () => e.path(op) });
  const path = menuButton(host, 'Yol', 'pathUnion', 'Yol işlemleri: birleşim, fark, kesişim, çizgiyi yola çevir, küçült/büyüt, sadeleştir', () => [
    p('union', 'Birleşim', 'Ctrl++', 'Seçilenlerin kapladığı her yer tek yol'),
    p('difference', 'Fark', 'Ctrl+-', 'Alttakinden üsttekiler çıkar'),
    p('intersection', 'Kesişim', 'Ctrl+*', 'Yalnızca hepsinin ortak yeri'),
    p('exclusion', 'Dışlama', 'Ctrl+^', 'Ortak yerler boşalır'),
    p('division', 'Bölme', 'Ctrl+/', 'Alttaki, üsttekilerin çizgileriyle parçalara bölünür'),
    p('cut', 'Yolu kes', 'Ctrl+Alt+/', 'Alttakinin çizgisi kesişimlerde açık parçalara ayrılır'),
    { kind: 'separator' },
    p('combine', 'Tek yolda topla', 'Ctrl+K'),
    p('breakApart', 'Parçalara ayır', 'Ctrl+Shift+K'),
    p('split', 'Parçalara ayır, delikler kalsın'),
    { kind: 'separator' },
    p('toPath', 'Nesneyi yola çevir', 'Ctrl+Shift+C', 'Dikdörtgen ve elips düğümlü yol olur'),
    p('strokeToPath', 'Çizgiyi yola çevir', 'Ctrl+Alt+C', 'Çizginin boyadığı alan dolgulu yol olur'),
    p('inset', `İçe küçült (${e.ui.offset})`, 'Ctrl+('),
    p('outset', `Dışa büyüt (${e.ui.offset})`, 'Ctrl+)'),
    p('simplify', `Sadeleştir (%${e.ui.simplify})`, 'Ctrl+L', 'Daha az düğüm; mesafe ve tolerans Özellikler’de'),
    { kind: 'separator' },
    p('reverse', 'Yönü çevir'),
    p('close', 'Yolu kapat'),
    p('open', 'Yolu aç'),
  ]);
  const a = (name: ActionName, label: string, shortcut?: string): MenuItem => ({ label, shortcut, run: () => host.action(name) });
  const obj = menuButton(host, 'Nesne', 'alignLeft', 'Hizala, dönüştür, dizi, sıra, grup', () => [
    { label: 'Hizala ve dağıt…', icon: 'align', shortcut: 'Ctrl+Shift+A', run: () => host.showTab('align') },
    { label: 'Dönüştür…', icon: 'move', shortcut: 'Ctrl+Shift+M', run: () => host.showTab('transform') },
    { label: 'Dizi ve aynalı kopya…', icon: 'array', run: () => host.showTab('array') },
    { kind: 'separator' },
    { label: 'En öne', shortcut: 'Home', run: () => e.restack('top') },
    { label: 'Bir öne', shortcut: 'PageUp', run: () => e.restack('raise') },
    { label: 'Bir arkaya', shortcut: 'PageDown', run: () => e.restack('lower') },
    { label: 'En arkaya', shortcut: 'End', run: () => e.restack('bottom') },
    { kind: 'separator' },
    a('flipH', 'Yatay çevir', 'H'),
    a('flipV', 'Dikey çevir', 'Shift+H'),
    a('rot90', '90° döndür'),
    { kind: 'separator' },
    a('group', 'Grupla', 'Ctrl+G'),
    a('ungroup', 'Grubu çöz', 'Ctrl+Shift+G'),
    a('duplicate', 'Çoğalt', 'Ctrl+D'),
    a('delete', 'Sil', 'Delete'),
  ]);
  const sel = menuButton(host, 'Seç', 'selectSame', 'Tümünü seç, ters çevir, benzerini seç', () => [
    { label: 'Tümünü seç', shortcut: 'Ctrl+A', run: () => e.selectAll() },
    { label: 'Seçimi ters çevir', shortcut: '!', run: () => e.invertSelection() },
    { label: 'Seçimi kaldır', shortcut: 'Esc', run: () => host.select([]) },
    { kind: 'separator' },
    { label: 'Aynı dolguyu seç', run: () => e.selectSame('fill') },
    { label: 'Aynı çizgiyi seç', run: () => e.selectSame('stroke') },
    { label: 'Aynı dolgu ve çizgiyi seç', run: () => e.selectSame('both') },
    { label: 'Aynı türü seç', run: () => e.selectSame('kind') },
  ]);
  return [path, obj, sel];
}

/** Snapping on/off with its kinds, and the rulers, at the bar's right; `update` shows the current options. */
export function viewSwitches(host: MenuHost): { els: HTMLElement[]; update: () => void } {
  const o = host.options;
  const snap = h('button', { class: 'btn btn--small svge__switch', type: 'button', 'aria-pressed': String(o.snapObjects), title: 'Şekillere, kılavuzlara ve tuvale kenetle (%)' }, icon('snap', 14), 'Kenet');
  snap.addEventListener('click', () => host.setOption({ snapObjects: !host.options.snapObjects }));
  const kinds = h('button', { class: 'ibtn svge__switchmore', type: 'button', title: 'Kenet türleri', 'aria-label': 'Kenet türleri', 'aria-haspopup': 'menu' }, icon('chevronDown', 12));
  kinds.addEventListener('click', () => {
    const on = new Set(host.options.snapKinds);
    openAt(host, kinds, [
      { kind: 'header', label: 'Kenet türleri' },
      ...SNAP_KINDS.map((k) => ({
        label: k.label,
        checked: on.has(k.kind),
        run: () => {
          on.has(k.kind) ? on.delete(k.kind) : on.add(k.kind);
          host.setOption({ snapKinds: SNAP_KINDS.map((x) => x.kind).filter((x) => on.has(x)) });
        },
      })),
      { kind: 'separator' },
      { label: 'Izgaraya kenetle', checked: host.options.snapGrid, run: () => host.setOption({ snapGrid: !host.options.snapGrid }) },
    ]);
  });
  const rulers = h('button', { class: 'btn btn--small svge__switch', type: 'button', 'aria-pressed': String(o.rulers), title: 'Cetveller: kılavuz çekmek için cetvelden sürükleyin' }, svgIcon('rulers', 14), 'Cetvel');
  rulers.addEventListener('click', () => host.setOption({ rulers: !host.options.rulers }));
  const update = () => {
    snap.setAttribute('aria-pressed', String(host.options.snapObjects));
    rulers.setAttribute('aria-pressed', String(host.options.rulers));
  };
  return { els: [h('span', { class: 'svge__switches' }, snap, kinds), rulers], update };
}
