import type { ToolInfo } from '../../contracts/generated/sheet/ToolInfo';
import type { Command } from '../../core/commands';
import type { Disposable } from '../../core/disposable';
import { presetIcon, toolIcon } from '../../product/sheet/profile';
import type { AppContext } from '../context';
import type { SheetService } from './service';

/**
 * The commands of the mode's tools (design §11a), registered again whenever
 * the profile changes (another mode, another coordinate system): one per
 * tool that adds an item and one per ready look of it, the profile's keys
 * bound to them (keys.ts).
 */

const P = 'Pafta';

/**
 * The commands of the mode's tools (design §11a): `sheet.add.<tool>` takes a
 * tool that adds an item in hand (its first ready look), and
 * `sheet.add.<tool>.<preset>` one of its ready looks; Karelaj adds a grid to
 * the chosen map, Grupla groups. A tool the mode shows but cannot use says
 * the profile's reason; a hidden one is not registered.
 */
export function sheetToolCommands(ctx: AppContext, sheets: SheetService, tools: readonly ToolInfo[]): Disposable {
  const { state } = sheets;
  const watch = [state.open, state.engine, state.tool, state.selection];
  const out: Command[] = [];
  const shown = tools.filter((t) => t.state !== 'hidden');
  for (const t of shown) {
    const why = () => state.whyNot(true) ?? (t.state === 'disabled' ? (t.reason ?? 'Bu projede kullanılamaz.') : null);
    const base = { category: P, watch, isEnabled: () => why() === null, whyDisabled: why };
    if (t.item) {
      const take = (preset?: { id: string; label: string }) => () => state.tool.set({ kind: 'add', tool: t.id, preset: preset?.id, label: preset && t.presets.length > 1 ? preset.label : t.label, item: t.item! });
      const checked = (preset?: string) => () => state.tool.value.kind === 'add' && (state.tool.value as { tool: string }).tool === t.id && (preset === undefined || (state.tool.value as { preset?: string }).preset === preset);
      out.push({ ...base, id: `sheet.add.${t.id}`, title: t.label, icon: toolIcon(t), description: ADD_TIP[t.item] ?? undefined, run: take(t.presets[0]), isChecked: checked() });
      if (t.presets.length > 1)
        for (const p of t.presets) out.push({ ...base, id: `sheet.add.${t.id}.${p.id}`, title: p.label, icon: presetIcon(t, p.id), run: take(p), isChecked: checked(p.id) });
    }
  }
  return ctx.commands.registerAll(out);
}

/** The tooltip line of a kind's tool (what it adds; the mode names the tool). */
const ADD_TIP: Readonly<Record<string, string>> = {
  map: 'Projenin çiziminden ölçekli bir görünüm: ölçek standart listeden, merkez görünümden; karelaj ve yazı bandı çerçeveye dahil.',
  text: 'Metin kutusu; [% ifade %] parçalarıyla değişkenler (@pafta_adi, @olcek, @tarih …).',
  legend: 'Bağlı haritanın katmanlarından lejant.',
  scaleBar: 'Bağlı haritanın ölçek çubuğu (1-2-5 kuralı) ve “1/1000” sayısal ölçek.',
  northArrow: 'KentOS “K” kuzey oku; grid, coğrafi ya da manyetik kuzey.',
  table: 'Sabit satırlı ya da katmandan tablo.',
  coordinateList: 'Seçili nesnelerin ya da bir katmanın noktaları: No, Y, X (Z).',
  titleBlock: 'Hücreli antet: etiketler, değer ifadeleri, imza hücreleri.',
  border: 'Pafta çerçevesi: kenar boşluklarının içinde tek ya da çift çizgi.',
  picture: 'Logo ya da resim (PNG, JPEG, SVG); şablonla birlikte taşınır.',
  shape: 'Dikdörtgen, elips, üçgen ya da çokgen şekil.',
  line: 'Çizgi; uçlarında ok, nokta ya da çubuk.',
};

/** The key a tool's command is bound to in sheet mode: the profile's (M, T, L …). */
export const toolKeys = (tools: readonly ToolInfo[]): [string, string][] => tools.filter((t) => t.item && t.shortcut && t.state !== 'hidden').map((t) => [t.shortcut!, `sheet.add.${t.id}`]);
