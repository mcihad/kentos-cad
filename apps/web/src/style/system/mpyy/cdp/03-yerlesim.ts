import { WHITE, framed, hatch, rgb, sheet, solid } from '../dsl';

/**
 * ÇDP (EK-1c) › Yerleşim alanları: kentsel meskûn ve gelişme alanları,
 * kırsal yerleşik alan, bağlık-bahçelik alan (EK-1e s.47-48).
 */

export const yerlesim = sheet('cdp', ['Yerleşim alanları'], 'yerlesim', 30);

yerlesim.area('kentsel-meskun-alan', 'Kentsel meskûn (yerleşik) alan', [solid(rgb(140, 84, 26)), hatch(90, 3, 0.2)], { ref: 'EK-1c s.1; EK-1e s.47', note: '0,2 mm, 3 mm aralıklı dikey tarama.' });
yerlesim.area('kentsel-gelisme-alani', 'Kentsel gelişme alanı', [solid(rgb(255, 250, 38)), hatch(0, 3, 0.2)], { ref: 'EK-1c s.1; EK-1e s.47', note: '0,2 mm, 3 mm aralıklı yatay tarama.' });
yerlesim.point('kirsal-yerlesik-alan', 'Kırsal yerleşik alan', [framed('K', { frame: 'circle', size: 6, strokeWidth: 0.3, textSize: 3, font: 'sans', weight: 700, background: WHITE })], {
  ref: 'EK-1c s.1; EK-1e s.48',
  note: 'Nokta: 6 mm çapında daire içinde "K". Alan rengi ve tarama yok.',
});
yerlesim.area('baglik-bahcelik-alan', 'Bağlık-bahçelik alan', [solid(rgb(199, 235, 0)), hatch(0, 5, 0.2)], { ref: 'EK-1c s.1; EK-1e s.48', note: '0,2 mm, 5 mm aralıklı yatay tarama.' });
