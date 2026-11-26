# ADR 9321: Kaydet, tablo bekleyen bütün katmanları tek pencerede taşır; taşınan katman kimliğini korur

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  15, P1). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 9107](9107-project-in-a-source.md) §8 (eski katman kaynağa taşınır; bu ADR değiştirir),
  [ADR 9196](9196-storage-by-project-kind.md) §2 (CBS'nin verisi tablolarındadır),
  [ADR 9114](9114-text-tables.md) §2 (yazıların tablosu).

## Bağlam

- **Kusur:** yeni CBS projesinde veri deposu seçilmeden iki GeoPackage tablosu İçe aktar kipinde alındı. Ardından Tampon,
  En kısa yol ve Hizmet alanı üretildi, Ctrl/Cmd+S ile dosya hedefi seçildi.
  - Kayıt engellendi ve yalnız ilk katmanın (qa_yollar) “Veri kaynağına taşı” penceresi açıldı. Öbür katmanlar için bir
    yol gösterilmedi.
  - Önceki uyarı yalnız Komut geçmişinde ve kesilmiş durum satırında görünüyordu.
  - Proje diskten yeniden açılarak doğrulanamadı.
- **Kod:**
  - Kayıt denetimi (`saving.rs`) bekleyen katmanların yalnız ilkinin penceresini açıyordu.
  - İçe almadan sonra veri yeri bilinmiyorsa (`data_into_tables`) yalnız uyarı yazıyordu.
- **Gizli kusur:** `cad.layers.migrate` katmanın yerine yeni kimlikli bir veri katmanı koyup eskisini siliyordu.
  - Katmanı kimliğiyle anan her şey kopuyordu: ağın kenar ve düğüm katmanları, ilişkiler, formlar, birleştirmeler,
    görünümler.
  - Katmanın süzgeci ve zaman ayarı da yeni katmana geçmiyordu.
  - İncelemenin kabulü yeniden açılan projede ağ katmanı başvurularını istiyor.

## Araştırma

- **QGIS:** geçici (bellek) katmanlar diske yazılmaz ve program kapanınca kaybolur.
  - Katmanın menüsündeki “kalıcı yap”, katmanı seçilen biçime (GeoPackage dahil) yazar. Kaydedilen dosya Katmanlar
    panelinde geçici katmanın yerini alır.
    ([katman oluşturma](https://docs.qgis.org/3.10/en/docs/user_manual/managing_data_source/create_layers.html))
  - Bütün geçici katmanları tek seferde kaydetmek için ayrı bir eklenti var
    ([Save Temp Layers](https://plugins.qgis.org/plugins/save_temp_layers/)). Toplu yol ürünün kendisinde yok.
- **KentOS kararı:**
  - Kayıt, tablo bekleyen bütün katmanları tek listede gösterir. Yer bir kez seçilir, hepsi taşınır ve kayıt sürer.
  - Taşınan katman yerinde ve kimliğiyle kalır; böylece hiçbir başvuru yeniden yazılmaz.

## Karar

### 1. Taşınan katman kimliğini korur (`cad.layers.migrate`)

- Nesneler tabloya yazılır ve çizimden çıkar. Katmanın kendisi aynı kimlikle tabloyu okur: veri kaynağı, tablonun
  alanları ve geometri türü katmana verilir.
- Adı, stili (işleyici, etiketler, renk), süzgeci, zamanı, etkinliği ve onu anan her şey aynen kalır: ağın kenarları ve
  düğümleri, ilişki, form, birleştirme, görünüm.
- **Yazılar:**
  - Yalnız yazı tutan katman, yazı tablosunu yerinde okur.
  - Başka nesnelerle birlikte duran yazılar, üstüne eklenen “(yazılar)” katmanına gider (ADR 9114 §2; bu yeni bir
    katmandır).
- **Geri alma:** tek adımdır, adı “Katmanı kaynağa taşı”dır. Geri alınınca katmanın çizim hâli ve nesneleri döner, tablo
  kaynakta kalır.
- **Sözleşme:** `LayersMigrated.layer` katmanın kendi kimliğidir. Retler ve sıraları ADR 9107 §8'deki gibidir.

### 2. Kaydet tek pencere açar

CBS projesinin dosyası katman nesnesi tutacaksa (`layer_kind::held_in_file`) kayıt bekler. Açılan tek pencere şunları
gösterir:

- **Yer, bir kez:** GeoPackage dosyası ya da PostgreSQL bağlantısı ve şeması.
  - Öneri, projenin verisinin yeridir; yoksa kaydedilecek dosyanın yanındaki GeoPackage, o da yoksa olası dosya.
- **Koordinat sistemi:** tabloların projenin sisteminde yazılacağı (ör. “TUREF / TM36 (EPSG:5256)”).
- **Her katman bir satırda:** adı, nesne sayısı, tablosunun tutacağı tür (nokta, çizgi, alan, yazı, karışık) ve
  düzenlenebilir tablo adı.
- **Taşı ve kaydet:**
  - Önce her satırın taşınması yazmadan sorulur (`plan`).
  - Kaynakta var olan tablo ya da listede iki katmanın paylaştığı ad, satırında söylenir. Yerine boş bir ad konur
    (“… tablosu kaynakta var; “noktalar_2” önerildi.”).
  - Yazı tablosunun adı (`<tablo>_yazilar`) da listede alınmış sayılır.
  - Bir satır takılırsa hiçbir şey yazılmaz.
- **Sonra katmanlar tek tek taşınır.** Her katman kendi geri alma adımıdır.
  - Taşınan katman listeden çıkar.
  - Bir hata orada durur ve satırında söylenir. Yeniden Taşı yalnız kalanları taşır; hiçbir tablo iki kez yapılmaz.
- **Bütün katmanlar taşınınca** bekleyen kayıt kendi hedefine sürer.

### 3. Verinin yeri bir kez sorulur

- Projenin verisinin yeri yokken (kayıt yeri, veri katmanı ya da dosya yok) bir içe alma yeni katman getirirse aynı
  pencere açılır (“Verinin yeri”). Bu oturumda bir kez sorulur. İşlem ve araç çıktıları için bakınız Ek.
  - Yer seçilip taşınınca projenin verisinin yeri olur. Sonraki içe almalar oraya kendiliğinden yazılır.
  - Vazgeç'te yeniden sorulmaz; kalanları kayıt sorar.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Kaydet'in tek penceresi, verinin yeri | yok: projenin tamamı dosyadadır (CLAUDE.md §25 kural 5) | yok: aynı gerekçe | yok: aynı gerekçe | var |
| Taşınan katmanın kimliğini koruması | yok: Veri kaynağına taşı CBS'nindir | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.layers.migrate` her katman için aynı komuttur; kimliği koruması ve retleri aynıdır (`test_command_cases.py` 29 ortak durumu koşar). Tek pencere masaüstünün sunumudur; betik katmanları sırayla taşır. |
| MCP | var | Aynı komut, aynı ortak durumlar. |
| Yapay zekâ ajanı | var | Aynı komut. |
| Eklenti | var | `document.write` izniyle aynı komut. |

## Ek (27 Kasım 2026): işlem ve araç çıktısı işi kesmez

- **Kusur:** ilk uygulamada pencere, verinin yeri yokken katman getiren her mesajdan sonra açılıyordu. Araç ya da pencere
  daha sürerken modal pencere araya giriyordu:
  - En kısa yol ve Hizmet alanı Enter'la Rota katmanını yazınca açılıyor; sonraki Esc, Ctrl+Z, S ve E tuşları araca değil
    pencereye gidiyordu.
  - Veri karşılaştır'ın Farkları çizime yaz'ı, Senaryo oluştur, Kopyasını oluştur ve şablonla çizim de onu açıyordu.
  - Etkileşim izlerinden onu her varyantta bu yüzden kırmızıydı (`cargo test -p kentos-cad --test programs`):
    data-compare, layer-merge, layer-move, network-route, network-service-area, scenarios ve dört şablon izi.
- **Karar:** pencere yalnız içe almada açılır; incelemenin açık isteği budur (“ilk içe almada veri deposunu bir kez
  seçtir”).
  - İşlem ve araç çıktıları çizimde bekler. Kalıcı olmaları Kaydet'tedir: Kaydet'in tek penceresi (§2) onları da listeler.
    İncelemenin “ilk kalıcı analiz çıktısı” sözü böyle karşılanır.
  - Bu, geçici katmanı kayıtta kalıcı yapma geleneğine de uyar (bakınız Araştırma).
- **Doğrulama:**
  - `cargo test -p kentos-cad`: 37 geçti; bütün izler her varyantta geçiyor.
  - `cargo test -p kentos-desktop --lib -- processing:: layer_migrate cbs_storage exchange::gis_tests`: 152 geçti.

## Açık

- Analiz çıktılarının (Tampon, En kısa yol, Hizmet alanı) veri yeri bilindiğinde kendiliğinden tabloya yazılması.
  Bugün yalnız içe almalar yazılır; çıktılar Kaydet'in penceresinde listelenir.
- Gerçek pencerede fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Ortak durumlar:** `scripts/fixtures/layer_migrate_command_cases.py` 29 durum yazıyor (`--check` yeşil).
  - Yeni: geri alma katmanı ve nesneleri döndürüyor; ağ, kenar katmanını anmayı sürdürüyor (`networks` beklentisi).
    Beklentinin gerçekten denetlendiği, bilerek bozulup kırmızı görülerek sınandı.
  - Rust: `KENTOS_CASES_ONLY=cad.layers.migrate cargo test -p kentos-native-application --test all --
    fixtures::every_command_case`.
  - Python: `test_command_cases.py`, yerel modül yeniden derlendikten sonra.
- **Masaüstü:** `cargo test -p kentos-desktop --lib -- layer_migrate cbs_storage exchange::gis_tests
  reproject_window`.
  - `layer_migrate_many_tests` (3 sınama):
    - Kaydet bütün bekleyen katmanları ağaç sırasıyla tek pencerede listeliyor.
    - Taşı ve kaydet GeoPackage'a 3, 2 ve 1 satır yazıp projeyi yazıyor.
    - Dosyadan yeniden açılan projede ağ `yollar`'ı anıyor; katman tablosunu 5256 sisteminde okuyor (2 satır), proje
      dosyasında nesne yok.
    - Var olan tablo ve paylaşılan ad boş adlarla söyleniyor, hiçbir şey yazılmıyor. İkinci Taşı dört tablo yapıyor;
      hiçbiri iki kez yapılmıyor.
    - Yeri olmayan projede ilk gelen katmanlar yeri bir kez soruyor; yer seçilince projenin yeri oluyor.
  - `layer_migrate_tests`: katman kimliğiyle tabloyu okuyor; tek geri almayla çizim hâline dönüyor.
- **Resim** (`.run/gui-evidence/kullanim-9321/`, açık ve koyu tema, büyütülerek bakıldı):
  - `01-kaydet-tek-pencere`: üç katman, yer, sistem, Taşı ve kaydet;
  - `02-cakisma-onerileri`: var olan tablo ve paylaşılan ad, önerilen boş adlarla.
- **Koşulmayan:** ortada bir giriş/çıkış hatasıyla yarıda kalan taşıma sınamada üretilemedi (adımlar önce yazmadan
  denetleniyor). Taşınan katmanın listeden çıkması ve her katmanın ayrı adım olması koddadır.
