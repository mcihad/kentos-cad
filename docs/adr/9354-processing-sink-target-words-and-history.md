# ADR 9354: İşlem sonucunun dosya ve veritabanı hedefi: önce hedef, pencerenin dilinde çakışma, eylemler ve işlem geçmişi

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, kullanım turları 7 ve 8
  (`gis-eksik-27-11-2026.md`, T30 P2, T31 P2). Sahibin sözü: “Gis eksik 27 11 dosyasındaki yeni maddeleri tamamla”.
- **İlgili:**
  - [ADR 0592](0592-processing-from-sources-to-databases.md) §3 (Sonucun yeri), [ADR 0593](0593-batch-and-model-sinks-and-run-records.md) (kayıt);
  - [ADR 0529](0529-processing-batch-and-run-record.md) (çalıştırmanın kaydı), [ADR 9179](9179-buffer-cases-and-the-run-in-words.md)
    (geçmişte çalıştırmanın sözü);
  - [ADR 9353](9353-analysis-tools-read-data-layer-rows.md) (işlem araçları veri satırlarını okur).

## Bağlam

- **T30:**
  - GeoPackage hedefi seçilince de Çıktı katmanı ve projenin “Bu adda katman yoksa oluşturulur” sözü ekranda
    kalıyordu.
  - Aynı dosyaya yeniden yazınca uyarı kanalın sözünü söylüyordu: “üzerine yazmak için overwrite: true verin”.
    Pencerede o iş Varsa üzerine yaz anahtarıdır.
  - Başarıdan sonra çıktıyı projeye ekleyen ya da konumunu açan eylem yoktu.
- **T31:**
  - İşlem geçmişinde yalnız çizime yazan çalıştırmalar vardı; dosyaya yazanlar ve reddedilenler yoktu.
  - Geri alınan çalıştırmanın kaydı başarı olarak kalıyor, sonuç katmanının kaldırıldığını söylemiyordu.
  - Yeniden aç, girdinin artık bulunmadığını söylemeden genel bir uyarıya düşüyordu.

## Araştırma

- **QGIS, İşleme:** araç penceresinde çıktı satırı geçici katman, dosya ya da veritabanı tablosu seçer. Dosyada yol,
  veritabanında bağlantı, şema ve tablo sorulur. Çalıştırma bitince çıktı isteğe göre projeye katman olarak eklenir
  (“Open output file after running algorithm”)
  ([Processing toolbox: output](https://docs.qgis.org/latest/en/docs/user_manual/processing/toolbox.html#the-algorithm-dialog)).
- **QGIS, İşleme geçmişi:** her çalıştırma, başarısı ve hatası dahil, geçmişte durur. Kaydı çift tıklamak aracı aynı
  değerlerle açar ([History](https://docs.qgis.org/latest/en/docs/user_manual/processing/history.html)).
- **KentOS kararı:**
  - Hedef çıktının adından önce seçilir. Dosyada ve veritabanında çıktı katmanı tablonun adıdır.
  - Ret pencerenin dilinde söylenir: açılacak anahtar, seçilecek seçenek. Kanalların sözü değişmez.
  - Her çalıştırma, yazılan ya da reddedilen, hedefiyle geçmişe girer.

## Karar

### 1. Önce hedef (T30)

- **Sıra:** pencerede Girdi ve Ayarlar'dan sonra Sonucun yeri, sonra Çıktı gelir.
- **Çizime:** çıktı katmanı bugünkü gibidir (Yeni katman ya da var olan katman; “Bu adda katman yoksa oluşturulur.”).
- **GeoPackage dosyasına, PostGIS veritabanına:**
  - Çıktı katmanının satırı “Tablo adı”dır.
  - Altında “GeoPackage'da bu adla tablo yazılır; çizim değişmez.” ya da “Veritabanında …” yazar.
  - Çizimin katmanları sunulmaz; önce var olan bir katman seçildiyse tablonun adı o katmanın adıdır.
- **Dosya ve bağlantı:** yol, Tablolar kipi ve Varsa üzerine yaz (dosyada); adres ve şema (veritabanında) Sonucun
  yeri'ndedir.

### 2. Çakışma pencerenin dilinde (T30)

Sağlayıcının kodu dosyanın mı tablonun mu var olduğunu söyler:

| Kod | Pencerenin sözü |
|---|---|
| `FILE_EXISTS` | “Bu GeoPackage zaten var: sonuc.gpkg. Başka dosya seçin ya da Varsa üzerine yaz'ı açın. Dosyaya dokunulmadı.” |
| `TABLE_EXISTS` (dosya) | ““Merkezler” tablosu sonuc.gpkg dosyasında var. Tablolar'da Tabloyu değiştir'i seçin ya da başka bir tablo adı yazın. Dosyaya dokunulmadı.” |
| `TABLE_EXISTS` (veritabanı) | ““…” tablosu veritabanında var. Tablolar'da Tabloyu değiştir'i seçin, başka bir tablo adı ya da şema yazın. Veritabanına dokunulmadı.” |
| `FILE_NOT_FOUND` | “… yok. Tablo eklenecek GeoPackage'ı seçin ya da Tablolar'da Yeni'yi seçin.” |
| `PARTIAL_SCOPE` (uyarı) | “… araç o parçayla çalıştı. Bütün tablo için girdinin altındaki Bütün tablo (kaynaktan oku) anahtarını açın.” |

- **Kanallar:** Python, MCP ve ajan kanalın sözünü almaya devam eder (`overwrite: true`, `sink.mode`); orada doğru
  olan odur.
- **Dosya korunur:** üzerine yazma kapalıyken ilk dosyaya dokunulmaz (sınamada değişme zamanı aynı).

### 3. Başarıdan sonra (T30)

Formun başında “Sonuç dosyaya yazıldı” (ya da “veritabanına”) kartı durur:

- dosyanın yolu ya da veritabanının sunucusu;
- tabloları ve satır sayıları;
- **Katman olarak ekle:** yazılan tablolar veri katmanı olarak eklenir (Kaynaklar'ın Bağla yolu, `add_tables`).
  Eklenince düğme “Katman olarak eklendi” olur.
- **Dosya konumunu aç** (yalnız dosyada): dosyanın klasörü sistemin dosya yöneticisinde açılır.
- Kart, hedef değişince, yeni çalıştırmada ya da retle kalkar.

### 4. İşlem geçmişi (T31)

- **Dosyaya ve veritabanına yazan çalıştırma** geçmişe girer:
  - girdileri çözüldükleri gibi (“Nesneler: 4 kapalı alan; “bina” katmanında”);
  - “Sonucun yeri: sonuc.gpkg (GeoPackage dosyası)” ya da “…, şema … (PostGIS veritabanı)”;
  - yazılan tablolar ve satırları, okunan ve yazılan kayıt sayısı, süresi.
- **Reddedilen çalıştırma** “Başarısız” olarak kısa nedeniyle girer (“Yazılmadı: Bu GeoPackage zaten var …”);
  durdurulan “Durduruldu”.
- **Durum çipi:** her kartta Başarılı, Başarısız, Durduruldu ya da Geri alındı.
- **Geri alınan:** çalıştırmanın eklediği nesneler çizimde yoksa kart “Geri alındı”dır. Yeni katmanı da gittiyse adıyla
  söylenir: “Geri alındı: eklediği nesneler çizimde yok, “Çizimdeki merkezler” katmanı kaldırıldı.” Bir kısmı yoksa
  sayısı söylenir.
- **Yeniden aç:**
  - Dosyaya ya da veritabanına yazan çalıştırmanın hedefi de geri gelir (yol, Tablolar kipi, adres, şema).
  - Girdi artık o değilse formun başında uyarı durur. Katmanı yoksa adı söylenir; modelin önceki adımının nesneleri
    yoksa sayısı. Girdi şimdi daha az nesne okuyorsa: “önceki çalıştırma 4 nesne okudu, girdi şimdi 3 nesne; silinen
    nesneler ya da kaynağa yazılmadan atılan geçici kayıtlar artık girdide değil”.
  - Uyarı ilk değişiklikte ya da çalıştırmada kalkar.
- **Bellek:** geçmiş oturumluktur (önceki gibi); hedefler kaydın sırasıyla oturumda tutulur.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Önce hedef, tablo adı, pencerenin dilinde çakışma, kart | yok: Mimari'de İşlemler yoktur | var: İşlemler ve Sonucun yeri Harita'da da vardır | var: aynı | var |
| İşlem geçmişinde dosya/veritabanı ve ret kayıtları, geri alındı, yeniden aç uyarısı | yok: aynı gerekçe | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.processing.run` aynı hedefi (`sink`) yazar ve aynı kodla reddeder (`FILE_EXISTS`, `TABLE_EXISTS`). Kanalın sözü kanalın alanlarını anar; pencereninki yalnız arayüzdedir. Çalıştırmanın kaydı yanıttaki `record`'dur (ADR 0529). |
| MCP | var | Aynı gerekçe. |
| Yapay zekâ ajanı | var | Aynı gerekçe. |
| Eklenti | yok | İşlem aracını çalıştırmak eklentiye kapalıdır; eklenti kendi aracını işlem olarak yazar (ADR 9310). |

## Açık

- **Kalıcı geçmiş:** işlem geçmişi ve hedefler oturumluktur; proje ya da makine ile saklanmaz.
- **Veritabanında tablo listesi:** tablo adı elle yazılır; şemadaki tablolardan seçme yok.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `processing::sink_tests::a_file_there_is_said_in_the_windows_words_and_each_run_goes_into_the_history` geçti:
    - `sonuc.gpkg`'ye `Merkezler` (4 satır) yazıldı, kart dosya ve tabloyu söylüyor;
    - ikinci çalıştırma (üzerine yazma kapalı) pencerenin sözüyle reddedildi, dosyanın değişme zamanı aynı;
    - Tablo ekle kipinde tablo çakışması dosyanınkinden ayrı söylendi;
    - Katman olarak ekle `Merkezler`'i veri katmanı yaptı; Dosya konumunu aç dosyanın klasörünü açtı;
    - çizime yazan çalıştırmayla geçmiş yeniden eskiye Başarılı, Başarılı, Başarısız, Başarısız, Başarılı;
    - Geri al'dan sonra kart “Çizimdeki merkezler” katmanının kaldırıldığını söylüyor;
    - bir kare silinip ilk çalıştırma yeniden açılınca uyarı “4 nesne okudu, girdi şimdi 3”; hedef (dosya, Yeni) geri
      geldi.
  - `processing::sink_tests` (4 geçti; ilk sınama artık Çalıştır'ın kendi iş parçacığını sürüyor, ikinci çalıştırmayla
    yarışmıyor).
  - `-- processing:: module_tests ribbon_tests history::text catalog settings`: 303 geçti. Kırmızı tek sınama bilinen
    bağımlılık tabanıdır (render birleşmesinin bağları).
- **Resim** (`.run/gui-evidence/kullanim-9354/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-dosya-hedefi`: Sonucun yeri Çıktı'dan önce; Çıktı'da “Tablo adı”;
  - `02-yazildi`: “Sonuç dosyaya yazıldı” kartı, Katman olarak ekle, Dosya konumunu aç;
  - `03-cakisma`: “Bu GeoPackage zaten var: sonuc.gpkg …”;
  - `04-gecmis`: Geri alındı ve Başarısız çipleri, kaldırılan katmanın adı;
  - `05-yeniden-ac`: “Önceki çalıştırmanın girdisi değişti …”.
