# ADR 9353: Analiz araçları veri katmanlarının satırlarını okur: işlem araçları, ağlar, veri karşılaştırma

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, kullanım turları 7 ve 8
  (`gis-eksik-27-11-2026.md`, T28 P1, T29 P2, T32 P1, T36 P1). Sahibin sözü: “Gis eksik 27 11 dosyasındaki yeni maddeleri
  tamamla”.
- **İlgili:**
  - [ADR 9340](9340-data-layers-in-select-by-location-and-the-inspector.md) ve
    [ADR 9345](9345-expression-select-over-data-layers-and-results-by-layer.md) (seçim araçları satırlarda);
  - [ADR 9344](9344-file-exports-write-data-layer-rows.md) (dışa aktarmalar satırlarda, kaynak kopyası);
  - [ADR 9101](9101-data-layers.md) (veri katmanı), [ADR 0209](0209-network-analysis.md) (ağ analizi),
    [ADR 0210](0210-temporal-layers-and-scenarios.md) §8 (veri karşılaştırma).

## Bağlam

Bir veri katmanının satırları kaynağındadır, çizimin nesneleri değildir (ADR 9101). Seçim araçları (ADR 9340, 9345) ve
dışa aktarmalar (ADR 9344) satırları okuyordu; öbür analiz araçları okumuyordu:

- **T28:** Tampon, yalnız veri katmanları olan projede Katman, Görünen ve Tümü'de uygun nesne bulmadı. Düzenleme
  oturumunda çizilen bir nokta ise “1 nokta” diye bütün katman sayıldı; kaynaktaki üç nokta sessizce dışarıda kaldı.
- **T29:** Anahtarla birleştir “Bu nesnelerde öznitelik alanı yok” dedi; tabloların `fid` ve `ad` alanları vardı.
- **T32:** Ağ tanımı veri katmanını kabul etti, ama En kısa yol, Hizmet alanı ve Şebeke izleme “tıkladığınız yerin
  20 piksel yakınında ağ yok” dedi. Ağ kurucusu kenarları çizimin nesnelerinden alıyordu.
- **T36:** Veri karşılaştır iki dolu kaynak katmanında yeşil onayla “karşılaştırılacak nesne yok” dedi.
  - Dosyadan okunan çizimin veri katmanları satırsızdı.
  - Ayrı sekmede açık proje Eski veri seçicisinde yoktu.

## Araştırma

- **QGIS, İşleme:** araçlar girdi katmanının sağlayıcısından (GeoPackage, PostGIS) okur; düzenleme belleğindeki
  değişiklikler de girdidedir
  ([Buffer](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectorgeometry.html#buffer)).
- **QGIS, ağ analizi:** ağ, kaynak çizgi katmanının sağlayıcısından kurulur
  ([Network analysis](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/networkanalysis.html)).
- **KentOS kararı:**
  - Analiz araçları veri katmanını satırlarıyla okur.
  - Okuma, satırların çizimin bir kopyasına nesne olarak konmasıdır (ADR 9340'ın yolu). Düzenleme oturumunun bekleyen
    satırları kaynağınkilerle katman ve anahtar üzerinden birleşir.
  - Sonuç çizime yazılır.

## Karar

### 1. İşlem araçları (T28, T29)

- **Girdi:** her işlem aracının nesne girdileri (Katman, Görünen, Tümü, Seçili) veri katmanlarının satırlarını alır.
  - Satırlar kopyadadır: kaynağın satırları (oturumun tuttukları dışında), sonra katmandaki çizim nesneleri.
  - Oturumda eklenen satır girer; değiştirilen bir kez, değişmiş hâliyle girer; silinmesi bekleyen girmez.
- **Sayım:** pencere girdiyi kopyada sayar. Girdinin satırı kapsamdaki veri katmanlarını ve oturumda bekleyeni söyler:
  “Kapsamdaki veri katmanları: bakim (6 kaynakta, bekleyen 1 yeni satır).”
- **Çalıştır:** araç girdiyi kopyadan okur, sonucunu çizime yazar: yeni katman ve nesneleri, tek geri alma adımı.
  - Okuma tarafı kopya, yazma tarafı çizimdir (`data_inputs.rs` `RowsStage`).
  - Dosyaya ya da veritabanına yazan yol (`sink`) da kopyadan okur; tabloda seçili satırlar Seçili girdisine girer.
- **Girdisini yerinde değiştiren araç** (Alan hesapla, Anahtarla birleştir'in hedefi gibi `writes` girdileri): veri
  katmanının satırlarını almaz. Çalıştırmadan önce nedenini ve ne yapılacağını söyler: katmanı Düzenle sekmesinde
  düzenlemeye açıp değişikliği orada yapın ya da girdi olarak bir çizim katmanı seçin.
- **Anahtar alanları:** Anahtarla birleştir'in alan listeleri satırların alanlarından gelir.
- **Okunurken:** kaynak okunurken Çalıştır bekler ve söyler; okunamayan kaynak adıyla söylenir.
- **Yerine geçen yol:** seçim araçlarının yolu (ADR 9340, 9345) değişmedi.

### 2. Ağlar (T32)

- **Kurulum:** bir ağın kenar ya da kavşak katmanları arasında veri katmanı varsa ağ, satırlı kopyadan kurulur. Kurucu
  kopyanın nesnelerini kendi geometri deposuna alır; kurallar, maliyetler ve yönler değişmez.
- **Yeniden kurma:** kopya değişince (çizim değişti, kaynağın satırları geldi ya da yazıldı) ağ yeniden kurulur.
- **Uyarılar:**
  - Kaynak okunurken: “ağın katmanlarının satırları kaynaktan okunuyor; birazdan yeniden deneyin.”
  - Okunamayan kaynak: “… katmanının kaynağı okunamadı: …. Ağ kurulmadı.”

### 3. Veri karşılaştır (T36)

- **Bu çizim:** karşılaştırmada ve anahtar alanı listesinde açık çizim satırlı kopyasıyla okunur.
- **Dosyadan ve revizyondan:** okunan çizimin veri katmanlarının satırları kaynaklarından okunur
  (`data_write::all_rows`). Okunamayan kaynak dosyanın ve katmanın adıyla söylenir.
- **Açık sekmeler:** başka sekmede açık çizimler Eski veri seçicisinde “Sekme: …” diye durur.
- **Sonuç:**
  - Pencerenin özeti “6 nesne karşılaştırıldı: 6 aynı.” der.
  - İki tarafta nesne ya da satır yoksa “karşılaştırılacak nesne yok: …” uyarıyla söylenir, yeşil onayla değil.
  - Günlük satırı önceki biçimdedir (“Karşılaştırıldı: 6 aynı.”); izler onu okur.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| İşlem araçlarının veri satırlarını okuması | yok: Mimari'de İşlemler ve veri katmanı yoktur | var: CBS tabloları altlık olabilir | var: aynı | var |
| Ağların veri satırlarından kurulması | yok: aynı gerekçe | yok: ağ araçları CBS'nindir | yok: aynı | var |
| Veri karşılaştırmanın satırları, açık sekmeler | yok: aynı gerekçe | var | var | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok (bu dilimde) | `cad.processing.run` çizimin kendisinde çalışır; veri katmanının satırlarını okumaz. Kanalda aynı okuma ayrı iştir (Açık). |
| MCP | yok (bu dilimde) | Aynı gerekçe. |
| Yapay zekâ ajanı | yok (bu dilimde) | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Kanallar:** `cad.processing.run`'ın veri satırlarını kanallar için de okuması (dışa aktarmaların
  `package_data::typed_with_rows` yolunun işlem için karşılığı) ayrı iştir.
- **Arka plan:** satırlı girdi bu dilimde arayüzün iş parçacığında çalışır; 2 000 nesneden büyük girdide arka plan
  yolu satırlı kopyayı da taşımalı.
- **Ağın kenar sayısı:** Ağlar penceresi kenar ve düğüm sayısını Denetle ile söyler; kaynak durumunu ayrıca yazmaz.
- **Gerçek pencere:** bu dilimin akışları fare ve klavyeyle denendi (Doğrulama); `scripts/usage/real/`'ın Linux koşusu yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `data_inputs_tests` (3 geçti):
    - Tampon `bakim`'de kaynağın 6 satırını sayıyor, Tümü 9; Çalıştır 6 tampon yazıyor, tek geri alma kaldırıyor;
    - oturumda çizilen bekleyen noktayla girdi “6 kaynakta, bekleyen 1 yeni satır” diyor, 7 tampon yazılıyor;
    - Alan hesapla satırları almıyor ve nedenini söylüyor;
    - Anahtarla birleştir'de hedef `mahalle`, kaynak `ad` alanlarını sunuyor.
  - `data_compare::tests::two_data_layers_are_compared_by_their_rows`: iki GeoPackage'ın `bakim`'i konumla “6 aynı”;
    dosyadan okunan çizim iki katmanın 12 satırını getiriyor.
  - `-- processing:: data_compare exchange:: data_rows module_tests ribbon_tests history::text catalog settings`: 511
    geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır (render birleşmesinin bağları).
- **CBS paketi:** `networks::data_rows_tests::denetle_builds_the_network_over_the_data_layers_rows`: üçgenin üç kenarı
  veri satırı; Denetle 3 düğüm, 3 parça, 120 m. `cargo test -p kentos-desktop-cbs` 64 geçti.
- **Programlar:** `cargo test -p kentos-cad` 37 geçti; veri karşılaştırma izleri günlük satırını önceki biçimde okuyor.
- **Resim** (`.run/gui-evidence/kullanim-9353/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-tampon-veri`: Tampon, `koseler` Katman: “3 nokta … Kapsamdaki veri katmanları: koseler (3).”;
  - `02-veri-karsilastir`: “6 nesne karşılaştırıldı: 6 aynı.”, altı satır.
- **Gerçek pencere** (27 Kasım 2026, macOS; sistem imleci ve klavye olaylarıyla, yalıtılmış kullanıcı dizininde, sınama
  GeoPackage'ı üzerinde; resimler `.run/real-9353/shots/`, Git'e eklenmez):
  - Tampon `bakim`'de “6 nokta … Kapsamdaki veri katmanları: bakim (6)”; oturumda değişen satırla “5 kaynakta, bekleyen
    1 değişen satır”.
  - `yollar` veri katmanından kurulan ağda En kısa yol ve Hizmet alanı çalıştı.
  - Veri karşılaştır: kaydedilen `.kcad` Dosyadan ile okundu (“17 nesne”), `bakim` iki tarafta “6 nesne karşılaştırıldı:
    6 aynı.”
  - Kusur bulundu ve düzeltildi: ağ kurulurken uyarı projenin bütün veri katmanlarını sayıyordu (“bakim, mahalleler,
    yollar”). Artık yalnız ağın kendi katmanlarını bekler ve söyler (“yollar”); okunamayan kaynak da yalnız ağınki
    sayılır.
