import type { EntityGeometry } from '../contracts/generated/EntityGeometry';
import { dimensionFault, type DimensionFault } from '../model/geom/dimension';
import { error, failed, type Stop } from './checks';

/**
 * A dimension's geometry as the commands take it (docs/adr/0147 §6): only a slope has elevations, an ordinate's axis
 * is 0 (its Y) or 90 (its X), and a new kind is one the core can draw (`dimensionFault`). Refused as
 * `invalid_dimension` with the place to mend; the desktop's `kentos_native_application::dimension` says the same.
 */
export function checkDimension(g: EntityGeometry, at: (field: string) => string): Stop | null {
  if (g.kind !== 'dimension') return null;
  const refuse = (message: string, field: string) => failed(error('invalid_dimension', message, at(field)));
  if (g.style !== 'slope' && (g.za != null || g.zb != null))
    return refuse('Kot yalnız eğim ölçüsünde olur. Kotları (za, zb) kaldırın ya da ölçünün biçimini eğim yapın.', g.za != null ? '.za' : '.zb');
  if (g.style === 'ordinate' && g.angle != null && g.angle !== 0 && g.angle !== 90)
    return refuse(`Koordinat ölçüsünün ekseni 0 (Y) ya da 90 (X) olmalı; ${g.angle} verildi. Y için 0, X için 90 verin.`, '.angle');
  const fault = dimensionFault(g);
  if (!fault) return null;
  const edge = g.style === 'slope' ? 'Eğim' : 'Semt';
  const said: Record<DimensionFault, [string, string]> = {
    ordinateTooShort: ['Koordinat ölçüsünün çizgisi noktadan eksene dik yönde yazı yüksekliğinin yarısından uzun olmalı. Çizginin ucunu (b) noktadan daha uzağa verin.', '.b'],
    arcNoCentre: ['Yay uzunluğu ölçüsünün merkezi (c) verilmeli. Ölçülen yayın merkezini verin.', '.c'],
    arcNoRadius: ['Yay uzunluğu ölçüsünde yayın başlangıcı (a) merkezde (c); yarıçap sıfır. Başlangıcı yayın üstünde verin.', '.a'],
    arcNoSweep: ['Yay uzunluğu ölçüsünde yayın iki ucu merkezden aynı doğrultuda; yayın açısı sıfır. Sonu (b) başka bir doğrultuda verin.', '.b'],
    arcInside: ['Yay uzunluğu ölçüsünün ölçü yayı merkeze ulaşıyor: içe ötelenme yarıçaptan küçük olmalı. Ötelenmeyi büyütün.', '.offset'],
    joggedNoCentre: ['Kırıklı yarıçap ölçüsünün gösterilen merkezi (c) verilmeli. Çizginin başlayacağı noktayı verin.', '.c'],
    joggedNoRadius: ['Kırıklı yarıçap ölçüsünde yaydaki nokta (b) merkezde (a); yarıçap sıfır. Noktayı yayın üstünde verin.', '.b'],
    joggedCentre: [
      'Kırıklı yarıçap ölçüsünde gösterilen merkez (c) yarıçap boyunca yaydaki noktadan (b) geride olmalı; yarıçap çizgisinden uzaklığı bu geriliği aşmamalı. Gösterilen merkezi yayın içinde, yarıçapa yakın verin.',
      '.c',
    ],
    edgeTooShort: [`${edge} ölçüsünün iki ucu aynı nokta; ölçülecek kenar yok. Kenarın öbür ucunu (b) verin.`, '.b'],
    slopeNoElevations: ['Eğim ölçüsünün iki ucunun da kotu verilmeli. Eksik kotu (za ya da zb) metre olarak verin.', g.za == null ? '.za' : '.zb'],
  };
  const [message, field] = said[fault];
  return refuse(message, field);
}
