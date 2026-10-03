import type { ToolGroup } from '../../contracts/generated/sheet/ToolGroup';
import type { ToolInfo } from '../../contracts/generated/sheet/ToolInfo';
import { toolIcon, type SheetProfile } from '../../product/sheet/profile';
import type { RibbonItem, RibbonPanel, RibbonSize, RibbonTab } from '../ribbon';

/**
 * The contextual Pafta tab (docs/sheet/design.md §11, §11a): shown while a
 * sheet is in front. Its own panels: Pafta (Yeni ▾, Şablondan, Sayfa
 * ayarları, Değişkenler, Modele dön), Görünüm (Sayfayı sığdır, Gerçek boyut)
 * and Çıktı (Ön denetim, Dışa aktar ▾, Şablon olarak kaydet). Between them
 * the mode's profile's groups, as the engine gives them: Araçlar (Seç, El),
 * Ekle (the mode's tools, each in its mode's name, a tool with several
 * ready looks as a drop-down of them), Harita (Karelaj, Atlas) and Düzen
 * (Hizala ▾, Dağıt ▾, Sıra ▾, Grupla, Grubu çöz). A tool the profile hides
 * is not there; one it shows off says why in its tooltip.
 */

const command = (id: string, size: RibbonSize = 'small'): RibbonItem => ({ kind: 'command', id, size });
const menu = (label: string, icon: string, items: readonly string[], size: RibbonSize = 'small'): RibbonItem => ({ kind: 'menu', menu: { label, icon, items }, size });

export const SHEET_TAB_ID = 'sheet';

/** The commands of the tools that add no item. */
const TOOL_COMMAND: Readonly<Record<string, string>> = { select: 'sheet.tool.select', pan: 'sheet.tool.hand', group: 'sheet.group', grid: 'sheet.grid', atlas: 'sheet.atlas' };

/** A tool as a ribbon item: its command, or a drop-down of its ready looks. */
function toolItem(t: ToolInfo): RibbonItem | null {
  if (t.state === 'hidden') return null;
  if (!t.item) return TOOL_COMMAND[t.id] ? command(TOOL_COMMAND[t.id], t.id === 'select' || t.id === 'pan' ? 'large' : 'small') : null;
  // The tools with a key of their own are the mode's main ones: drawn large.
  const size: RibbonSize = t.shortcut ? 'large' : 'small';
  if (t.presets.length > 1) return menu(t.label, toolIcon(t), t.presets.map((p) => `sheet.add.${t.id}.${p.id}`), size);
  return command(`sheet.add.${t.id}`, size);
}

function groupPanel(g: ToolGroup, extra: readonly RibbonItem[] = []): RibbonPanel | null {
  const items = [...g.tools.map(toolItem).filter((i): i is RibbonItem => !!i), ...extra];
  if (!items.length) return null;
  const first = g.tools.find((t) => t.state !== 'hidden');
  return { label: g.label, icon: first ? toolIcon(first) : 'sheetLayout', items };
}

const ARRANGE: readonly RibbonItem[] = [
  menu('Hizala', 'sheetAlignLeft', ['sheet.align.left', 'sheet.align.center', 'sheet.align.right', '-', 'sheet.align.top', 'sheet.align.middle', 'sheet.align.bottom', '-', 'sheet.alignTo.selection', 'sheet.alignTo.page', 'sheet.alignTo.margins']),
  menu('Dağıt', 'sheetDistributeH', ['sheet.distribute.hCenters', 'sheet.distribute.hGaps', '-', 'sheet.distribute.vCenters', 'sheet.distribute.vGaps', '-', 'sheet.matchSize.width', 'sheet.matchSize.height']),
  menu('Sıra', 'sheetFront', ['sheet.order.front', 'sheet.order.forward', 'sheet.order.backward', 'sheet.order.back']),
];

export function sheetRibbonTab(profile: SheetProfile): RibbonTab {
  const groups = profile.groups.map((g) => (g.id === 'arrange' ? groupPanel(g, [...ARRANGE, command('sheet.ungroup')]) : groupPanel(g)));
  // A profile with no arranging group still arranges (every mode's items are arranged alike).
  if (!profile.groups.some((g) => g.id === 'arrange')) groups.push({ label: 'Düzen', icon: 'sheetAlignLeft', items: [...ARRANGE, command('sheet.group'), command('sheet.ungroup')] });
  return {
    id: SHEET_TAB_ID,
    label: 'Pafta',
    contextual: 'sheet',
    panels: [
      {
        label: 'Pafta',
        icon: 'sheetLayout',
        items: [
          menu('Yeni', 'sheetNew', ['sheet.new', 'sheet.duplicate', '-', 'sheet.importKpafta'], 'large'),
          command('sheet.fromTemplate', 'large'),
          command('sheet.pageSetup'),
          command('sheet.variables'),
          command('sheet.model'),
        ],
      },
      ...groups.filter((p): p is RibbonPanel => !!p),
      { label: 'Görünüm', icon: 'sheetZoomPage', items: [command('sheet.zoomPage', 'large'), command('sheet.zoomReal', 'large'), command('sheet.zoomSelection')] },
      {
        label: 'Çıktı',
        icon: 'sheetPreflight',
        items: [command('sheet.preflight', 'large'), command('sheet.print', 'large'), menu('Dışa aktar', 'export', ['sheet.export.pdf', 'sheet.export.svg', 'sheet.export.png', '-', 'sheet.export.kpafta'], 'large'), command('sheet.saveTemplate', 'large')],
      },
    ],
  };
}

/** Every command the tab reaches (its buttons and its drop-downs), for the tests and the shortcut list. */
export function sheetTabCommands(tab: RibbonTab): string[] {
  return tab.panels.flatMap((p) => p.items.flatMap((i) => (i.kind === 'command' ? [i.id] : i.kind === 'menu' ? i.menu.items.filter((x): x is string => typeof x === 'string' && x !== '-') : [])));
}
