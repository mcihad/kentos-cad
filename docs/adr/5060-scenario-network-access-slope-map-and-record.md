# 5060 — Senaryo ölçülerinde ağ üzerinden erişim, eğim haritası ve damganın çizimde saklanması

Tarih: 2026-11-11. plan/map-cad.md MAP-10, onuncu tur. Durum: taslak. Plan satırının “Sıradaki”si: ağ üzerinden erişim;
damganın çizimde saklanması; eğim sınıflarının haritada gösterimi.

İlgili:
- [ADR 1440](1440-design-alternative-impact.md): senaryo ölçüleri ve taban damgası (damga yalnız oturumda saklanıyordu);
- [ADR 2040](2040-impact-impervious-and-access.md), [ADR 2260](2260-scenario-access-on-the-drawing.md): düz uzaklıkla
  erişim ve çizimde gösterimi;
- [ADR 4260](4260-scenario-terrain-slope.md), [ADR 4660](4660-scenario-slope-classes-and-aspect.md): eğim, eğim
  sınıfları ve bakı (ölçü satırı olarak);
- [ADR 0209](0209-network-analysis.md) ve sonrası: projenin ağları (Ağlar), ağ grafiği ve en yakın tesis araması.

## Dayanak

- **QGIS 3.44**, [Network analysis › Service area (from layer)](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/networkanalysis.html)
  (11 Kasım 2026'da okundu; resimler `.run/research/map-cad/uygulama/MAP-10.10-resim/`):
  - erişim, bir ağın kenarlarında bir başlangıç noktasından uzaklık ya da süre sınırına dek gidilebilen yerlerdir;
  - “Maximum point distance from network” (3.38'de eklendi, öntanımlı yok): noktanın ağa uzaklığı bundan büyükse nokta
    “non-routable” sayılır; verilmezse nokta ne kadar uzak olursa olsun ağın en yakın yerine kenetlenir;
  - “Non-routable features” çıktısı yönlendirilemeyen nesneleri (ör. ağdan çok uzak) ayrıca verir;
  - “Topology tolerance” (öntanımlı 0): düğümleri bundan yakın çizgiler bağlı sayılır.
- **ArcGIS Pro**, [Service area analysis layer](https://doc.esri.com/en/arcgis-pro/latest/help/analysis/networks/service-area-analysis-layer.html)
  (10 Kasım 2026, araştırma notu `MAP-10.10.md`): seyahat kipi, kesme değerleri, yön (tesisten / tesise), ağ katmanı.
- **Autodesk Civil 3D**, yüzey analizi (Surface Properties › Analysis, Slopes; gösterge tablosu Dynamic / Static;
  araştırma notu `MAP-10.10.md`): eğim aralıkları renk şemasıyla (az eğimden çok eğime) boyanır, gösterge tablosu
  aralığın alanını verir; Dynamic tablo analizle güncellenir, Static tablo yerleştirildiği anın değerini tutar.
- **Mevzuat** (`.run/research/map-cad/mevzuat/MAP-10.10.md`): MPYY ve 1987 genelgesi imar etüdü için sabit eğim sınıfı
  vermez (sınırlar jeoloğun kararı); AFAD'ın etüt raporu biçiminin derece sınıfları yalnız o rapor içindir. Ağ erişimi
  ve önbellek için hüküm yok. Netcad'in wiki'si oturum açmadan okunamıyor (11 Kasım 2026, denendi, atlandı).
- **KentOS bugün**: erişim yalnız düz uzaklıktı (ADR 2040); ağ grafiği ve aramalar ortak çekirdekte vardı
  (`kentos_geometry_core::ops::network`), ama uygulama katmanından projenin ağı kurulamıyordu (ADR 2260 Sonra); eğim
  sınıfları ölçü satırıydı, çizimde gösterilmiyordu (ADR 4660 Sınırlar); taban damgası yalnız bu oturumda saklanıyordu.

## Karar

1. **Ağ üzerinden erişim** (`cad.scenarios.impact`'in `access.network` ve `access.connect`'i; sürüm aynı, ek alanlar):
   - ağ projenin Ağlar'daki ağıdır (kimliği ya da adı; her türü): bağlanma kuralı, toleransı ve yönü onun tanımıdır;
     KentOS yeni bir ağ kuralı koymaz. Ağ İşlemler araçlarının ağ kurucusuyla kurulur (`ToolHost::network_of`, yeni
     yöntem; `pipes_check` gibi), senaryonun yerine geçtiği ağ katmanı senaryo tarafında senaryonun katmanıdır;
   - her alanın ağırlık merkezi ağın en yakın yerine bağlanır (`Graph::locate`); bağlanma uzaklığı kullanıcının
     `connect`'inden büyükse alan **ağa bağlanamaz** (QGIS'in “Maximum point distance from network”ü gibi; KentOS değer
     vermez): erişmez sayılır, veri eksiği olarak söylenir (“Ağa bağlanamayan … alanı: mevcut durumda n, senaryoda m”);
     hedef alan bağlanamıyorsa ona erişilmez;
   - uzaklık = erişen alanın bağlanması + ağda en kısa yol (maliyet 0, uzunluk; ağın tanımının yönleri) + hedefin
     bağlanması. Bağlanmalar gizlice kenetlenmez, sayılır ve ayrıca verilir; eşit toplamda çizimde önce gelen hedef;
   - `reach`'e `baseRoutes`/`alternativeRoutes` (her bağlanan alanın en yakın hedefi, uzaklığı, iki bağlanması ve yolun
     noktaları: merkez, ağdaki yeri, ağın yolu, hedefin ağdaki yeri, hedefin merkezi) ve `baseOff`/`alternativeOff`;
   - retler: `invalid_input` (`access.connect`: ağla verilmemiş ya da 0'dan büyük değil; `access.network`: ağsız
     bağlanma uzaklığı), `unknown_network`, `invalid_network`, `tools_unavailable`.
2. **Eğim haritası** (`slope.map`): cevapta `slopeMap` (iki taraf): her ölçülen alanın yüzey üçgenlerine düşen
   parçaları, sınıfı (ADR 4660'ın sınıfları; sınırsızsa 0), bakısı, üçgenin eğimi, parçanın alanı ve halkası
   (komutların halkası: saat yönünün tersine, en alt köşeden, eşitlerde batıdaki). Parçalar sınıf ölçülerinin kendi
   parçalarıdır: bir sınıfın parçalarının toplamı o sınıfın ölçüsüdür (tablo ve harita aynı sayı). Alanın parçaları ilk
   köşelerine göre (güneyden, sonra batıdan) sıralanır.
3. **Eğimi çizimde göster** (Senaryo ölçüleri penceresi): pencere kapanır, senaryonun ölçülen alanlarının parçaları
   sınıfının rengiyle boyanır (en az eğim başarı renginden uyarı rengine, en dik tehlike rengine; tema jetonları) ve sol
   altta anahtar: her sınıfın adı ve senaryodaki alanı (pencerenin sayısı). Çizim değiştikçe yeniden; İşareti kaldır'a
   dek; çizime yazılmaz.
4. **Erişimi çizimde göster** ağ seçilince her alanın yolunu da çizer (vurgu renginde, haleli; uçlarda nokta).
5. **Damganın çizimde saklanması: `cad.scenarios.record` v1** (Ölçüleri çizime yaz, yeni yazan komut):
   - girdi: karşılaştırmanın girdisi (`impact`), tablonun katmanı (`layerId`), köşesi (`at`, yoksa karşılaştırılan
     nesnelerin sağında, tablo yüksekliğinin iki katı, üstleri hizasında) ve beklenen sürüm;
   - karşılaştırma bir tablo olarak `cad.entities.create`'in yazısıyla yazılır, tek geri alma adımı “Ölçüleri çizime
     yaz”: başlık (ölçü ve senaryonun adı, birim, mevcut durum, senaryo, fark), her ölçü bir satır (alanlar projenin
     alan basamağıyla, uzunluklar uzunluk basamağıyla, paylar bir basamak, sayılar tam; fark işaretli), son satır
     “Damga” (taban ve senaryo damgası);
   - damga tablonun öznitelikleridir (`SenaryoKimlik`, `TabanDamgasi`, `SenaryoDamgasi`): `.kcad` şeması değişmez;
   - senaryo katmanlarının da damgası vardır (`scenarioStamp`, tabanınki gibi); `cad.scenarios.impact` her
     karşılaştırmada senaryonun çizimdeki tablolarını `records`'ta verir, her biri tabanın ya da senaryonun o zamandan
     beri değişip değişmediğiyle. Eskiyen tablo kendiliğinden güncellenmez (Civil 3D'nin Static tablosu gibi); yeniden
     yazılır;
   - retler: karşılaştırmanınkiler (yolları `impact.` altında), `invalid_input` (`layerId`: katman yok, grup ya da
     karşılaştırılan katman — tablo o katmanın ölçülerini değiştirirdi; `at`: sonlu değil), `invalid_revision`,
     `revision_conflict`, `layer_locked`.
6. **Masaüstü** (Senaryo ölçüleri): **Erişim ağı / bağlanma (m)** satırı (projenin ağları, “düz uzaklık” önce;
   bağlanma metre, boş bırakılırsa söylenir); **Ölçüleri çizime yaz** tabloyu “Senaryo ölçüleri” katmanına yazar
   (yoksa aynı adımda kurulur; tek geri alma adımı), pencere açık kalır; özet her tablo için “güncel” ya da “eskidi:
   taban / senaryo / ikisi yazıldıktan sonra değişti; … yeniden yazın” der; **Eğimi çizimde göster**. Yeni kısayol yok.
7. **Kapsam işareti:** **Harita + CBS** (eğim yüzeyi ve ağı olan projede).

## Sınırlar

- Yalnız uzunlukla erişim; süre (hız) ve yürüme profilleri ağın tanımında varsa bile kullanılmaz (maliyet 0).
- Hedef alana merkezinden bağlanılır; parkın kapısı ya da alanın ağa değdiği yer aranmaz.
- Eğim haritası yalnız sınıfları boyar; bakının haritası, derece birimi ve özel bakı sınıfları yoktur.
- Mevcut durumun haritası çizilmez (senaryo onun yerinde); cevapta iki taraf da vardır.
- Tablo yeniden yazılınca eskisi silinmez; eski tablo “eskidi” diye kalır.
- Eğim sınıfı sınırlarının resmî bir değeri yoktur; renk ölçeği uygunluk sınıflaması değildir.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/scenarios_impact_cases.py`, KentOS kodu yok): ağ, uçlarından ve başka bir
  çizginin ucunun değdiği yerden kesilen çizgiler; yer en yakın nokta; Dijkstra; eğim parçaları kesin kesirli kırpma ve
  kanonik halka. 11 yeni durum (61): eğim haritası (sınıf ve bakıyla; konutta, sınıfsız), ağ boyunca erişim (100 m,
  adıyla 120 m, 25 m bağlanma: konut ağa bağlanamaz, 18 m: hedef de bağlanamaz), dört ret, çizimdeki iki tablonun
  okunması (biri güncel, birinin senaryosu değişmiş). `cad.scenarios.record`: 22 durum
  (`scripts/fixtures/scenarios_record_cases.py`): plan, verilen köşede ve köşesiz yazma, geri alma, geçirimsizlik ve
  erişim, eğim, altyapı, ağ ile; aynı senaryo; taban yolu silinince tablonun eskimesi; karşılaştırılan katman, olmayan
  katman, grup, karşılaştırmanın retleri, sürüm, kilitli katman. Rust (`every_command_case`, `capabilities`) geçer.
  Bozma denemesi: bir yolun uzaklığı ve bir hücre değiştirilince koşucu tam bu iki durumu yakalar.
- **Python** (`python/tests/test_scenario_network_record.py`): testin kendi elle yürüdüğü yollar (L biçimli ağ),
  her yolun noktalarının uzunluğu uzaklığa eşit, düz uzaklık yoldan uzun değil; eğim parçalarının halkası saat
  yönünün tersine, alanı shoelace'e eşit, sınıfların toplamı ölçüye eşit; tablo yazılır, güncel okunur, geri alınır,
  karşılaştırılan katman reddedilir.
- **MCP** (`crates/native/mcp/tests/scenario_network_record.rs`): ağ boyunca yollar durumunki; tablo yazılır, bir
  sonraki karşılaştırma onu güncel okur; karşılaştırılan katman `layerId`'de reddedilir.
- **Masaüstü** (`impact_network_record_tests`): pencerenin ağ erişimi durumunki, okunmayan bağlanma söylenir, yollar
  çizimde; eğim haritası durumun parçaları ve dört sınıflı anahtar, İşareti kaldır siler; tablo tek adımda (katmanıyla)
  yazılır ve geri alınır. Resimler `.run/gui-evidence/map-10-10/`.

## Sonra

- Süreyle (hız, yürüme profili) erişim; hedef alanın ağa değdiği yerden bağlanma; servis alanı çizimi.
- Bakının haritası; derece birimi; özel bakı sınıfları (araştırma notu `R12-MAP-10.md`).
- Eskiyen tabloyu yerinde yeniden yazma (Dynamic tablo); mevcut durumun haritası.
- Web (TODOS §0.2).
