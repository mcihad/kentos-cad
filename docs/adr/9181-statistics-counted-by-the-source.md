# ADR 9181: İstatistik ve Grafik veri katmanını kaynağın kendi sayımıyla sayar

- **Durum:** kabul edildi (2026-11-23); adım VY-80. Plan: [plan/veri.md](../../plan/veri.md) VY-80.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”; 23 Kasım: “küçük kalanlarla”,
  “Evet, 17'yi yap”. Değerlendirmenin 17. maddesinin kalanı ([ADR 9130](9130-statistics-and-charts-over-data-layers.md)
  §4): “Büyük tablolarda sağlayıcının kendi toplamı (`GROUP BY`) ile sayım. Bugün satırlar okunup sayılıyor.” Aynı
  ADR'nin “Grafik'in sütunlarından haritada seçim” kalanı da bu adımda kapandı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok. **Sürücü sözleşmesi:**
  değişmedi (`Source::groups`, ADR 9108 §2, olduğu gibi kullanıldı).

## §1 Bağlam ve araştırma

- İstatistik ve Grafik veri katmanının satırlarını kaynağından bütün olarak okuyup sayıyordu (ADR 9130). Her satırın
  geometrisi okunuyor, projenin sistemine çevriliyor ve bir nesne kuruluyordu. Bir milyon parselde bu, bir alanın
  sayısı için bütün tabloyu belleğe almak demekti.
- KentOS'ta aynı işin örneği vardı: katman ağacının sınıf sayıları kaynağın `GROUP BY`'ıyla sayılır (ADR 9110 §2).
- **Araştırma (okundu 23 Kasım 2026):**
  - QGIS 3.44 API belgesi ([QgsAggregateCalculator](https://qgis.org/pyqgis/3.44/core/QgsAggregateCalculator.html)):
    toplamanın katmanın kendi yöntemiyle istenmesi önerilir, çünkü katman hesabı veri sağlayıcısına devredebilir.
  - SQLite ([toplama işlevleri](https://sqlite.org/lang_aggfunc.html)): girdilerden biri tam sayı değilse `sum()`
    toplamın kayan noktalı bir yaklaşığını verir; `avg()` her zaman kayan noktalıdır.
  - PostgreSQL 19 ([9.22 Aggregate Functions](https://www.postgresql.org/docs/current/functions-aggregate.html)):
    `sum(double precision)`'ın sonucu yine `double precision`'dır.
- **Sonuç:** KentOS'un sayı kuralı (`kentos.statistics/1`, ADR 0200 §4) toplamı, en azı, en çoğu ve çeyrekleri kesin
  ondalıkla hesaplar. Kaynağın `SUM`/`AVG`'si bu kurala uymaz. Bu yüzden kaynak toplamı değil, her değerin kaç satırda
  olduğunu sayar (tam sayı, kesin); toplamı KentOS kesin ondalıkla yapar.

## §2 Karar

1. **Kaynak değerleri sayar** (`field_counts`, `crates/native/application/src/gis/field_counts.rs`).
   - Bir alan için dosya ya da veritabanı kendi `GROUP BY`'ını yapar (`Source::groups`). Her değer bir kez ve satır
     sayısıyla gelir; geometri okunmaz, nesne kurulmaz.
   - Her değer, satır okunsaydı çizimin özniteliğinde nasıl duracaksa öyle yazılır (`attribute_text`). Böylece
     kaynağın sayımı ile okunan satırların sayımı değer değer aynıdır.
   - Düzenleme oturumunun tuttuğu satırlar (alınan ve silinenler) anahtarlarıyla dışarıda bırakılır
     (`Not(Fids)`). Çizimin satırları (alınan ve yeni olanlar) çizimdeki hâlleriyle eklenir.
   - Her satırı kendi değerini tutan alan (bir kod, bir ölçü) satır başına bir değer döndürür. Bu bile geometrisiz ve
     nesnesiz olduğu için satırları okumaktan hafiftir.
   - Sınırlar: bir alanın 5 000 000'dan çok farklı değeri varsa (değerlerin belleğine konan sınır) ya da çizim
     10 000'den çok satır tutuyorsa sayım kaynakta yapılmaz; satırlar eskisi gibi okunup sayılır.
2. **Sayılan alan satır gibi sayılır** (`Grouped`, `apps/desktop/src/gis/data_counts.rs`).
   - Her değer bir hücredir. Satır listesinde hücrenin yeri, o değeri tutan satır sayısı kadar tekrarlanır. Çekirdeğin
     istatistik ve grafik hesabı değişmeden çalışır ve her satır okunmuş gibi sonuç verir. Satır başına bellek
     4 bayttır; kaynağın döndürdüğü değerler kopyalanmaz, sayımla paylaşılır.
   - Toplam, ortalama, en az, en çok, çeyrekler, sayılar ve paylar kesindir; okunan satırlarınkiyle aynıdır.
   - **Std. sapma** kuralın tek çift duyarlıklı sayısıdır (“değerlerin sırasıyla iki geçiş”). Kaynağın sayımında
     değerler grupların sırasıyla toplanır. Okunan satırların sırasıyla toplamadan son ikili basamaklarda
     ayrılabilir; ortalamanın ölçeğinde yazılan metin sınanan bütün durumlarda aynı çıktı.
3. **İstatistik** veri katmanında Göster: Tümü iken her alanı kaynağa saydırır.
   - Özet tablosu alanların sayımı geldikçe dolar. Sayı satırı “6 satır, kaynakta sayıldı” der. Kaynağa yazılmamış
     değişiklik varsa “…; kaynağa yazılmamış değişiklikler bu cihazdaki hâliyle” diye ekler.
   - Sayılırken “Kaynak sayıyor…” yazılır. Kaynak bir alanı sayamazsa “Kaynak sayamadı: neden” yazılır.
   - Seçililer, Görünümdekiler ve Tablonun süzgeciyle satırları okumaya devam eder (ADR 9156).
4. **Grafik** tek alan okuyan grafiği kaynağın sayımıyla çizer. Bunlar Sayı'yla Çubuk ve Pasta, Histogram ve
   gruplanmamış Kutu'dur.
   - İki alan okuyan grafik (bir değerin kategoriye göre toplamı, gruplu Kutu, Saçılım, Çizgi) satırları okur.
   - Grafiğin ayarları katmanın alanlarını, satırlar okunmadan listeler.
5. **Basınca seçim.**
   - İstatistik'in değer satırına ya da kaynağın sayımıyla çizilen grafiğin çubuğuna, dilimine, aralığına basmak
     kaynağa o değerleri tutan satırların anahtarlarını sordurur (`value_keys`). Değerler 500'erli parçalarla
     (`In`), boş değer `IsNull` ile sorulur; en çok 10 000 satır seçilir.
   - Anahtarlar katmanın tablosunda seçilir: Shift ekler, Ctrl çıkarır. Çizimin satırları kendi yuvalarıyla
     seçilir. Günlük ““acil”: 2 satır seçildi.” der.
   - Satırları okunan yolda da kaynak satırları anahtarlarıyla tabloda seçilir. Önceden bu basış çizimin seçimine
     olmayan yuvalar yazıyordu ve hiçbir şey seçilmiyordu; kusur düzeldi.
6. **Satırları okunamayanlar.** Kaynağın sayımı tablonun her satırını sayar. Satırları okuyan yol, geometrisi projenin
   sistemine çevrilemeyen satırı hiçbir şey söylemeden dışarıda bırakıyordu. Bu ayrım ancak böyle bir satırda görülür.

## §3 Kanıt

- **Uygulama** (`tests/all/field_counts.rs`, raporun altı bakım noktası):
  - kaynağın sayımı her alanda okunan satırlarla değer değer aynı;
  - durum acil, planlı, tamam ikişer; boş bedel en sonda;
  - oturumun tuttuğu 3. ve 6. satır dışarıda;
  - olmayan alan söyleniyor;
  - anahtarlar tablonun sırasıyla geliyor; en çok sınırı ve “daha fazlası var” işleniyor; 1 200 değer parçalarla
    soruluyor.
- **Masaüstü** (`gis/data_counts_tests.rs`):
  - **Tümü:** bedel 5 dolu, 1 boş, 4 farklı değer, en az 0, en çok 250, ortalama 100, toplam 500. Durum 6 dolu,
    3 değer; her biri %33,3. Satırları okuyan yol hiç çağrılmadı.
  - **Okunan satırlarla aynı:** 240 satır, üç ölçekte ondalık, tekrar eden ve boş değerler, Türkçe sözcükler (İnceleme,
    ılık, çalışma, Işık, Ödendi). Her alanın özeti ve ayrıntısı, Std. sapma dahil, satırları okuyan yolunkiyle aynı.
  - **Oturum:** B03'ün bedeli 300'e çıktı, B06 silindi, yeni bir satır (40, acil) eklendi. Bedel 0 ile 300 arası,
    toplam 490; acil 3, planlı 2, tamam 1. Sayı satırı yazılmamış değişiklikleri söylüyor; sonuç okunan satırlarla
    aynı.
  - **Seçim:** acil'e basınca 1 ve 2 seçiliyor. Shift ile tamam eklenince 1, 2, 5, 6 oluyor. Ctrl ile acil
    çıkarılınca 5, 6 kalıyor. Boş bedel 4'ü seçiyor.
  - **Grafik:** durum çubukları 2, 2, 2. Acil'in çubuğu 1 ve 2'yi seçiyor. Bedelin histogramı ve notu okunan
    satırlarınkiyle aynı. Kategoriye göre toplam satırları okuyor.
  - **Geri dönüş:** sayılamayan alan nedenini söylüyor. Çok değerli alan satırları okutup eskisi gibi sayıyor.
- **Ölçüm.** İstatistik'in yedi alanının hepsi sayılıp özetlendi; her yol kendi sürecinde, en yüksek bellek
  `/usr/bin/time -l` ile (sınama: `measure_a_million_parcels`).
  - Veri: bir milyon parsel (küçük dörtgenler, 289 MB GeoPackage). Alanlar: benzersiz kod, ada (5 000 değer), parsel
    (200), mahalle (8), kullanım (6), alan (900 000 farklı ondalık), bedel (%10'u boş, 1 800 değer).
  - Ortam: hata ayıklama derlemesi (iyileştirmesiz); Apple M5, 10 çekirdek, 32 GB, macOS 27.2.1; `ee2add67d` üstüne bu
    değişiklik; SQLite rusqlite 0.43'ün içindeki.

  | Yol | Süre | En yüksek bellek |
  |---|---|---|
  | Kaynağın sayımı (`GROUP BY`) | 5 713 ms | 736 MB |
  | Satırları okuyarak (ADR 9130) | 31 931 ms | 5 536 MB |

  - Yedi alanın özet satırları iki yolda harfi harfine aynı (900 000 farklı ondalığın kesin toplamı
    `4599936999.99999999998873` dahil).
  - Sürüm derlemesinde ölçülmedi.
- **PostgreSQL** (23 Kasım 2026, `KENTOS_DATA_PG` ile): geçici bir PostGIS'te (PostgreSQL 18.4, PostGIS 3.6). Görüntü
  amd64'tür ve bu Mac'te öykünmeyle çalışır.
  - **Uygulama** (`a_postgis_table_is_counted_by_the_server_alike`): aynı denetimler sunucunun sayımıyla geçiyor.
    - değer değer okunan satırlarla aynı, boş değer en sonda;
    - oturumun satırları dışarıda, olmayan alan söyleniyor;
    - anahtarlar tablonun sırasıyla geliyor, 1 200 değer parçalarla soruluyor.
  - **Masaüstü** (`a_postgis_layer_is_counted_by_the_server_alike`): PostGIS katmanında da aynı denetimler geçiyor.
    - Tümü'nün sayımı ve basınca seçim;
    - Grafik;
    - 240 satırlık ondalık ve Türkçe verinin okunan satırlarla aynılığı.
  - **Ölçüm:** aynı bir milyon parsel `public.kentos_olcum_parsel` tablosunda, ortam yukarıdaki gibi.

    | Yol | Süre | En yüksek bellek |
    |---|---|---|
    | Sunucunun sayımı (`GROUP BY`) | 6 572 ms | 791 MB |
    | Satırları okuyarak (ADR 9130) | 37 134 ms | 5 536 MB |

    Özet satırları GeoPackage'takilerle ve birbirleriyle harfi harfine aynı.
  - **Aynı koşuda öbür PostgreSQL sınamaları:**
    - sürücünün uygunluk takımı işlem kaydı denetimleriyle geçti (ADR 9175);
    - topoloji ve sınıf sayımı durumları geçti (ADR 9124, 9110);
    - ADR 9136'nın PostGIS histogram sınaması ilk kez koştu. Tablo sağlayıcısını kaydetmiyordu ve şemayı iki kez
      ekliyordu; ikisi düzeldi, sınama geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9181/01-istatistik-{light,dark}.png` (durum seçili, değerleri ve sayı satırı)
  ve `02-grafik-{light,dark}.png` (durum çubukları, acil seçili).

## §4 Kalan

- Seçililer ve Görünümdekiler'in de kaynağa sorulması (anahtarlar ve görünümün kutusuyla).
- Grafikte tablonun seçili satırlarının işaretlenmesi. Bugün yalnız çizimin seçili nesneleri işaretlenir; basınca
  seçilen kaynak satırları tabloda ve haritada görünür, çubukta görünmez.
- İki alan okuyan grafiklerin birlikte gruplanması (`GROUP BY a, b`). Sürücü sözleşmesine yeni bir işlev ister;
  PostgreSQL'de sınanmadan eklenmemeli.
- Sürüm derlemesinde ölçüm, gerçek pencerede deneme.
- Web (TODOS.md §0.2).
