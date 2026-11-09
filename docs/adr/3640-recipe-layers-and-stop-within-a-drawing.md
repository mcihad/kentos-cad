# 3640 — Tarifin katmanları adıyla ve çizimin kendi hesabının içinde durdurma

Tarih: 2026-11-09. plan/map-cad.md MAP-20, altıncı tur. Durum: taslak. ADR 3240'ın “Sonra”sındaki “katmanların da adla
eşlenmesi ya da tarife katman tanımlarının (ad, stil) girmesi” ve ADR 2440 ile 3240'ın “bir çizimin kendi hesabının
içinde durdurma (koridorun kesitleri arasında)” maddeleri.

İlgili:
- [ADR 0311](0311-layer-flags-states-find-translate.md) §5: Katmanları çevir (`cad.layers.translate`, AutoCAD'in LAYTRANS'ı);
- [ADR 1940](1940-project-graph.md): İş grafiği ve Tarifi kaydet…;
- [ADR 2440](2440-rebuild-as-a-job-progress-and-stop.md): yeniden üretim bir iş, ilerleme ve durdurma;
- [ADR 3240](3240-produce-a-recipe-in-another-project.md): Tariften üret.

## Dayanak

- **AutoCAD 2024**, [Layer Translator Dialog Box](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-597F7140-6BB9-444C-8F14-5E5655556B69.htm)
  (9 Kasım 2026'da okundu):
  - Map Same, iki listede aynı adı taşıyan bütün katmanları eşler; Map, seçilen katmanı seçilen hedefe eşler;
  - New, çeviri için yeni bir katman tanımlar. Var olan bir katmanın adıyla yeni katman açılamaz;
  - hedef katmanlar bir çizimden, şablondan ya da standart dosyasından yüklenir. Aynı adlı ikinci katman yok sayılır,
    ilki kalır.
- **QGIS API**, [QgsFeedback](https://api.qgis.org/api/classQgsFeedback.html) (9 Kasım 2026'da okundu):
  - olay döngüsü olmayan iş parçacığı `isCanceled()`'ı düzenli yoklayarak durmasının istenip istenmediğine bakar;
  - `cancel()` ana iş parçacığından çağrılır; ilerleme yüzde olarak verilir (0–100).
  - QGIS'in alt adımlı geri bildirimi bir adımın ilerlemesini bütünün bir aralığına yerleştirir (ADR 2440'ın okuması).
- **KentOS bugün:**
  - Katmanları çevir katmanı katlanmış adıyla bulur (I → ı, İ → i, sonra küçük harf; ADR 0311 §5). Hedef yoksa onu kuralın
    stiliyle ekler;
  - tarif çizimin katmanını yalnız kimliğiyle taşıyordu (ADR 3240 Sınırlar). Başka şablondan açılan projede katman yoksa
    çizim reddediliyor ya da bırakılıyordu; aynı kimlik orada başka bir katmanınsa çizim o katmana gidiyordu;
  - yeniden üretim ve Tariften üret durdurmayı yalnız çizimler arasında dinliyordu (ADR 2440). Bir koridorun şevli planı
    yüzlerce kesit çözer; durdurma o çizim bitene dek beklerdi, ilerleme çubuğu da o sürede kımıldamazdı.
- **Bulunan:** İş grafiği'nin MCP testi (`crates/native/mcp/tests/project_graph.rs`) ADR 2840'tan beri durumların
  yakaladığı revizyonun yer tutucusunu (`$r1`) tanımıyordu ve kırıktı. Yer tutucu artık bir revizyon metni olarak
  kabul ediliyor.

## Karar

1. **Tarif katmanın tanımını taşır** (`road.project.graph`, ek alan; komut sürümü değişmez): her çizimin tarifinde
   `layer`, yani çizimin katmanının kimliği, adı, rengi, yerleşik çizgi türü ve çizim kalınlığı (mm). Proje çizgi türü en
   yakın yerleşik türüyle gelir. Katman yoksa ya da grupsa alan yazılmaz.
2. **Tariften üret çizimin katmanını şöyle bulur** (`road.project.produce`; komut sürümü değişmez):
   - `layers` (yeni girdi) tarifin katmanının kimliğinden bu projenin katmanına bir eşlemedir. Verilen katman her şeyden
     önce gelir (AutoCAD'in Map'i);
   - verilmediyse ve tarif tanımı taşıyorsa, burada o adı taşıyan tek katman seçilir (Katmanları çevir'in katlamasıyla;
     AutoCAD'in Map Same'i). Tarifin kimliği burada başka bir katmanın kimliği olsa da ad seçer (kaynaklardaki gibi,
     ADR 3240 §2);
   - o adda katman yoksa tarifteki ad, renk, çizgi türü ve kalınlıkla en alta yeni katman eklenir (AutoCAD'in New'i).
     Hiçbir çizim ona çizilmezse adımın içinde geri alınır; kalan her eklenen katman `recipe_layer_created` uyarısıyla
     söylenir;
   - o adda iki ya da daha çok katman varsa hangisi olduğu sorulur. Bütün çizimler istenince çizim bu soruyla bırakılır
     (`refused`). Adıyla istenince ret gelir: `ambiguous_recipe_layer`, `layers`'ta, iletide tarifin çizimi adlanır;
   - tanım taşımayan eski tarif katmanı kimliğiyle bulur (ADR 3240'ın kuralı).
3. **Girdinin denetimi:** `layers`'ın anahtarı tarifteki çizimlerin bir katmanı değilse `unknown_recipe_layer`
   (`layers`'ta). Değeri, burada nesne alan bir katman olmalıdır; değilse `layer_not_found`, `not_a_layer`,
   `service_layer` ya da `layer_locked` (`layers.<kimlik>`'te). Tanımın kimliği çizimin girdisindeki katmanınki değilse
   `invalid_project_recipe`.
4. **Cevap:** `layers`, çizilen çizimlerin gittiği her tarif katmanı (ilk kullanıldığı sırayla): tarifteki kimliği ve adı
   (eski tarifte buradaki adı), buradaki katman ve nasıl bulunduğu (`given`, `name`, `id`, `created`). Var olan çizim
   denetimi (aynı komut, ana kaynak ve katman) bulunan katmanla yapılır. Tutulan çizimin girdisi bu projenin katman
   kimliğini taşır, böylece yeniden üretim aynı katmana çizer.
5. **Çizimin kendi hesabının içinde durdurma:**
   - koridorun çözümü (`road_corridors::make_ticked`) her kesitten önce ilerlemesini söyler. Her bölge bütünün eşit bir
     payıdır, bölgenin içinde kesit kesit ilerler. Durdurma istenmişse orada `cancelled` döner;
   - şevli plan (`road.corridors.draw`) bir işin parçası olarak koşunca işin ilerlemesini ve durdurmasını kullanır.
     Doğrulama ve plan yalnız durdurmayı dinler, ilerleme yazımındır;
   - yeniden üretim ve Tariften üret her çizime işin ilerlemesinden kendi payını verir (`in_share`, işin `span`'ı):
     çizimin kendi ilerlemesi o payın içinde ilerler, durdurma işinkidir. Durdurulan iş bugünkü gibi hiçbir şey yazmaz.
6. **Masaüstü** (İş grafiği › Tariften üret…):
   - tarif seçilince önce gösterilir: tarif dosyası ve tarifin her katmanı için bu projenin katmanı (Adıyla bul, yoksa
     ekle ya da bir katman). Altında planın sözü yer alır: üretilecek çizimler, her katmanın nereye gittiği (adıyla
     bulundu, eklenecek, seçilen), bırakılacaklar ve uyarılar;
   - Üret çizer, Vazgeç bırakır; Üret'e dek hiçbir şey yazılmaz. Üretimden sonraki ileti katmanların nereye gittiğini de
     söyler.
7. **Kapsam işareti:** ADR 3240 gibi **Harita + Yol**.

## Sınırlar

- Bellek ve CPU bütçesi yoktur. KentOS'un bellek ölçümü ve iş başına sınır koyacak bir ölçüsü yok; gerçek veriyle ölçüm
  yapılmadan sınır değeri uydurulmaz (MAP-20 kabulündeki ölçüm raporu ile birlikte).
- Kendi hesabının içinde duran çizim bugün koridorun şevli planıdır. Kesitler, istinat duvarı, eş yükselti eğrileri ve
  kazı-dolgu çizimleri hâlâ çizim bitince durur.
- Eklenen katman en üst düzeye eklenir. Tarifin katmanının grubu taşınmaz; katmanın dolgusu, nokta ve etiket stili de
  tarife girmez.

## Kanıt

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/road_project_graph_cases.py`: her çizimin tarifine kurulumun katmanının tanımı; 21 durum;
  - `scripts/fixtures/road_project_produce_cases.py`: 46 durum (31'i ADR 3240'ın, çıktıları artık `layers` ile).
    Altı yeni tarif `fixtures/recipes/v1` altında elle yazıldı. 15 yeni durum: adla (tarifin kimliği başka; Türkçe büyük
    harfler; kimliği burada başka bir katmanın), tanımıyla eklenme ve ikinci kez adıyla bulunma, plan, çizilemeyen
    çizimin eklenen katmanının geri alınması, adıyla istenen çizimde ret ve katmanın eklenmemesi, aynı adlı iki katman
    (bütün çizimlerde bırakılır, adıyla istenince ret, `layers` ile verilen), verilen katmanın addan önce gelmesi ve
    dört ret (tarifte olmayan anahtar, projede olmayan ve kilitli katman, tutmayan tanım).
  - Yol projesi ortak durumları (graph, produce, query, rebuild, edit), koridor durumları ve kod listesi denetimi Rust'ta,
    Python'da geçer.
- **Rust birim testleri:**
  - `road_corridors_draw::tests`: şevli planın kesitleri tek bir bütünün içinde artarak söylenir; dördüncü kesitte
    istenen durdurma `cancelled` döner ve ondan sonra kesit çözülmez. Durdurulmuş işin parçası olarak şevli plan hiçbir
    şey çizmez;
  - `road_project_rebuild::tests::a_drawing_s_progress_is_told_within_its_share`: dört çizimin ikincisinin yarısı işin
    0,375'idir; payın içindeki durdurma işinkidir. ADR 2440 ve 3240'ın iş testleri değişmeden geçer.
- **Bağımsız Python testi** (`python/tests/test_recipe_layers.py`):
  - A'nın tarifi katmanın tanımını taşır;
  - katmanı olmayan B'de katman tarifin görünüşüyle eklenir (uyarıyla), B'nin çizgileri A'nınkiyle koordinat koordinat
    aynıdır, ikinci kez hiçbir şey yazılmaz;
  - katmanı “SIFIR HATTI” adıyla başka kimlikte olan C'de o katman bulunur ve grafik güncel tutar;
  - aynı adlı iki katmanlı D'de çizimler soruyla bırakılır, `layers` ile verilene çizilir.
- **MCP** (`crates/native/mcp/tests/recipe_layers.rs`):
  - eklenen katman, grafiğin tarifinde tarifin görünüşüyle görünür; ikinci çağrı iki `exists` verir;
  - başka kimlikteki aynı adlı katman bulunur, verilen katman kullanılır.

  İş grafiği'nin MCP testi düzeltmeyle (Bulunan) geçer.
- **Masaüstü** (`project_graph_window_tests::tariften_uret_shows_the_recipe_s_layers_before_it_draws`):
  - seçilen tarif önce gösterilir, “Kazı sınırı”nın ekleneceğini söyler, bu sırada hiçbir şey yazılmaz;
  - Drenaj seçilince yeniden planlanır, Üret iki çizimi Drenaj'a çizer, katman eklenmez; Vazgeç tarifi bırakır.

  ADR 3240'ın masaüstü testi yeni akışla (Üret) geçer, iletisi bulunan katmanı da söyler. Proje ağacı testleri ve
  çapraz denetimler geçer. Resimler `.run/gui-evidence/tarif-katmanlari-3640/`.

## Sonra

- Bellek ve CPU bütçesi, gerçek veriyle ölçümle birlikte (Sınırlar).
- Kesitler, istinat duvarı, eş yükselti eğrileri ve kazı-dolgu çizimlerinin kendi hesabının içinde durması ve
  ilerlemesi.
- Tarifin katmanının grubu ve tam stili.
- Ölçüm arşivinin zincire bağlanması; atlas sayfalarının denetimi.
- Web (TODOS §0.2).
