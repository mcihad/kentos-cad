import type { Command } from '../core/commands';
import { startServiceInfo } from '../ui/bottom/serviceInfoRun';
import { basemapCommand, PRESET_GROUPS, PRESETS } from '../model/servicePresets';
import type { AppContext } from './context';
import { addBasemap, bottomBasemap, removeBasemap } from './services';

/**
 * Harita servisleri (docs/adr/0208 §14): a command for each ready basemap (`basemap.<id>`, Altlık ▾), Altlığı
 * kaldır, Harita servisi, Servisten veri al, Servis bilgisi and Bağlantılar. Their windows load on first use
 * (CLAUDE.md §20); requests, capabilities, tiles and features are the services core's (crates/shared/services).
 */
export function registerServiceCommands(ctx: AppContext): void {
  const groupName = new Map(PRESET_GROUPS.map((g) => [g.id, g.name]));
  const basemaps: Command[] = PRESETS.map((p) => ({
    id: basemapCommand(p.id),
    title: p.name,
    short: p.name,
    category: 'Altlık',
    icon: p.icon,
    description: `${groupName.get(p.group) ?? ''}: altlık olarak en alta konur; en alttaki hazır altlık varsa onun yerini alır. ${p.note}`.trim(),
    run: () => addBasemap(ctx, p),
  }));
  const list: Command[] = [
    ...basemaps,
    {
      id: 'basemap.remove',
      title: 'Altlığı kaldır',
      category: 'Altlık',
      icon: 'basemapRemove',
      description: 'Katman ağacının en altındaki hazır altlığı kaldırır (tek adım).',
      aliases: ['ALTLIKKALDIR', 'ALTLIĞIKALDIR'],
      run: () => removeBasemap(ctx),
      isEnabled: () => bottomBasemap(ctx) !== null,
      watch: [ctx.doc.layers.version],
    },
    {
      id: 'service.add',
      title: 'Harita servisi…',
      short: 'Harita servisi',
      category: 'Altlık',
      icon: 'serviceAdd',
      description:
        'XYZ / TMS, WMS, WMTS, OGC API Tiles, vektör karo, ArcGIS REST ya da Google servisini katman olarak ekler: adres, bağlantı, Bağlan ile yetenekler ve katmanlar, stil, biçim, sistem, saydamlık ve katlar. Altlık en alta, saydam servis altlıkların üstüne konur.',
      aliases: ['WMS', 'WMTS', 'XYZ', 'SERVIS', 'SERVİS', 'HARITASERVISI', 'HARİTASERVİSİ'],
      run: () => void import('../ui/services/ServiceDialog').then((m) => m.openServiceDialog(ctx)),
    },
    {
      id: 'service.feed',
      title: 'Servisten veri al…',
      short: 'Servisten veri al',
      category: 'Altlık',
      icon: 'serviceFeed',
      description:
        'WFS, OGC API Features, ArcGIS REST katmanı ya da GeoJSON adresinden nesneleri bir katmana alır: alan, süzgeç, en çok nesne, hedef katman; tek geri alma adımı. Katman kaynağını hatırlar, Yenile ile yeniden alınır.',
      aliases: ['WFS', 'SERVISTENVERI', 'SERVİSTENVERİ', 'GEOJSONADRES'],
      run: () => void import('../ui/services/FeedDialog').then((m) => m.openFeedDialog(ctx)),
    },
    {
      id: 'service.info',
      title: 'Servis bilgisi',
      category: 'Altlık',
      icon: 'serviceInfo',
      description: 'Tıklanan yerde görünen WMS katmanlarının GetFeatureInfo ve ArcGIS katmanlarının identify kayıtlarını alt panelde katman katman gösterir.',
      aliases: ['GETFEATUREINFO', 'IDENTIFY', 'SERVISBILGISI', 'SERVİSBİLGİSİ'],
      run: () => startServiceInfo(ctx),
    },
    {
      id: 'service.connections',
      title: 'Bağlantılar…',
      short: 'Bağlantılar',
      category: 'Altlık',
      icon: 'serviceConnections',
      description:
        'Projenin servis bağlantıları: kimlik doğrulama türü (parametre, başlık, kullanıcı adı ve parola, belirteç, ArcGIS belirteci, OAuth 2, Google anahtarı) ve gizli değerleri. Gizli değerler çizime yazılmaz, bu cihazda saklanır; Dene servise sorar.',
      aliases: ['BAGLANTILAR', 'BAĞLANTILAR', 'APIKEY', 'ANAHTAR'],
      run: () => void import('../ui/services/ConnectionsDialog').then((m) => m.openConnections(ctx)),
    },
  ];
  ctx.commands.registerAll(list);
}
