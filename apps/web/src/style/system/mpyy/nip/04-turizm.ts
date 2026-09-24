import { BLACK, WHITE, framed, pattern, rgb, sheet, solid } from '../dsl';
import { S, box, speckle, stack } from './common';

/**
 * NİP (EK-1ç) › Turizm alanları: turizm, günübirlik tesis ve golf turizm
 * alanları. Üçünün taraması aynı: 0,2 mm kalemle 4 mm çapında serbest
 * noktalama, 12 mm karolaj merkezlerinde, şaşırtmalı sıra (EK-1e s.57, 71, 76).
 */

/**
 * The speckled discs on a 12 mm grid, every other row shifted half a cell.
 * The legend puts the staggered rows half a column pitch apart (dots at
 * the cell centres and between them), so the rows are 6 mm apart.
 */
const discs = () => pattern(speckle(4), 12, 6, { stagger: true });

const discNote =
  '4 mm çapında serbest noktalama 12 mm karolaj merkezlerinde, şaşırtmalı sıra. Her benekli daire, daire içinde rastgele noktalardan oluşan tek bir çizim (benekli-daire). Sıralar arası 6 mm: EK-1ç örneğinde şaşırtmalı sıralar sütun aralığının yarısı kadar aralıklı.';

export const turizm = sheet('nip', ['Turizm alanları'], 'turizm', 40);

turizm.area('turizm-alani', 'Turizm alanı', [solid(rgb(255, 115, 0)), discs()], { ref: 'EK-1ç s.5; EK-1e s.71', note: discNote });
turizm.area('gunubirlik-tesis-alani', 'Günübirlik tesis alanı', [solid(rgb(255, 173, 0)), discs(), box('G', { weight: 900, heavy: true, max: 4.4 })], { ref: 'EK-1ç s.5; EK-1e s.76', note: discNote });
turizm.area(
  'golf-turizm-alani',
  'Golf turizm alanı',
  [solid(rgb(161, 194, 114)), discs(), stack(framed('GTA', { frame: 'circle', size: S, strokeWidth: 0.45, textSize: 2.6, font: 'sans', weight: 400, color: BLACK, background: WHITE }))],
  { ref: 'EK-1ç s.5; EK-1e s.57', note: `${discNote} EK-1e ölçeğe göre sembol değiştirir (GTB, GTA, GTT); NİP'te GTA.` },
);
