import { BLACK, rgb, sheet, stroke } from '../dsl';

/**
 * MSP (EK-1e bölüm I, s.1) › Sınırlar: plan sınırı. Mekânsal strateji planı
 * gösterimleri yalnız EK-1e'nin ilk bölümünde; MSP satırları sembol ölçüsü
 * vermez, ölçüler çizimden (not düşülür).
 */

export const sinirlar = sheet('msp', ['Sınırlar'], 'sinirlar', 10);

sinirlar.line('plan-siniri', 'Plan sınırı', [{ ...stroke(rgb(178, 178, 178), 1, { cap: 'butt' }), blur: 1.2, shift: [0, -0.6] }, stroke(BLACK, 1)], {
  ref: 'EK-1e s.1',
  note: '1 mm siyah çizgi, 178/178/178 gölgeli. Gölge sayfada hep aşağıya düşer (çizginin yönünden bağımsız): 1 mm gri çizgi 0,6 mm aşağı kaydırılmış, kenarları 1,2 mm boyunca yumuşar; siyah çizginin alt kenarından 178/178/178 ile başlayıp 1,2 mm aşağıda kaybolur. Kayma ve yumuşama ölçüsü yazılı değil, EK-1e çiziminden (orada gölge çizilen çizgi kalınlığının yaklaşık 1,5 katı aşağıda kaybolur; yana kayma yok).',
});
