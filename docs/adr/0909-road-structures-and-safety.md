# ADR 0909 — Sanat yapıları ve güvenlik: otokorkuluk, menfez, duvar ve metraj

Tarih: 2026-11-03. plan/road.md Y11 (ROAD-DRAIN-01, 02, 11; ROAD-JUNC-12). Durum: kabul edildi; aşamalar aşağıda.

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
- **İstinat duvarı:** Autodesk Civil 3D'nin RetainWallTapered alt montajı
  ([Subassembly Reference](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Subassembly-Ref/files/GUID-AB42AE09-E74B-4348-B5FA-7209AB33EB34.htm))
  duvar yüksekliğini temelin üstünden zeminin ya da bağlantı noktasının yüksek olanına ölçer; temel örtüsü (temelin
  üstünden bitmiş zemine), temel eni ve gövde eğimi parametredir. KentOS aynı ölçme biçimini alır, ölçüleri vermez:
  gövde, temel ve örtü projenindir; temel enini yüksekliğe bağlayan kural (Civil 3D'nin örneği) yoktur.
- **Poz metrajı:** KGM her yıl yol, sanat yapısı ve asfalt işleri için poz numaralı (örneğin “KGM/6220-A” biçiminde)
  birim fiyat listesi yayımlar; liste resmî yayındır, değerleri ikincil kaynaklardan (fiyat siteleri, yazılım
  satıcıları) alınmaz. KentOS listeyi taşımaz: defteri kurum yazar, kaynağı ve sürümü zorunludur.

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

3. **İstinat duvarı (ROAD-DRAIN-11).** İstinat duvarı yol ayarlarında `walls[]`'tır: bir koridorun bir yanında
   (`side`) `from` ile `to` km'leri arasında yolun şev başında şevin yerine duran konsol duvar; kimliği, koridoru ve
   koridorun sürümü (koridor değişince `retaining_wall_stale`), `step` (her kaç metrede okunduğu; aralığın uçları da),
   kesit `shape` (`top` gövdenin üst kalınlığı, `batter` metrede genişlemesi, `footing` temelin eni, `base`
   kalınlığı, `toe` yüzün önüne taşan temel, `cover` temelin üstündeki zemin, `above` korkuluk payı), isteğe bağlı
   `highest` (istenen en yüksek gövde; aşılırsa `retaining_wall_high`) ve `material`. Her km'de yanın şev başı (yol
   kenarı) ile kesit çizgisindeki zemin, şev başının ötelemesinde (kesit çizgisinin zemin noktaları arasında
   doğrusal) okunur; duvarın üstü ikisinin yükseği artı `above`, temelin üstü alçağı eksi `cover`, gövde yüksekliği
   aradaki; yol kenarı zeminden yüksekse duvar dolgu tutar, yüz yoldan dışa bakar, değilse yarmayı tutar, yüz yola
   bakar. Kesitin beton alanı gövdenin yamuğu (üstte `top`, altta `top + batter × H`) artı temelin dikdörtgenidir.
   Boyunca: yüzün plan boyu, gövde yüzü alanı (her aralığın boyu × yüksekliklerin ortalaması), beton (boy × alanların
   ortalaması); en yüksek gövde ve km'si. Kesitsiz km, zeminsiz yüz ret. Komutlar `road.walls.edit` (tek adım
   “İstinat duvarı”), `road.walls.query`, `road.walls.draw` (planda km'lerdeki yüz yerlerinden açık çizgi ve temelin
   ön ve arka kenarlarıyla kapalı dış çizgi; `labels` ile km aralığı ve en yüksek gövde). Kodlar `cad.walls`'ınkilerle
   çakışmasın diye `invalid_retaining_wall`, `unknown_retaining_wall`.

4. **Poz metrajı (ROAD-DRAIN-11).** Yol ayarlarının `pay`'i projenin tek poz defteridir: `source` (listenin adı ve
   yılı, zorunlu), `version`, `revision`, isteğe bağlı `currency`, `items[]` (kurumun yazdığı `code`, `name`, `unit`
   ve isteğe bağlı `price` birim fiyat) ve `rules[]` (`measure`: KentOS'un ölçülerinden biri, otokorkuluk boyu,
   menfez boyu, borusu ve betonu, duvar boyu, yüzü ve betonu, kazı, dolgu ve çarpanlı hâlleri; `item`; `factor`;
   isteğe bağlı `material`: yalnız o malzemedeki menfez ve duvarlar). Her kural ölçüsünü projenin o türdeki her
   kaynağından okur, çarpanla çarpar; poz satırı kaynak satırlarının toplamı, fiyatı varsa tutarı fiyat × miktar;
   toplam fiyatlı pozların tutarlarıdır, fiyatı olmayıp metrajı olan pozlar sayılır. Metraj yazılmaz, her okumada
   kaynakların bugünkü okumasından gelir: kaynağın koridoru değişmişse satırı eskimiş işaretlenir ve
   `pay_source_stale`, okunamayan kaynak metraj vermez ve `pay_source_unread`. Bu, planın “menfezin değişmesi bağlı
   metrajı eskitir” koşuludur; onaylı bir metrajın dondurulması (kübajın onayı gibi) sonraya. Komutlar `road.pay.edit`
   (tek adım “Poz defteri”) ve `road.pay.query`. Masaüstünde Koridor › Metraj › Poz metrajı.

## Aşamalar

Y11.1 (yapıldı, otokorkuluk): `road.guardrails.edit` 29, `.query` 20, `.draw` 21 ortak durum
(`road_guardrail_cases.py`; kesitler road_stakeout_cases.py'nin, yerler road_alignment_cases.py'nin 40 basamaklı
başvurusundan, kural ADR'den; KentOS kodu yok; kuralın sayıları durumlarındır, bir standardın değil). Çekirdek
`kentos_road::guardrail`, KCAD şema 161'in `guardrails`'i bağımsız okuyucuda (`broken/road-guardrail-unknown-corridor.kcad`).
Python, MCP, masaüstünde Koridor › Yapılar › Otokorkuluk.

Y11.2 (yapıldı, menfez): `road.culverts.edit` 29, `.query` 20, `.draw` 21 ortak durum (`road_culvert_cases.py`; kesit
road_stakeout_cases.py'nin, yerler road_alignment_cases.py'nin başvurusundan, gövde, örtü ve metraj §2'den; KentOS kodu
yok; boru ve kutu ölçüleri durumlarındır). Çekirdek `kentos_road::culvert`, KCAD şema 161'in `culverts`'i bağımsız
okuyucuda (`broken/road-culvert-right-angle.kcad`). Python, MCP (`tests/road_structures.rs`: durumların çizimi dosyaya
yazılıp açılır; örnek `road-work.kcad`'in koridoru şev buluşturmadığından menfezleri okunmaz, `invalid_culvert`).
Masaüstünde Koridor › Yapılar › Menfez: ölçüler yazıldıkça gövde boyuna kesitte (yükseklikler en çok beş kat), uçların
ötelemesi, yeri ve akış kotu; Kaydet, Sil, Planda çiz. Menfezin bağlı kesite ve metraja eskitme olarak işlenmesi ve
palye hendeği §4 ve sonraya.

Y11.3 (yapıldı, istinat duvarı): `road.walls.edit` 30, `.query` 20, `.draw` 21 ortak durum (`road_wall_cases.py`; kesit
ve kesit çizgisindeki zemin road_section_command_cases.py'nin, yerler road_alignment_cases.py'nin başvurusundan, duvar
§3'ten; KentOS kodu yok; ölçüler durumlarındır). Çekirdek `kentos_road::wall`, KCAD şema 161'in `walls`'ı bağımsız
okuyucuda (`broken/road-wall-toe-past-footing.kcad`). Python, MCP (`tests/road_structures.rs`), masaüstünde Koridor ›
Yapılar › İstinat: ölçüler yazıldıkça boyuna görünüş (yol kenarı, zemin, duvarın üstü ve temeli), km'lerin tablosu
(dolgu ya da yarma, kotlar, gövde, alan), boy, gövde yüzü, beton; Kaydet, Sil, Planda çiz. Duvarın kesite işlenmesi
(şevin yerine), iksa ve gabion türleri sonraya.

Y11.4 (yapıldı, poz metrajı): `road.pay.edit` 26, `.query` 20 ortak durum (`road_pay_cases.py`; ölçüler yapıların ve
kübajın kendi başvurularından, kurallar §4'ten; KentOS kodu yok; pozlar, numaralar ve fiyatlar durumlarındır). KCAD şema
161'in `pay`'i bağımsız okuyucuda (`broken/road-pay-rule-unknown-item.kcad`). Python, MCP (`tests/road_structures.rs`),
masaüstünde Koridor › Metraj › Poz metrajı: kaynak, sürüm, para birimi; düzenlenen poz ve kural tabloları; yazıldıkça
metraj, kaynakları (eskiyen işaretli), tutar ve toplam; Kaydet, Sil.

## Sonra

Gerekli boyun yanal uzanım ve kaçış boyuyla hesabı ve kurpta grafik çözümü, engel noktaları (bir katmanın nesneleri)
ve güvenli bölge genişliği, korkuluğun enkesit görünüşünde işareti, uç birimleri ve geçiş elemanları.
