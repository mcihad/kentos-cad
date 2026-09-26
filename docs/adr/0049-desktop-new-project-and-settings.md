# ADR 0049: Masaüstünde yeni proje ve proje ayarları

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.4, §5, §4.8; ADR 0020 (masaüstü belgesi), 0023 (tipli ayarlar), 0048 (masaüstünde dosya alışverişi)
- **Sahibin yönü (26 Eylül):** masaüstü web ile aynı düzeye getirilir.

## Bağlam

- Web'de Dosya → Yeni proje, standart katman ağacıyla boş bir çizim açar: çalışma modu, koordinat sistemi, pafta ölçeği sorulur (`ui/settings/NewProjectDialog.ts`, `model/newProject.ts`, `model/standardLayers.ts`).
- Web'de Dosya → Proje ayarları projeyle saklanan ayarları düzenler: ad, ölçek, çalışma modu, çizim yazı tipi, koordinat sistemi (atama; dönüşüm yok), birimler ve basamaklar (`ui/settings/ProjectSettingsDialog.ts`).
- Masaüstünde ikisi de yoktu: masaüstü yalnız bir dosya açarak çalışmaya başlayabiliyordu.
- Masaüstü belgesi (`kentos-domain`) adı ve proje ayarlarını değiştiremiyordu.

## Karar

### Yeni projenin çizimi, ortak bir fixture'la

- Web, kendi kodunun birkaç seçim için kurduğu çizimi kaydeder (`fixtures/project/v1/new-project.json`).
  - Kaydeden: `scripts/fixtures/record-new-project.test.ts`, `GOLDEN_WRITE=1` ile.
  - Durumlar: beş koordinat sistemi (TUREF TM36 ve TM30, WGS 84 coğrafi, Pseudo-Mercator, WGS 84 UTM 36N), dört ölçek, üç çalışma modu, iki yazı tipi, boş ve boşluklu ad.
- `src/model/newProject.test.ts`, kod ile dosya ayrıştığında düşer. Böylece yalnız bilerek yapılan bir değişiklik yeniden kaydedilir.
- Masaüstü aynı çizimi kendisi kurar (`apps/desktop/src/project/content.rs`):
  - standart katman ağacı ve stilleri;
  - varsayılan birimler;
  - dilimin çalışma alanının ortası (`work_area_centre`; coğrafi sistemde 35° D 39° K, Pseudo-Mercator'da yuvarlanmış metre);
  - pafta çerçevesi etiketinde ölçek.
- Masaüstünün testi, fixture'daki her durum için kurduğu çizimi `DocumentSnapshotV1` olarak alan alan karşılaştırır. Katman ağacının verisi masaüstünde bir kez daha yazılıdır; bu fixture, iki kopyanın ayrışmasını yakalar.
- Yeni çizim kalıcı kimliksiz ve kaynak kaydı olmadan açılır, web'deki gibi. Dosyası yoktur: ilk Kaydet yerini sorar.
- Boş çizim, web'deki gibi anchor'ın çevresinde pafta ölçeğinde bir sayfa (50 × 37,5 cm) gösterir.

### Adın ve ayarların değişmesi, ortak fixture'la

- `Document::set_name` ve `Document::set_settings`, web'in `name.set` ve `settings.assign` anlamıyla eklendi:
  - değer değişirse düzenlemedir (belge kaydedilmemiş olur);
  - geri alma adımı değildir;
  - aynı değer düzenleme değildir.
- Koordinat sistemi atamak koordinatları değiştirmez (CLAUDE.md §5).
- Ortak belge fixture'larına `setName` ve `setSettings` işlemleri, `name` ve `settings` beklentileri eklendi. Yeni dosya `fixtures/document-ops/v1/project.json` iki senaryo içerir; web ve masaüstü koşucuları ikisini de geçer.

### Pencereler (`apps/desktop/src/project/`)

- **Yeni proje:**
  - adı (ilk kayıtta dosya adı olarak önerilir) ve pafta ölçeği (1:500 … 1:25.000);
  - çalışma modu kartları: seçilebilen üçü; "Yakında" diye 3D Plan ve Afet Analizi, soluk ve seçilemez;
  - koordinat sistemi;
  - açılacak çizimin özeti;
  - ekrandaki çizimin durumu.
  - Oluştur'da kaydedilmemiş iş pencerenin üstünde sorulur; Vazgeç pencereye döndürür. Bulut projesi önce bırakılır (gönderilmemiş iş taslağa yazılır).
  - Varsayılanlar tipli ayarlardan gelir (`newProjects.srid`, `newProjects.workspace`, `newProjects.drawingFont`). Bu üç ayar artık masaüstünde de vardır: Uygulama ayarları → Yeni projeler (koordinat sistemi listesi, çalışma modu, çizim yazı tipi).
- **Proje ayarları:** üç bölüm, web'in sözleriyle, taslak üzerinde çalışır.
  - Genel: ad, ölçek, çalışma modu, çizim yazı tipi, özet.
  - Koordinat sistemi: atama; değişince amber kenar ve “Koordinatlar dönüştürülmez” uyarısı; datum farkında dönüşümün yokluğu söylenir.
  - Birimler ve hassasiyet: uzunluk ve alan basamağı, alan birimi, açı birimi, önizleme kartı.
  - Kaydet adı ve ayarları atar, Vazgeç hiçbir şey değiştirmez.
- **Koordinat sistemi seçici** (`apps/desktop/src/crs.rs`, web'in `crsPicker`'ı):
  - seçilen sistemin kartı;
  - datuma göre gruplu, aranabilir liste (SRID'nin başı, ad ya da kapsam; tam bir SRID yazmak onu seçer, bilinmeyeni söylenir);
  - parametre kartı;
  - notlar.

  Liste web'in kaydıdır (`fixtures/crs/v1/registry.json`). DXF ve koordinat listesi içe aktarımının sorusu da bu modüldedir (ADR 0048).
- **Çalışma modları:** masaüstü kataloğu, modların adını, açıklamasını ve maddelerini gömülü web envanterinden okur. Envanter bu dilimde açıklama ve maddeleri de yazar. Sıra sözleşmenin sırasıdır: Hibrit, CAD, CBS, 3D Plan, Afet Analizi.

## Bu dilimde olmayanlar

- **Başlangıç ekranı ve son dosyalar (`file.start`):** masaüstünde son dosyaların listesi yoktur. Sonraki dilimdir; yeni bir kalıcı yer gerekir (CLAUDE.md §2'ye yazılacak).
- **Çalışma modunun şeridi süzmesi (`workspace.*`):** masaüstü modu saklar ve gösterir. Şeridi, menüleri ve araçları moda göre süzmesi sonraki dilimdir; kurallar envanterde hazırdır (`hide`).

## Doğrulama (26 Eylül 2026, Linux)

- `kentos-domain`: ortak fixture'ların senaryoları, iki yenisiyle, web ve masaüstünde geçer.
- Web: `newProject.test.ts` fixture'la kodu karşılaştırır; `documentOps.test.ts` yeni senaryoları geçer.
- Masaüstü:
  - `project::content`: fixture'daki beş çizim alan alan aynıdır; bilinmeyen sistem reddedilir; sayfa boyu doğrudur.
  - `project`, dört akış (ve elle çalıştırılan `screens`: pencerelerin koyu ve açık görüntüleri):
    - yeni proje temiz çizimin yerini alır, katmanları ve ayarlarıyla;
    - kaydedilmemiş iş sorulur, Vazgeç pencereye döner, onay yeni projeyi açar;
    - boş ad reddedilir;
    - proje ayarları taslakta çalışır, Vazgeç bir şey değiştirmez, Kaydet atar.
  - `crs`: arama web'in kuralıyla bulur, Türkçe basamak gruplaması.
