# 4320 — Yüzeyden çizilenlerin yalnız değişen kısmının yeniden yazılması

Tarih: 2026-11-10. plan/map-cad.md MAP-14, sekizinci tur. Durum: taslak. ADR 3920'nin ve 3520'nin “Sonra”sındaki
“bölgesel yeniden kurma”; planın maddesi: “Kaynak değiştiğinde yalnız etkilenen bölge yeniden kurulabilsin.”

İlgili:
- [ADR 0900](0900-road-terrain-surface.md) §10: yüzey tanımıyla saklanır, üçgenleri saklanmaz; açılışta kaynaklardan
  kurulur;
- [ADR 0911](0911-road-project-tree-views-and-rebuild.md): yol çizimlerinin kaydı (kaynak damgaları, nesneler) ve `road.project.rebuild`;
- [ADR 0916](0916-road-surface-analysis-and-quality.md) §7: parçalı yüzey (karolar, kesinleştirilen üçgenler).

## Dayanak

- **Esri, ArcGIS Pro 3.1, Edit a terrain dataset**
  ([belge](https://pro.arcgis.com/en/pro-app/3.1/help/data/terrain-dataset/edit-a-terrain-dataset.htm), 10 Kasım 2026'da
  okundu):
  - ölçü düzenlemesi “kirli alan” bırakır: arazinin geçersiz olan ve yeniden kurulması gereken kısımları;
  - kirli alanlar karo tabanlıdır: bir karodaki düzenleme bütün karoyu geçersiz kılar;
  - kurma kirli karoyu ve yakın komşularını işler, toplam dokuz: düzenlemenin etkisi karo sınırını aşabilir.
- **Esri, ArcMap 10.7, Displaying dirty areas of a terrain**
  ([belge](https://desktop.arcgis.com/en/arcmap/10.7/manage-data/terrains/displaying-dirty-areas-of-a-terrain.htm),
  10 Kasım 2026'da okundu): arazi nesne düzenlemelerinden haberdar edilir, düzenlenen alanların kaydını tutar; kirli
  alanlar katmanın simgelemesinde çizgiyle gösterilir.
- **Autodesk, Civil 3D 2025 kılavuzu**
  ([belge](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-UserGuide/files/GUID-EA66F3B2-5C3B-4397-92F9-04869060FEAA.htm),
  10 Kasım 2026'da okundu): verisi ya da kurma ayarı değişen yüzey eskiyebilir ve yeniden kurulur; Prospector'da eskime
  işareti; Rebuild - Automatic açıkken değişince kendiliğinden kurulur. Belge kurmanın yüzeyin bir kısmıyla sınırlı olup
  olmadığını söylemiyor.
- **KentOS bugün:**
  - Yüzeyin üçgenleri saklanmaz, her kullanımda kaynaklardan kurulur (ADR 0900 §10). Ölçüm: 200 000 noktada 98 ms
    (ADR 0916 §7). Esri'nin kirli karosu saklanan bir üçgen piramidini kısmen kurmak içindir; KentOS'ta öyle bir şey yok.
  - Saklanan, yüzeyden çizilenlerdir: eş yükselti eğrileri, sıfır hattı, kazı/dolgu bölgeleri, boykesit ve enkesit
    çizimleri. Hepsi kaynak damgalarıyla yol çizimi olarak kaydedilir (ADR 0911).
  - `road.project.rebuild` eskiyen çizimin bütün nesnelerini siler, yenilerini yazar. Bir tepedeki tek nokta değişse de
    bütün eğriler yeni kimlikle yazılır.
  - Delaunay'ın eş çemberli dört noktadaki seçimi girdinin sırasına bağlıdır (ADR 0232 §4). Esri'nin karo ve komşu
    kuralı bu yüzden bütün yeniden çizimle aynı çizgileri güvenceye almaz.
- **Karar için:** bölge karo kuralıyla tahmin edilmez. Çizim bütün olarak yeniden yapılır, sonucu var olanla tam
  karşılaştırılır; yalnız farklı çıkan nesneler yazılır.

## Karar

1. **`road.project.rebuild`'e `regional`** (öntanımlı kapalı; v1'in davranışı aynı kalır). Masaüstünde Proje ağacı'nın
   **Yalnız değişeni yaz** kutusu, öntanımlı açık.
2. **Kural.** Her çizim önce projenin bir kopyasında, yeniden üretimin yoluyla yeniden yapılır: nesneleri silinir,
   komutunun denetimi ve yazması koşar. Sonra:
   - **Aynı çıkan çizim:** aynı nesneler aynı sırada, hiçbiri eksik değil. Hiçbir nesne yazılmaz. Kaydın kaynak
     damgaları yenilenir; nesneleri yapan yazmanın sürümü ve revizyonu kalır. Güncel çizim adıyla istenirse hiçbir
     şey değişmez.
   - **Çizgi çizimi:** eski ve yeni her nesne yazısız bir çizgi ya da çoklu çizgi, hepsi tek renk ve tek saydamlıkta.
     Tek renkli çizgiler hangi sırayla çizilirse çizilsin aynı pikselleri o renkle örter; çizim sırası resmi
     değiştirmez. Her yeni çizgi, alınmamış ve tıpatıp aynı (koordinatları bit bit) ilk eski çizginin yerine geçer; o
     eski çizgi kimliği ve belgedeki yeriyle kalır. Kalan eskiler silinir, kalan yeniler çizimin sırasıyla sona yazılır.
   - **Değişen başka çizim:** dolgu, yazı, maske ya da iki renk; çizim sırası resmin parçasıdır. Bütün yeniden yazılır,
     nesneleri komutun sırasında kalır.
   - **Kopyada nesnelerden fazlası değiştiyse** (ayar, katman, biçem, blok, pafta): çizim projede, `regional`
     olmadan olduğu gibi yeniden üretilir.
3. **Karşılaştırma:**
   - nesnenin JSON'u, belgedeki numarası (`id`) olmadan;
   - çizimin bir nesnesinin andığı çizim nesnesi (etiketin nesnesi) çizimdeki sırasıyla okunur, kimliğiyle değil;
   - kopyada yapılan nesneler projeye kopyada aldıkları kimliklerle taşınır, birbirini anışları bozulmaz.
4. **Cevap:** her çizimde `kept` (yerinde kalan nesne). Çizgi çiziminde `region` da yazılır: silinen ve yazılan
   çizgilerin kutusu (en küçük x, y, en büyük x, y), yani değişen bölge. Masaüstü cümlesi: “1 çizim yeniden üretildi:
   11 nesne yerinde kaldı, 8 yazıldı, 5 silindi; değişen bölge 63.248 m × 48.601 m.”
5. **Eşdeğerlik:** sonuç bütün yeniden çizimle aynıdır: aynı nesneler aynı sırada, aynı kayıt özeti ve kaynak damgaları.
   Fark yalnız kalan nesnelerin kimlikleri ve belgedeki yerleridir. Geri alma tek adımdır (“Yeniden üret”).
6. **Kapsam işareti:** **Harita + Yol**. Bulut eşitlemesinde değişmeyen nesneler gönderilmez (kimlikleri aynı).

## Sınırlar

- Kalan çizgiler belgedeki yerlerinde durur. Aynı katmana çizimden sonra çizilmiş başka bir nesnenin altında kalırlar,
  yeni yazılanlar üstüne çıkar. Bütün yeniden çizimde hepsi üste çıkardı.
- Hesap bütündür: yüzey ve çizim baştan yapılır. Yalnız yazma bölgeseldir. Karolu hesap gerçek veride ölçülünce ayrı
  karar.
- Kendiliğinden yeniden üretim (ADR 0911 §3) bütün çizer.
- Pafta eğrileri (`road.terrains.sheetcontours`) kayıtlı çizim değildir; yüzeyin sürümüne bağları yok (Sonra).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/road_regional_rebuild_cases.py`; KentOS kodu yok), 23 durum
  (`fixtures/commands/v1/road.project.rebuild.regional.json`):
  - zemin 48 noktalı, iki tepe ve bir çukur; her durumda üçgenlemenin tek olduğu doğrulanır;
  - eğriler `road_terrain_command_cases.py`'nin Triangle üstündeki kesirli hesabı;
  - durumlar: kot değişimi, kaydırma, eklenen ve silinen nokta, kırık hat; hiçbir çizginin değişmediği kot; güncel
    çizim adıyla; elle silinen ve elle değişen eğri; elle boyanan eğri (bütün); iki yüzeyin ve bir yüzeyin iki aralığın
    çizimleri; bütün kotlar; geri al ve yinele; plan ve doğrulama; kilitli katman, kurulamayan ve silinen yüzey,
    mimari proje ve revizyon retleri;
  - beklenen `ids` belgenin nesne sırasıdır: kalan çizgiler numaralarını korur.

  Eski 24 yeniden üretim durumu değişmedi. Rust ve Python geçer. Bozma denemesi: `kept`'i bir artırınca koşucu
  yakalar.
- **Rust birim testleri** (`road_project_regional::tests`):
  - bölgesel sonuç bütünle nesne nesne, kayıt özeti ve damgalarıyla aynı;
  - kuralın renk, etiket ve tür denetimi;
  - çizim nesnesinin sırasıyla okunması;
  - kopyada ayar değişimi.
- **Python** (`python/tests/test_regional_rebuild.py`), testin kendi okumasıyla:
  - bütünle aynı çizim;
  - kalanlar testin kendi eşlemesi;
  - bölge çizgilerin yerlerinden;
  - kaydedilip açılınca güncel; adıyla istenince hiçbir şey yazılmaz.
- **MCP** (`crates/native/mcp/tests/regional_rebuild.rs`): durumun satırı (silinen, kalan, bölge), kalanların kimlikleri,
  kaydet ve aç. MCP'nin ana testleri ve yeniden üretim testleri geçer.
- **Masaüstü** (`road_project_regional_tests`):
  - kutu öntanımlı açık; kalanlar durumunki, yerleri eski;
  - cümle; geri alınınca eski çizgiler;
  - kutu kapalıyken bütün.

  Proje paneli testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/yalniz-degisen-4320/` (eskiyen ağaç
  ve kutu; yeniden üretimden sonra yazılan çizgiler seçili).

## Sonra

- Kendiliğinden yeniden üretimin de yalnız değişeni yazması (proje ayarı).
- Pafta eğrilerinin kayıtlı çizim olması: yüzeyin, detay, şev ve nokta katmanlarının damgasıyla (pafta sonuçlarının
  yüzey sürümüne bağı).
- Yalnız değişen atlas sayfalarının PDF'i (MAP-18) bu bölgeyle.
- Karolu hesap (gerçek veride ölçüm gerekir).
- Web (TODOS §0.2).
