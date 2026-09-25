import type { Command } from '../core/commands';
import type { AppContext } from './context';

/**
 * Hesap menüsü: surveying computations (poligon, kutupsal alım, aplikasyon,
 * önden ve geriden kestirme). Their windows load on first use (CLAUDE.md
 * §20); the computations are the geometry core's (crates/shared/geometry-core
 * /src/survey), checked against an independent reference (§23.4).
 */
export function registerCalcCommands(ctx: AppContext): void {
  const failed = (e: unknown) => ctx.log.error(`Hesap penceresi yüklenemedi: ${e instanceof Error ? e.message : String(e)}. Bağlantınızı denetleyip komutu yeniden çalıştırın.`);
  const list: Command[] = [
    {
      id: 'calc.traverse',
      title: 'Poligon hesabı…',
      short: 'Poligon',
      category: 'Hesap',
      icon: 'surveyTraverse',
      description:
        'Bilinen bir noktadan kırılma açıları ve kenarlarla yeni poligon noktalarını hesaplar: bağlı, kapalı ya da açık. Açı kapanma hatası açılara eşit, koordinat kapanma hatası kenarlara uzunluklarıyla orantılı dağıtılır; noktalar çizime eklenir.',
      aliases: ['POLIGON', 'POLİGON', 'TRAVERSE'],
      run: () => void import('../ui/calc/TraverseDialog').then((m) => m.openTraverse(ctx)).catch(failed),
    },
    {
      id: 'calc.polar',
      title: 'Kutupsal alım…',
      short: 'Kutupsal alım',
      category: 'Hesap',
      icon: 'surveyPolar',
      description:
        'Bilinen bir istasyondan yatay açı okumaları ve uzunluklarla alınan noktaları hesaplar (takeometri). Eğik uzunluk başucu açısıyla yataya indirilir, istasyon kotu verilirse nokta kotları da hesaplanır.',
      aliases: ['KUTUPSAL', 'TAKEOMETRI', 'ALIM'],
      run: () => void import('../ui/calc/PolarDialog').then((m) => m.openPolar(ctx)).catch(failed),
    },
    {
      id: 'calc.stakeout',
      title: 'Aplikasyon…',
      short: 'Aplikasyon',
      category: 'Hesap',
      icon: 'surveyStakeout',
      description: 'Bir istasyondan bilinen noktaların aplikasyon değerlerini verir: semt, yatay uzunluk ve bakılan noktadan saat yönünde dönülecek açı. Rapor panoya kopyalanır.',
      aliases: ['APLIKASYON', 'APLİKASYON', 'SEMTMESAFE'],
      run: () => void import('../ui/calc/StakeoutDialog').then((m) => m.openStakeout(ctx)).catch(failed),
    },
    {
      id: 'calc.forward',
      title: 'Önden kestirme…',
      short: 'Önden kestirme',
      category: 'Hesap',
      icon: 'surveyForward',
      description: 'İki bilinen noktada yeni noktaya ölçülen açılardan yeni noktanın koordinatını hesaplar.',
      aliases: ['ONDEN', 'ÖNDEN', 'ONDENKESTIRME'],
      run: () => void import('../ui/calc/IntersectionDialog').then((m) => m.openIntersection(ctx, 'forward')).catch(failed),
    },
    {
      id: 'calc.resection',
      title: 'Geriden kestirme…',
      short: 'Geriden kestirme',
      category: 'Hesap',
      icon: 'surveyResection',
      description: 'Yeni noktada üç bilinen noktaya ölçülen iki açıdan durulan noktanın koordinatını hesaplar; tehlike dairesine yakınsa uyarır.',
      aliases: ['GERIDEN', 'GERİDEN', 'GERIDENKESTIRME'],
      run: () => void import('../ui/calc/IntersectionDialog').then((m) => m.openIntersection(ctx, 'resection')).catch(failed),
    },
  ];
  ctx.commands.registerAll(list);
}
