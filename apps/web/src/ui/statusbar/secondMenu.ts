import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import { secondChoices, secondTitle } from '../../app/secondCrs';
import type { MenuItem } from '../widgets/PopupMenu';

/** The coordinate system's right-click menu, in the status bar and the tab row: Koordinat sistemi… and İkinci sistem ▸. */
export function crsMenu(ctx: AppContext): MenuItem[] {
  return [commandItem(ctx, 'crs.set'), { label: 'İkinci sistem', icon: 'crsSecond', items: () => secondMenu(ctx) }];
}

/**
 * İkinci sistem (docs/adr/0167 §1): Yok and the registry's systems but the project's own, a submenu for each datum, and
 * how a geographic second system's values are written. The coordinate system cell's right-click menu holds it, and the
 * second system's cell opens it; the desktop's `second_menu` lists the same. Choosing is a project setting, an edit of
 * the drawing as Proje ayarları's, never a transformation.
 */
export function secondMenu(ctx: AppContext): MenuItem[] {
  const settings = ctx.doc.settings;
  const project = settings.crs.value;
  if (project.kind === 'local') return [{ label: 'Yerel projenin ikinci sistemi olmaz', disabled: true }];
  const current = settings.secondSrid.value;
  const choose = (srid: number | null) => {
    if (srid === current) return;
    settings.assign({ secondSrid: srid });
    ctx.log.success(srid === null ? 'İkinci koordinat sistemi kaldırıldı.' : `İkinci koordinat sistemi: ${secondTitle(srid)}. Çizim dönüştürülmedi.`);
  };
  const notation = ctx.prefs.geographic;
  return [
    { kind: 'header', label: 'İkinci koordinat sistemi' },
    { label: 'Yok', radio: true, checked: current === null, run: () => choose(null) },
    // A datum each, the one chosen beside its name: the list stays short, Coğrafi değerler in view.
    ...secondChoices(project).map(
      ({ datum, systems }): MenuItem => ({
        label: datum,
        hint: systems.find((c) => c.srid === current)?.name,
        items: systems.map((c): MenuItem => ({ label: c.name, hint: `EPSG:${c.srid}`, radio: true, checked: c.srid === current, run: () => choose(c.srid) })),
      }),
    ),
    { kind: 'separator' },
    { kind: 'header', label: 'Coğrafi değerler' },
    { label: 'Derece, dakika, saniye', hint: '40°45′12.3456″K', radio: true, checked: notation.value === 'dms', run: () => notation.set('dms') },
    { label: 'Ondalık derece', hint: '40.7534293°K', radio: true, checked: notation.value === 'dd', run: () => notation.set('dd') },
  ];
}
