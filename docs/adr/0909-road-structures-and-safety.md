# ADR 0909 — Sanat yapıları ve güvenlik: otokorkuluk, menfez, duvar ve metraj

Tarih: 2026-11-03. plan/road.md Y11 (ROAD-DRAIN-01, 02, 11; ROAD-JUNC-12). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Otokorkuluk gerekçesi ve gerekli boy:** AASHTO'nun *Roadside Design Guide*'ı (5. bölüm) bir engelin ya da yüksek
  dolgunun korunmasını dolgu yüksekliği ve şev eğimine göre, korkuluğun gerekli boyunu (length of need) yanal uzanım
  ve kaçış boyuyla verir; yaklaşma ucu trafiğin geldiği yandadır. Kitap ücretlidir; resmî metinden değer alınmaz.
  Eyalet kılavuzları yöntemi aynen anlatır, değerleri kendi tablolarındandır: Iowa DOT
  ([8B-6 Locating Barrier](https://iowadot.gov/design-manual/chapter-8-roadside-safety/8b-6-locating-barrier)),
  Montana DOT ([Length of Need](https://mdt.mt.gov/other/webdata/external/cadd/rsh/MDT_Designer_Session_6-Length_of_Need.PDF)),
  FDOT'nin Length of Need tasarım aracı ([Index 400](https://www.fdot.gov/docs/default-source/roadway/training/webinar16/2-1.pdf)).
  MwRSF kaçış boylarının fazla olabileceğini gösterdi ([TRP-03-57-96](https://mwrsf.unl.edu/researchhub/files/Report263/TRP-03-57-96.pdf)):
  değer kurumun seçimidir.
- **Türkiye:** KGM'nin *Karayolu Tasarım El Kitabı* ve otokorkuluk standardı (TS EN 1317) ücretli basılı
  yayınlardır; ikincil kaynaktan sınır değeri alınmaz (plan/road.md ROAD-DEC-01). KentOS kuralın biçimini verir,
  sayılarını kurum girer; kuralın kaynağı zorunludur.

## Karar

1. **Otokorkuluk (ROAD-JUNC-12).** Otokorkuluk yol ayarlarında `guardrails[]`'tır: `id`, `name`, `corridor`,
   `corridorRevision`, `revision`, `rule`, `step`, `offset`. Koridorun kesitleri her `step` metrede okunur
   (`road.stakeout.query`'nin şev başı ve şev topuğu noktaları); bir yanın dolgu yüksekliği şev başının şev topuğundan
   yüksekliği, şevi aradaki yatayın o yüksekliğe oranıdır (1:n); yarma, açık ya da çözülmemiş yan dolgu sayılmaz.
   Kural projenin tablosudur: `source` (standart ve tablosu, zorunlu), `warrants[]` (`height` en az dolgu ve isteğe
   bağlı `slope`: 1:`slope`'tan dik; herhangi bir satır tutarsa korkuluk gerekir), `approach` ve `leaving` gerekli
   bölümden önce ve sonra eklenen boylar, `gap` iki korkuluk arasında bırakılabilecek en az aralık. Sağda trafik km
   yönündedir: `approach` gerekli bölümün önüne, `leaving` arkasına; solda tersi. Korkuluklar koridorun km'lerine
   kırpılır; aradaki boşluğu `gap`'ten kısa olanlar birleşir (birleşen korkuluk en yüksek dolguyu ve en dik şevi
   taşır). Gerekli boyun eğik çizgi yöntemi (yanal uzanım ve kaçış boyu) ve engel noktaları sonraya; bugünkü kural
   dolgu ve şev tablosudur. Korkuluklar yazılmaz, her okumada hesaplanır; koridor değişince `guardrail_stale`,
   `rebind` koridorun bugünkü sürümünü alır. Komutlar `road.guardrails.edit` (tek adım “Otokorkuluk”),
   `road.guardrails.query` ve `road.guardrails.draw` (korkuluk şev başı artı `offset` dışa, uçları kesitler arasında
   doğrusal okunan şev başında, aksın yerleşimiyle; `labels` ile km ve boy; tek adım “Otokorkuluk çiz”).
   `.kcad` şema 161'in `guardrails`'i. Masaüstünde Koridor › Güvenlik › Otokorkuluk.

2. **Menfez (ROAD-DRAIN-01, 02, 11).** Menfez yol ayarlarında `culverts[]`'tır, bir koridorun km'sinde aksı kesen
   kalıcı bir varlıktır (kimliği, koridoru ve koridorun sürümü; koridor değişince `culvert_stale`): `station`,
   `skew` (aksın dikinden sapma, derece, sağa artı), kesit `shape` (`pipe`: `diameter`, `thickness`, `count`;
   `box`: `width`, `height`, `thickness` yan duvar, `slab` taban ve tavan, `cells`), akış `invert` (aksın altındaki
   akış kotu), `slope` (yüzde, akış yönünde düşen) ve `flow` (`right` ya da `left`: suyun aktığı yan), isteğe bağlı
   `extend` (her uçta topuğun ötesine boy), `cover` (en az örtü; verilirse denetlenir), `material` ve `roughness`
   (Manning n; tipli öznitelik, hesapta kullanılmaz). Boyutlar kurumun ya da projenin; KentOS öntanımlı ölçü vermez.
   Bentley'nin ConceptStation'ı menfezi yola yerleştirir, biçimi daire ya da kutu, et kalınlığı ve çapıyla; yarmada
   menfez yerleştirmez. KentOS da: menfezin km'sinde kesitin iki yanı dolgu olmalıdır (şev topuğu şev başından
   alçakta), yoksa ret. Boy iki şev topuğu arasının eğik uzunluğudur (yatay ötelemeler farkı / cos `skew`) artı iki
   `extend`; uçların akış kotları `invert` ± eğim × eğik uzaklık. Örtü kesitin şev başları arasındaki üst yüzeyin
   (kesitin noktaları arasında doğrusal) menfezin üst yüzünden (akış kotu + iç yükseklik + et ya da tavan) en küçük
   yüksekliğidir; `cover`'dan azsa `culvert_cover`. Metraj geometriktir: gövde boyu, boru boyu (boru sayısı × boy) ve
   gövde betonu (borunun halkası ya da kutunun dış kesitinden gözlerin çıkarılmasıyla) × boy. Pozlara eşleme §4'te.
   Komutlar `road.culverts.edit` (tek adım “Menfez”), `road.culverts.query`, `road.culverts.draw` (planda gövdenin
   iki dış kenarı, ekseni ve uçları; `labels` ile km, kesit ve boy).

## Aşamalar

Y11.1 (yapıldı, otokorkuluk): `road.guardrails.edit` 29, `.query` 20, `.draw` 21 ortak durum
(`road_guardrail_cases.py`; kesitler road_stakeout_cases.py'nin, yerler road_alignment_cases.py'nin 40 basamaklı
başvurusundan, kural ADR'den; KentOS kodu yok; kuralın sayıları durumlarındır, bir standardın değil). Çekirdek
`kentos_road::guardrail`, KCAD şema 161'in `guardrails`'i bağımsız okuyucuda (`broken/road-guardrail-unknown-corridor.kcad`).
Python, MCP, masaüstünde Koridor › Güvenlik › Otokorkuluk.

## Sonra

Gerekli boyun yanal uzanım ve kaçış boyuyla hesabı ve kurpta grafik çözümü, engel noktaları (bir katmanın nesneleri)
ve güvenli bölge genişliği, korkuluğun enkesit görünüşünde işareti, uç birimleri ve geçiş elemanları.
