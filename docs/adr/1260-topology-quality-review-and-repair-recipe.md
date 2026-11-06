# ADR 1260: Topolojik üretimin kalite süreci: bulgunun yaşam döngüsü, kalite profili ve onarım tarifi

- **Durum:** kabul edildi (2026-11-06). `plan/map-cad.md` MAP-07'nin bir dilimi; madde açık kalır (§7).
- **Bağlam belgesi:** ADR 0202 (13 kural, bulgular, düzeltmeler), ADR 0524 (kalıcı anahtar, artımlı denetim), ADR 0589
  (`cad.topology.check`, istisnanın geometri bağı, düzeltmenin önizlemesi), ADR 0756 (iş olarak denetim ve durdurma),
  ADR 1012 (`cad.drawing.repair`'in tarifi), ADR 0160 (topolojik düzenleme), ADR 0047 (kilitli katman).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **Şema:** `.kcad` 190 (dalda geçici numara; birleşmede yeniden numaralanır).

## 1. Kaynaklar ve KentOS kararı

Erişim: 6 Kasım 2026.

| Kaynak | Okunan | KentOS kararı |
|---|---|---|
| [GRASS v.clean belgesi](https://grass.osgeo.org/grass-stable/manuals/v.clean.html) | Birden çok araç sırayla çalışır, her araca bir eşik; snap kendinden eşik uzaklığındaki köşeye taşır; büyük eşik alan topolojisini onarılamaz bozabilir; hatalı geometri isteğe bağlı hata katmanına yazılır. | Onarım tarifi adım adım, sırası ve parametresi açık; tarifte her adım bulgusunu anar; kör “hepsini onar” yok. |
| [GRASS v.clean kaynak kodu (main.c)](https://raw.githubusercontent.com/OSGeo/grass/main/vector/v.clean/main.c) | Araçlar verilen sırayla döner; her araçtan önce topoloji (kısmen) yeniden kurulur; eşik araca sırayla eşlenir; araç başına sayım yazdırılmaz. | Her adımdan önce kurallar önceki adımların bıraktığı çizimde yeniden çalışır; GRASS'tan farklı olarak her adımın etkisi (nesne, taşınan köşe, alan) cevapta raporlanır. |
| [ArcGIS Pro: spatial reference özellikleri](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/the-properties-of-a-spatial-reference.html) | XY çözünürlüğü koordinatların saklandığı en küçük ayrımdır (varsayılan 0,0001 m); XY toleransı koordinatların aynı sayıldığı uzaklıktır (varsayılan 0,001 m, çözünürlüğün 10 katı); en küçük izinli tolerans çözünürlüğün iki katıdır. | Üç uzunluk ayrı: CAD yakınlık toleransı (var olan `tolerance`), depolama hassasiyeti (`precision`), kurumun kabul eşiği (`acceptance`). Tolerans hassasiyetin en az iki katıdır (ArcGIS'in kuralı); KentOS varsayılan değer koymaz: profil kurumun yazdığıdır. |
| [ArcGIS Pro SDK: MarkAsException](https://doc.esri.com/en/arcgis-pro/latest/sdk/api-reference/topic30040.html) ve [ProSnippets Topology](https://github.com/esri/arcgis-pro-sdk/wiki/ProSnippets-Topology) | Doğrulama hatası kabul edilebilirse istisnaya yükseltilir, kaldırılınca yeniden hatadır; düzenleme kirli alan üretir, doğrulama onu temizler. | Hata, istisna ve değerlendirilmemiş bulgu ayrı durumlar; geometri ya da kural değişince işaret düşer (kirli alanın karşılığı ADR 0524'ün artımlı denetimi). |
| [QGIS Topology Checker](https://docs.qgis.org/latest/en/docs/user_manual/plugins/core_plugins/plugins_topology_checker.html) | Kurallar katman başına; “Validate All” ve “Validate Extent”; istisna işaretleme belgede yok. | KentOS'un 13 kuralı ve tam/artımlı denetimi korunur; istisna modeli ArcGIS'e yakın. |
| Netcad Topoloji › Düzelt ([217385486](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=217385486), [217385453](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=217385453), [217385487](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=217385487)), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md) | Toleranslı düzeltme araçları. Bu oturumda Netcad'in bulgu istisnası ya da kalite profili için resmî bir sayfası bulunamadı. | Netcad'den değer ya da davranış alınmadı. |

Türk mevzuatının kabul eşikleri (BÖHHBÜY vb.) bu ADR'de **değer olarak yazılmadı**: profil kurumun değerini taşır,
KentOS uydurmaz.

## 2. Bulgunun yaşam döngüsü (`state`)

`cad.topology.check`'in her bulgusu `state` taşır:

- `open` — değerlendirilmedi: hiçbir işaret onu anmıyor;
- `error` — hata diye onaylandı, düzeltilecek (`cad.topology.exceptions` `error`);
- `exception` — bilerek bırakıldı (`mark`);
- `lapsed` — bir işaret onu anıyordu ama nesnelerinin geometrisi ya da kuralı değişti: yeniden değerlendirilmeli.

`reason` işaretin gerekçesidir. Sayılar: `open` (istisna olmayanlar; hata ve düşenler dahil), `exceptions`, `errors`,
`lapsed`. `exception` ve `lapsed` alanları geriye uyum için kalır.

İşaret (proje ayarlarının `topology.exceptions` listesi, aynı kayıt türü) şunları taşır:

- **nesne kimliği**: nesnelerin kalıcı kimlikleri bulgunun sırasıyla (var);
- **geometri özeti** `digest` (ADR 0589 §3, var);
- **kural sürümü** `ruleDigest` (§3, yeni);
- **gerekçe** `reason` (yeni; boş olmayan, en çok 500 karakter);
- **karar** `decision` (yeni; yokluğu `exception`, ya da `error`);
- **kapsam** `scope` (yeni; yokluğu `finding`: yerindeki bulgu, toleransla; `objects`: aynı nesneler arasında kuralın
  her bulgusu, yeri ne olursa olsun).

`cad.topology.exceptions` v1'e geriye uyumlu eklendi: `operation` `error`; `reason`; `scope` (yalnız `mark`). Aynı karar,
gerekçe ve kapsamla tutan işaret yeniden yazılmaz; başka karar ya da düşmüş işaret bugünkü geometri ve kuralla
değiştirilir. Proje **kalite profili** taşıyorsa gerekçesiz işaret `reason_required` ile reddedilir; boş ya da uzun
gerekçe, `unmark`'ta gerekçe, `mark` dışında kapsam `invalid_input`'tur. Kural sürümü değişince düşen işaretler için
`exception_lapsed` uyarısı “işaretin kuralı … değişti” der.

## 3. Kural sürümü

`rule_digest(rule)`: SHA-256(“kentos.topology-rule/1”, tür, katman, öbür katman, yürürlükteki değerin IEEE 754 bitleri
16 onaltılık hane ya da `-`), her parça sıfır baytla kapanır; ilk 8 bayt. `1` kuralların motor sürümüdür
(`TOPOLOGY_RULES_ENGINE`): bir türün bulduğu değişirse artar ve eski işaretler yeniden değerlendirilir. Değeri değişen,
başka katmana taşınan kural başka kuraldır. Şema 190'dan önceki işaretlerin sürümü yoktur, kural değişince düşmez.

## 4. Kalite profili ve onarım tarifi

**Profil** (`topology.profile`, şema 190): `name`, isteğe bağlı `edition`, `precision` (0,000000001–0,5 m) ve
`acceptance` (en az `precision`). Topoloji toleransı `2 × precision`'dan küçükse profil geçersizdir. Kural penceresi
Tolerans'ın yanında Kalite profili, Depolama hassasiyeti ve Kabul eşiği alanlarını gösterir; adı boş profil yoktur.

**`cad.topology.repair` v1** (yeni komut, belge yazar, tek geri alma adımı “Topoloji düzelt”):

- Girdi `steps` (1–1000): her biri bulgunun kalıcı anahtarı ve düzeltmesi (ADR 0202 §4'ün anahtarları). Her adımdan önce
  kurallar önceki adımların bıraktığı çizimde çalışır; anahtar o çizimdeki bulgunun anahtarıdır.
- Retler sırayla: `invalid_input`; sürüm (`invalid_revision`, `revision_conflict`); adım adım `finding_not_found`
  (`steps[i].key`; ilk adım dışında “önceki adımlar onu giderdi” der), `repair_refused_exception` (istisna olan bulgu),
  `fix_unavailable`, `layer_locked` (`steps[i]`: kilitli katmandaki nesneyi değiştiren adım bütün tarifi reddeder);
  son olarak `repair_beyond_acceptance` (bir kaynak köşeyi profilin kabul eşiğinden uzağa taşıyan adım). Hata diye
  onaylanmış bulgu onarılır. Durdurulan iş `cancelled` cevaplar, hiçbir şey yazmaz.
- Cevap: uygulanan tarif (adım başına anahtar, düzeltme, kural, sorun, nesneler); değişen her nesne çizim sırasıyla
  (alan ya da uzunluk önce/sonra, `removed`, kaynak köşelerinden **bit bit** yerinde kalan `kept` ve taşınan `moved`
  sayısı, en uzak `maxMoved` ve toplam `totalMoved`, onu değiştiren adımlar); bütün tarif için taşınan köşe sayısı ve
  uzaklıkları; profil varsa `belowPrecision` (hassasiyetin altında taşınanlar: yine taşınmış sayılır, yuvarlanmaz);
  alanların toplamındaki değişim (`areaChange`; silinen alanın bütünü); önce/sonra açık bulgu (`openBefore`,
  `openAfter`). Plan aynı cevabı verir, yazmaz.
- **Taşınma** (`ops::topology_rules::shift`): kaynak köşe yeni geometride tam aynı yerde (bitleriyle) köşe varsa
  yerindedir; yoksa yeni geometrinin en yakın noktasına (kenar, yay olarak yay; nokta) uzaklığı kadar taşınmıştır.
  Onarımın eklediği köşe sayılmaz: ölçülen, ölçülmüş olandan sapmadır. Hiçbir koordinat hassasiyete yuvarlanmaz.
- Yazma `cad.entities.edit`'in yazmasından geçer (`topologyFix`, var olan denetimleriyle).

**Masaüstü** Topoloji sekmesi: Durum sütunu (Değerlendirilmedi / Hata / İstisna / Yeniden değerlendir ve gerekçe),
Gerekçe alanı, Hata işaretle, Tarifle onar (seçili bulgular gösterildikleri sırayla, her biri ilk düzeltmesiyle; önce
plan, şerit tarifin etkisini söyler; Uygula tek adımda yazar, Vazgeç bırakır).

## 5. Kanıt

- `scripts/fixtures/topology_repair_cases.py` (KentOS kodu yok): bulgular ve anahtarlar ADR 0202/0524 başvurularından,
  düzeltmelerin bıraktığı halkalar ADR 0202 §4'ten elle, alanlar kesirlerle, köşe uzaklığı kesirlerle ve `decimal`'ın
  50 basamaklı karekökü ile (`10/√401` m). `cad.topology.repair.json` 27 durum: tek ve iki adımlı tarifler, uygulama ve
  geri alma, **yeniden açılan projede aynı tarifin aynı değişiklik kümesi** (`reopen` adımı `.kcad`'e yazıp okur),
  sıradaki adımın giderdiği bulgu, istisna, hata işaretli bulgu, **kilitli komşu** (ilk adım da yazılmaz), kabul eşiği,
  silinen çizgi, sürüm retleri.
- `topology_check_cases.py`: bütün bulgulara `state`; `cad.topology.exceptions.json` 21'den 39 duruma: hata işareti,
  gerekçe, kapsam, profilin gerekçe istemesi, kural sürümünün yazılması (Python'da ayrıca hesaplanır) ve düşmesi.
- `crates/shared/geometry-core/tests/all/topology_quality.rs`: yaylı ortak kenarlı komşular (çakışma iki daire
  parçasının farkı, kapalı biçimden), delikli alan ve adası, yeniden okunan komşu (kaldırılan ve yeni kimlikle gelen),
  her yerde geçerli istisna: her düzenlemeden sonra artımlı ve tam denetim aynı; `shift`'in bilinen değerleri.
- `topology_repair::tests`: durdurulan plan ve uygulama kaynak geometriyi, sürümü, kirliliği ve geri alma zincirini
  değiştirmez.
- `.kcad` şema 190: `topology-reviews.json/.kcad` ve dört bozuk örnek; `kcad_v2_reference.py` ve `tools/kcad/kcad.py`
  birlikte.

## 6. Sınırlar

- Ortak sınırın bir kez düzenlenmesinin bütün yetkili komşularda atomik uygulanması (kilitli/yetkisiz komşu, yay, delik,
  üçlü birleşim, çoklu köşe için açıklanabilir ret) bu dilimde **yapılmadı**: ADR 0160'ın topolojik düzenleme kipi
  kilitli komşuyu değiştirmeden düzenlemeye devam eder; MAP-07'nin bu satırı açıktır.
- Tarif bulguyu kalıcı anahtarla anar; bulgu türü ya da kural düzeyinde (“bütün kısa kenarlar”) seçim yoktur (kör
  toplu düzeltmeyi bilerek dışarıda bırakır).
- Büyük toplu snap/temizleme (ADR 0148, `tool.topology`) bu raporu henüz vermez: taşınan toplam/en çok uzaklık yalnız
  `cad.topology.repair`'de.
- Onarım adımları her adımda bütün kuralları tam çalıştırır; büyük çizimde artımlı yeniden denetim sonraki iş.
- GIS Processing ve veri formu yolu, kurum kabulü ve resmî eşik değerleri yok.
