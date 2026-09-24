import { along, circle, rgb, sheet, solid } from '../dsl';

/** MSP (EK-1e bölüm I, s.11) › Su, doğal peyzaj ve ekosistemler. */

export const su = sheet('msp', ['Su, doğal peyzaj ve ekosistemler'], 'su', 70);

const BLUE = rgb(0, 160, 240);
su.line('su-havzalari', 'Su havzaları', [along(circle(0.7, { fill: BLUE }), 1.7)], {
  ref: 'EK-1e s.11',
  note: 'Alan şeffaf; sınır 0/160/240 noktalı: 0,7 mm çaplı noktalar 1 mm aralıklı (noktalar arası 1 mm boşluk okundu, eksenden eksene 1,7 mm; 1 mm eksenden eksene de okunabilir).',
});
su.area('onemli-koruma-alanlari', 'Önemli koruma alanları', [solid(rgb(120, 195, 0))], { ref: 'EK-1e s.11', note: 'Sınır verilmemiş.' });
su.area('hassas-ekolojik-sistemler', 'Hassas ekolojik sistemler', [solid(rgb(180, 225, 30))], { ref: 'EK-1e s.11', note: 'Sınır verilmemiş.' });
