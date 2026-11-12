# 5020 — Parsel belgesinin taslağı ve onayı, belgedeki kenar ölçüleri, yaylı sınırın bölünmesi

Tarih: 2026-11-11. plan/map-cad.md MAP-08, onuncu tur. Durum: taslak. ADR 4620'nin “Sonra”sındaki üç iş: yaylı sınırın
bölünmesi; kenarın belgedeki ölçüleri; belge taslağı ile onaylı kayıt.

İlgili:
- [ADR 4220](4220-parcel-register-documents-and-boundaries.md): parsel kayıtları, kalıcı sınır kimliği;
- [ADR 4620](4620-subdivision-splits-neighbour-boundary.md): ifrazda komşunun düz sınırının bölünmesi;
- [ADR 2000](2000-parcel-boundary-graph.md), [ADR 2600](2600-shared-edge-junctions-and-inner-vertices.md): 1 µm kuralı.

## Dayanak

Mevzuat (`.run/research/map-cad/mevzuat/MAP-08.10.md`, 10 Kasım 2026'da okundu):

- **BÖHHBÜY** (RG 21.05.2018), m. 48(1), metinden okundu (`.run/research/map-cad/mevzuat/pdf/bohhbuy/bohhbuy-pdf.txt`):
  “Ölçülerden hesaplanan ile cephelerin ölçüm değeri arasındaki fark d = 0.05 + 0.001S eşitliği ile bulunan miktardan
  fazla olamaz. Burada S, metre biriminde cephe uzunluğu ve d, metre birimindedir.” m. 49: ölçü krokisi ölçüleri taşır.
- **Planlı Alanlar İmar Yönetmeliği** m. 4(1)(a), aplikasyon krokisi: kenar ölçüleriyle birlikte köşe koordinatlarının
  yazıldığı kroki. Ölçü kenarın yanında, belgenin değeri olarak durur.
- **Tapu Plânları Tüzüğü** m. 5(2), m. 10, m. 11 ve **TKGM Genelge 2025/4** m. 6(2), m. 12(14)–(16): plan yetkili kişi
  ya da kuruluşça düzenlenir, kadastro müdürlüğünün kontrol ve tasdikinden sonra tescile konu olur; kontrol raporunu
  kontrol mühendisi imzalar, müdür onaylar. **LİHKAB Yönetmeliği** m. 22(1): lisanslı büro yapımdan sorumludur, kontrol
  müdürlüğündür. Sonuç: KentOS onay vermez; dışarıda verilen onayı yazıldığı gibi kaydeder.
- **Tapu Plânları Tüzüğü** m. 8 ve **Yanılma Sınırı Yönetmeliği** m. 4(1)(c), 8(5): yüzölçümü köşe koordinatlarıyla
  hesaplanır. Yay köşeleriyle kayda girer; yayın köşelere bölünme aralığı ya da kiriş sapması için resmî metinde kural
  yok (aranan: Tüzük, Yanılma Sınırı Yönetmeliği, Genelge 2025/4, BÖHHBÜY, UVDF).

Benzer uygulamalar (`.run/research/map-cad/uygulama/MAP-08.10.md`, resim `MAP-08.10-resim/`, 10–11 Kasım 2026):

- **ArcGIS Pro**, [Introduction to COGO](https://doc.esri.com/en/arcgis-pro/latest/help/editing/introduction-to-cogo.html):
  ölçü planında her kenarın belgedeki ölçüsü kenarın üstünde yazılı (doğruda semt ve uzunluk; yayda yay boyu L, yarıçap
  R, merkez açısı Δ); parsel çizgisinin COGO alanları (Direction, Distance, Radius, Arc Length) çizgiye aittir; Stated
  Area kayıttaki alandır, Calculated Area ölçülerden. Kayıt (record) parselleri yaratır ve emekliye ayırır; ayrı bir
  taslak durumu yoktur (bir ilçe uygulaması ön onay planı katmanında `status` özniteliğini elle tutuyor).
- **ArcMap** parsel kumaşı, Merge Courses: yay birleştirmede yarıçap ve merkez toleransı (ADR 5000).
- **Netcad NETMAP** Klasik İfraz: eski parsel PASIF_PARSEL tabakasına alınır; “Parsel Cephe Uzunlukları Yaz” kenar
  uzunluğunu yazar. (Bu turda wiki'nin sertifikası geçersizdi; bilgi önceki turun notundan.)

## Karar

1. **Belgedeki kenar ölçüleri.** Parsel kayıtları belgelerin yazdığı ölçüleri tutar (`lengths`: sınır, belge, ölçü;
   metre olarak düz ondalık, yazıldığı gibi; sıfırdan büyük; bir sınıra bir belgeden bir kez). Ölçü belgenindir:
   sınırın boyu ya da çizim değişince değişmez.
   - `cad.parcels.record`'un `create[i].lengths`'i (halka, kenar, ölçü) ya da yoksa alanın `Kenar ölçüleri (m)`
     özniteliği: dış halkanın kenarları sırasıyla `;` ile ayrılmış, boş ya da `-` yazılmamış sayılır.
   - İki alan aynı sınıra iki ayrı ölçü yazarsa ret (bir belge bir sınıra bir ölçü yazar); aynı ölçü bir kez yazılır.
   - Yeni retler `invalid_input` altında: olmayan halka ya da kenar, iki kez verilen kenar, sıfırdan büyük düz ondalık
     olmayan ölçü, özniteliğin kenarlardan çok ölçüsü.
   - Cevapta `lengths` (yazılan ölçü sayısı).
2. **Cephe denetimi.** `cad.parcels.records` her sınırın ölçülerini sınırın kayıttaki boyuyla (yayda yay boyu) okur:
   `difference` = boy − ölçü, `limit` = 0,05 + 0,001·S m (S belgedeki ölçü, m. 48(1)), `within` |fark| ≤ sınır;
   `overLimit` sınırı aşan güncel sınır sayısı. Sınır kural paketi verisi değil, maddenin kendisidir; sonuç teknik
   denetimdir, tescil kararı değildir.
3. **Belge taslağı.** `cad.parcels.record`'un `draft`'ı belgeyi taslak olarak yazar (`drafts`, belgelerden ayrı
   numaralı): alanlarının o anki kimliği, belgedeki alanı, halkaları (köşe ve kabarıklık, çizildiği gibi) ve kenar
   ölçüleri, kapatacağı parseller. Güncel parseller, sınırlar ve belgeler değişmez. Cevap onayın bugün yazacağıdır;
   `document` taslağın numarası, `draft` true. Belgenin adı belgeler ve taslaklar arasında tektir.
4. **Belgenin onayı** (`cad.parcels.approve` v1, yeni). Onay dışarıda verilir; KentOS onu yazıldığı gibi kaydeder:
   onay sayısı (boş olmaz, 64), tarihi (64), onaylayan (120).
   - `draft`: taslak, taslaktaki gibi sıradaki numarayla belge olarak yazılır, onayı yanında; taslak kalkar. Kapatacağı
     parsel o arada kapanmışsa `parcel_closed`, alanı o arada başka belgeyle kaydedilmişse `parcel_recorded`, kimliği
     başka parselindeyse `parcel_identity_taken` (yol `draft`): taslak bugün yazılamaz, geri çekilip yeniden hazırlanır.
   - `document`: kayıtlı belgenin onayı yazılır; belge değişmez. Onay bir kez yazılır (`document_approved`).
   - Yeni ret kodları: `unknown_parcel_draft`, `unknown_parcel_document`, `document_approved`. Tek geri alma adımı
     “Belge onayı”.
   - Taslağın onayla belge olması numarayı sona alır: taslak sonra kaydedilen belgelerden önce numaralanmaz, böylece
     “kapatan belge sonra gelir” kuralı (ADR 4220) bozulmaz.
5. **Taslağı geri çek** (`cad.parcels.withdraw` v1, yeni): taslak kalkar, adı yeniden kullanılabilir; yalnız taslaktan
   oluşan kayıt boşalırsa kalkar. Ret `unknown_parcel_draft`. Tek geri alma adımı “Taslağı geri çek”.
6. **Yaylı sınırın bölünmesi** (ADR 4620'nin yay karşılığı). Güncel bir komşunun kullandığı yaylı sınırın çemberine 1 µm
   içinde ve yay boyunca iki ucundan 1 µm'den uzak düşen yeni köşeler sınırı böler, parçalar yayın kendi yönünde
   sıralanır. Parça, iki köşesi arasındaki güncel yaylı sınırdır: yayının ortası çembere 1 µm içinde ve iki köşenin bu
   yanında (öbür yanın ortası bir çap uzaktadır; sınama çemberin yarıçapından yakınlıkla). Kabarıklık eşitliği
   aranmaz: 1 µm'lik köşe kayması kabarıklığı 1e-9'dan çok değiştirir. Komşunun halkası parçaları ADR 4620'deki gibi
   alır. Çember kirişten ve kabarıklıktan: merkez kirişin ortasından sol normale (1 − b²)/(4b) kiriş, R = c/(2 sin(θ/2)),
   θ = 4·atan b.
7. **`.kcad` şema 249**: parsel kayıtlarının `lengths`'i, `drafts`'ı ve belgenin `approval`'ı. Yazıcı yalnız bunlardan
   biri varken 249 yazar; 248 ve önceki okuyucu reddeder (`unknown_field`). Bilinmeyen sürüm örneği
   `broken/schema-version-250.kcad`.
8. **Masaüstü** (Parsel alan denetimi › Parsel kayıtları…):
   - Yeni belge'de “Taslak olarak yaz”; planın sözü “Taslak 1 “…”: onaylanınca …. Güncel kayıt değişmez.”
   - Belgeler tablosunda Durum (Kayıtlı, Onaylı: sayı, Taslak) ve taslaklar (T1 …); satıra basınca seçilir.
   - Seçili belge bölümü tablonun altında, Yeni belge'den önce: Onay sayısı, Onay tarihi, Onaylayan, planın sözü,
     Onayı yaz ve taslakta Taslağı geri çek. Yardım satırı: “KentOS onay vermez: … onayı yazıldığı gibi kaydeder.”
   - Sınırlar tablosunda Belgede (belgedeki ölçü) ve Fark (sınır) sütunları; sınırı aşan “aşıyor”. Özet satırı aşan
     sınır sayısını ve onay bekleyen taslakları söyler.
9. **Kapsam işareti:** ADR 4220 gibi **Harita + CBS**.

## Sınırlar

- Ölçüler kenar uzunluklarıdır; semt (doğrultu) ve yarıçap ölçüsü yok, ölçülerden alan ve kapanma hesaplanmaz. Yayda
  ölçü yay boyudur.
- Onay KentOS'ta denetlenmez; kurum kabulü yerine geçmez, “TKGM'ye uygun” denmez.
- Taslağın kapatacağı parsel kayıtta olmalıdır; taslak sonradan değiştirilemez, geri çekilip yeniden yazılır.
- Yaylı bölmede parçaların kabarıklığı çizildiği gibidir; KentOS yeni köşeyi yaya oturtmaz (çizimde düzeltme yok).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/parcel_register_cases.py`; KentOS kodu yok; çember merkezi kiriş açısından
  ayrı bir formülle):
  - `cad.parcels.record`, 54 durum (16 yeni): yaylı komşu sınırının yayın ortasında bölünmesi; köşe yaydan 2 µm
    uzakta bölünmez; verilen ve öznitelikten okunan kenar ölçüleri; iki parselin ortak sınıra aynı ölçüsü bir kez; taslak
    ve planı; 8 ret.
  - `cad.parcels.records`, 26 durum (3 yeni): fark ve cephe sınırı (20,08 ölçülü 20 m'lik kenar aşar, 10,01 aşmaz);
    taslak ve onay listesi; onaydan sonra kayıt.
  - `cad.parcels.approve`, 25 durum; `cad.parcels.withdraw`, 21 durum.
  - Rust ve Python (126 durum) geçer. Bozma denemesi: yaylı bölmenin `boundariesSplit`'i ve 20,08'in `within`'i
    değiştirilince koşucu tam o ikisini yakalar.
- **`.kcad`**: `parcel-drafts.kcad` (+ JSON) ve üç bozuk örnek (şema 248'de, sıfır ölçü, belgeyle aynı adlı taslak);
  bağımsız okuyucu `tools/kcad/kcad.py` aynı kuralları denetler; `kcad_v2_reference.py --check` 1801 dosyada tutarlı;
  kcad testleri geçer.
- **Python** (`python/tests/test_parcel_drafts_lengths.py`), testin kendi aritmetiğiyle: fark ve sınır; taslak güncel
  kaydı değiştirmez, onay 2. belgeyi yazar; geri çekilen taslağın adı yeniden kullanılır; yayın ortası çemberde
  (testin kendi merkezi: sagitta 2 m, r = 26 m), 101/9'un halkası (10, 22)'den geçer.
- **MCP** (`crates/native/mcp/tests/parcel_drafts.rs`): taslak, onay, ikinci taslağın geri çekilmesi.
- **Masaüstü** (`parcel_register_window_tests`, 8 test): pencereden taslak ortak durumun kaydını, onay ortak durumun
  kaydını yazar; belgenin onayı bir kez; geri çekme; sınırlar tablosunda ölçüler. Çapraz denetimler 177 test geçer.
  Resimler `.run/gui-evidence/belge-taslagi-onay-5020/`. Gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Sonra

- Semt ve yarıçap ölçüleri, ölçülerden alan ve kapanma (ADR 4220'deki ArcGIS'in Calculated Area'sı gibi).
- Kenar ölçülerinin pencerede alan alan girilmesi (bugün öznitelikten ya da komutla).
- Erişim denetimli hak/pay; resmî kapsam (sahibinde).
- Web (TODOS §0.2).
