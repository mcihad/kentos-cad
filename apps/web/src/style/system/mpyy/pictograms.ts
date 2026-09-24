import type { LibraryAsset } from '../../../model/style';
import { DRAWN } from './pictogramDrawings';

/**
 * Pictograms of the MPYY legends, drawn for KentOS as SVG assets
 * (mpyy.svg.<name>). Conventions (docs/STYLE.md §7):
 * - viewBox 0 0 100 100, the drawing centred with about 6 units margin;
 * - black parts use `currentColor`, so the symbol's colour (ink: black on
 *   paper, white on a dark screen) applies; the legend's white insides are
 *   holes (fill-rule="evenodd"), not white paint;
 * - pictograms that are coloured in the legend (lifebuoy, EV charger,
 *   green recycling, border flag, MSP tiles) keep their colours as hex;
 * - no text unless the pictogram itself carries letters (the lifebuoy's
 *   "A-K-L"); captions under frames are text markers of the symbol.
 *
 * Symbols refer to them with `pic('balik')`; a name missing here is a type
 * error. A name still drawn as PLACEHOLDER shows a question mark.
 */

export const PICTOGRAM_NAMES = [
  // Kentsel çalışma, askeri, özel kanunlar
  'serbest-bolge-bayragi', // outline flag on a pole, like a "P" (serbest bölge)
  'tufekler', // two crossed rifles, barrels up (askeri alan, askeri yasak bölge)
  'sinir-kapisi', // red pennant flag on a pole standing on a red bar (colour)
  'akaryakit', // fuel pump: pump body with window, hose and nozzle on the right
  'balik', // fish silhouette facing right, eye as a hole
  'dinamit', // tilted dynamite stick with a burning spark at the fuse
  'resmi-kurum', // circle, right half filled (resmi kurum)
  'disli', // hollow cog wheel, 12 square teeth, hole 5/7 of the outer diameter (TGB, OSB, endüstri, serbest bölge sınırları)
  'disli-dolu', // solid cog wheel, 12 square teeth (serbest bölge sınırı)
  // Turizm, tarım, doğa
  'cadir', // tent: two poles crossing at the apex, small filled door triangle
  'kayakci', // downhill skier under a roof-like chevron
  'plaj-semsiyesi', // beach umbrella: striped dome canopy, diagonal pole
  'fiskiye', // fountain: vertical jet with three drooping arcs above a basin ellipse (jeotermal)
  'basak', // wheat ear: curved stalk, feathery ear (tarımsal nitelikli, organik tarım)
  'sera', // greenhouse: heavy arch, two posts, heavy base line
  'radyasyon', // radiation trefoil
  'manzara', // view cone: viewpoint tick, two diverging rays, closing arc (kentsel görüntü öğeleri)
  'uc-konifer', // three filled conifers with trunks, middle smaller (milli park)
  'iki-katli-konifer', // two double-tiered filled conifers (tabiat parkı)
  'uc-kucuk-konifer', // three small filled two-tier conifers in a row (tabiatı koruma alanı)
  'geyik', // leaping deer silhouette (yaban hayatı)
  'kaplumbaga', // sea turtle, top view, line drawing
  'fok', // monk seal lying, head raised, line drawing
  'diken', // filled spiky tuft: pointed leaves fanning up from a flat base (doğal karakteri korunacak alan tarama)
  'ot', // five thin blades fanning up from one point (ekolojik öneme sahip alan tarama)
  'dalga', // one flat sine period ∼ across the full width, stroke 6 (a line marker: sulak alan sınırları)
  'hayvan-barinagi', // dog house with a paw print above the doorway
  'dort-konifer', // four outline conifers in two rows (ÇDP ağaçlandırılacak alan)
  'kent-ormani', // one conifer: two stacked open triangles on a thick trunk
  'iki-konifer', // two conifers, stacked open triangles on thin trunks (ağaçlandırılacak alan)
  'rekreasyon', // three filled trees: tall poplar, small round, large round
  'arboretum', // three outline trees: oval cypress, small round, large round
  'piknik-masasi', // picnic table with A-frame legs and bench (mesire)
  'hayvanat-bahcesi', // small picnic table next to a filled two-tier conifer
  'at', // standing horse silhouette facing right (hipodrom)
  'tahterevalli', // seesaw: tilted plank with round seats on a filled triangle (çocuk bahçesi)
  'fuar', // hollow circle with four arrows from the corners pointing at it
  // Sosyal altyapı
  'kitap', // open book seen from the front (kültürel tesis)
  'yurt', // roof chevron over an open book
  'bayrak-bos', // flag: pole with an outline triangular pennant (açık spor)
  'bayrak-dolu', // flag: pole with a filled triangular pennant (kapalı spor)
  'cami', // filled dome on a base line, finial and crescent
  'mescit', // outline dome on a base line, finial and crescent
  'hac', // filled Latin cross (kilise)
  'hac-bos', // outline Latin cross (şapel)
  'davut-yildizi', // hexagram of two interlaced outline triangles (sinagog)
  'can-simidi', // lifebuoy: green ring with four white segments, red centre, "A-K-L" (colour)
  // Ulaşım
  'terminal', // hollow circle with three spokes 120° apart (otogar)
  'gar', // locomotive front on tracks beside a station building and a standing figure
  'ara-istasyon', // locomotive front beside a person on a platform
  'tren-on', // train front view on rails, sleepers in perspective (raylı toplu taşıma, NİP)
  'metro-on', // metro/tram front view on converging sleepers (raylı toplu taşıma, UİP)
  'teleferik', // T-shaped hanger above a small square cabin with an X (havai hat)
  'havaray', // monorail car front: rounded body, window, two lights, hanger on top
  'ucak', // airplane, top view, nose up
  'tir', // box truck side view facing right, wheels as holes
  'motosiklet', // motorcycle silhouette
  'sarj-istasyonu', // EV charger: green column, white screen and bolt, cable (colour)
  'gemi', // ship seen from the bow with superstructure (tersane)
  'gemi-sokum', // ship silhouette, three-quarter bow view (gemi söküm)
  'capraz-capalar', // two crossed anchors (liman and its kinds)
  'capa-halat', // anchor with a rope round the shank (iskele)
  'capa', // simple anchor: ring, shank, curved arms with arrow tips (rıhtım, barınak)
  'yelkenli-bos', // sailboat with outline sails and hull (balıkçı barınağı)
  'yelkenli-dolu', // sailboat with filled sails and outline hull (tekne imal)
  'dolfen', // platform: horizontal line with a box on it, lower half filled
  // Enerji, altyapı
  'simsek', // lightning bolt, filled
  'trafo', // zig-zag lightning line ending in a filled arrowhead (electric hazard)
  'anten', // mast with three concentric signal arcs above
  'vana', // valve on a double pipe with a T handle (içme suyu tesisleri)
  'atiksu', // three circles in a triangle joined by curved arrows (atıksu tesisleri)
  'geri-donusum', // filled recycling symbol, three chasing arrows
  'geri-donusum-bos', // outline recycling symbol
  'geri-donusum-yesil', // green recycling symbol with a thin black outline (colour)
  'biyolojik-tehlike', // biohazard: three crescents, inner ring, central hole (tehlikeli atık tesisleri)
  'kaptaj', // three wavy lines above an open trapezoid basin (su kaynakları toplama yeri)
  'benekli-daire', // disc of free stippling: small dots at random inside a circle, no outline (turizm, günübirlik, golf discs)
  'benek', // speckle blob traced from EK-1d's turizm hatch: a 16 × 16 bitmap disc of about half cover (6 mm serbest noktalama)
  // MSP (EK-1e I): white pictograms on rounded tiles are drawn with the tile
  'msp-liman', // black rounded tile with a white anchor
  'msp-havalimani', // black rounded tile with a white airplane
  'msp-lojistik', // red rounded tile with a white bold letter L (EK-1e s.5 lojistik merkezler)
] as const;

export type PictogramName = (typeof PICTOGRAM_NAMES)[number];

/** Asset id of a pictogram. */
export const pic = (name: PictogramName): string => `mpyy.svg.${name}`;

/** Drawn until the real pictogram is in: a question mark in a circle. */
const PLACEHOLDER =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100"><circle cx="50" cy="50" r="40" fill="none" stroke="currentColor" stroke-width="6"/><path d="M38 38a12 12 0 1 1 18 10c-4 3-6 5-6 10" fill="none" stroke="currentColor" stroke-width="7"/><circle cx="50" cy="72" r="4.5" fill="currentColor"/></svg>';

export const PICTOGRAMS: readonly LibraryAsset[] = PICTOGRAM_NAMES.map((name) => ({
  kind: 'asset',
  id: pic(name),
  name,
  path: ['MPYY', 'Piktogramlar'],
  format: 'svg',
  data: DRAWN[name] ?? PLACEHOLDER,
  width: 100,
  height: 100,
}));
