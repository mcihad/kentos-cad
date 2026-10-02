# ADR 0155: Web'de yalnız şerit

- **Durum:** kabul edildi (2026-10-02). Sahibin kararı: “Bence web toolbar görünümünü iptal et sadece ribbon kalsın iki taraf daha uyumlu olur.”
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** ADR 0017 (masaüstü kabuğu), ADR 0051 (pencereye sığan şerit), ADR 0064, 0077 (sekme satırı ve Komut ara), ADR 0115 (kabuk düzeni ve yerleşim), ADR 0117, 0118 (şeridin hızlı erişimi, menüleri, harf ipuçları), ADR 0126 (görünüş ayarları); DESIGN.md §7; `fixtures/shell/v1`.

## Bağlam

Web'in iki kabuğu vardı: klasik (menü çubuğu, araç çubuğu, kayan ya da kenara sabitlenen araç kutusu) ve şerit. Seçim `appearance.shell` ayarındaydı; varsayılan klasikti. Masaüstünde 27 Eylül'den beri yalnız şerit vardır (sahibin kararı); klasik kabuk web'e özgü kalmıştı.

İki kabuk aynı komut ve araçları sunar (`app/menus.ts`, `tools/catalog.ts` tek kaynak), ama iki ayrı arayüzü, kurallarını (`ui/shell/shellPlan.ts`, `fixtures/shell/v1/shell.json`), yerleşim alanlarını, e2e denetimlerini ve resimlerini ayrı ayrı yaşatmak gerekiyordu. Web ve masaüstünün resimleri de bu yüzden farklı görünüyordu.

## Karar

### 1. Tek kabuk

Web'de yalnız şerit vardır, iki platform aynı kabuğu gösterir:

- **Kalkanlar:** menü çubuğu (`ui/menu/MenuBar.ts`), araç çubuğu (`ui/toolbar/Toolbar.ts`), araç kutusu (`ui/toolbox/Toolbox.ts`) ve kuralları (`ui/shell/shellPlan.ts`, `fixtures/shell/v1/shell.json`, `scripts/fixtures/shell_cases.py`); çizim alanının solundaki sabitleme yuvası.
- **Komutlar:** `view.ribbon` (kabuk değiştirme), `view.toolbox`, `view.toolboxDock` kalkar. Şeridin Görünüm sekmesi bunları göstermez.
- **Şeridin yüklenmesi:** şerit artık her açılışta gerekir; ayrı yüklenen parça olmaktan çıkar, başlangıçla gelir (CLAUDE.md §20: başlangıca gereken yük). Yüklenemeyince klasiğe dönme yolu da kalkar.
- **Kalanlar:** `app/menus.ts` şeridin kaynağı olarak kalır. `ui/toolbar/fields.ts`'in alanları (etkin katman, renk, çizgi tipi, kalınlık, ölçek) şeridin Katmanlar ve Özellikler panellerinin alanlarıdır; şeridin klasörüne taşınır (`ui/ribbon/fields.ts`). Yalnız dar araç çubuğunun kullandığı birleşik “Geçerli özellikler” alanı kalkar.

### 2. Ayar ve yerleşim

- **`appearance.shell` emekliye ayrılır:** şemadan çıkar. Saklanmış değeri (web `kentos.settings.v1`, dışa aktarılmış ayar dosyası) okunurken sessizce düşer, bilinmeyen anahtar diye raporlanmaz: emekli anahtarlar listesi (`RETIRED`) iki platformda aynı kuralla okunur. Uygulama ayarları → Görünüm'deki Arayüz düzeni kartları kalkar.
- **Yerleşim alanları:** araç kutusunun alanları (`toolboxVisible`, `toolboxDocked`, `toolboxX`, `toolboxY`, `toolboxColumns`, `toolboxFolded`) ve `ribbonToolbox` yerleşimden çıkar; eski yerleşimde bulunurlarsa okunurken düşer. Masaüstünün `yerlesim.json`'u aynı kuralla okunur (`fixtures/shell/v1/layout.json` iki platformda).

### 3. Sınama ve belgeler

- **e2e:** duman testinin klasik kabuğa bakan denetimleri (araç kutusunun kaydırmasız gösterimi, katlanan grubu, kabuk değiştirme, menü çubuğu ve araç çubuğunun sığması) kalkar ya da şeridin karşılığına çevrilir; `layout.mjs` menü çubuğunun menüleri yerine şeridin menülerini dener; resim sahneleri şeritte çekilir.
- **Envanter:** kalkan komutlar ve pencereler envanterden düşer; `apps/desktop/equivalents.json`'daki “masaüstünde anlamsız” notları da.
- **Belgeler:** DESIGN.md'nin klasik kabuk bölümleri, CLAUDE.md'nin “klasik arayüz yalnız web'dedir” notu.

## Sonuçlar

- Web ve masaüstü aynı kabuğu gösterir; resimler yan yana karşılaştırılabilir, bir arayüzün kuralları ve sınaması tek kez yaşatılır.
- Klasik kabuğa alışmış kullanıcı şeride geçer; şeridin harf ipuçları (Alt), Komut ara (Alt+Q), hızlı erişim çubuğu ve komut satırı aynı komutlara ulaştırır.
- Yeni özellikler yalnız şeride (ve panellere) eklenir.

## Doğrulama

2 Ekim'de, bu dilimin kodunda:

- **Ayar:** emekli anahtarın sessizce düştüğü ortak durum `fixtures/settings/v1`'de; iki çalıştırıcı (web Vitest, masaüstü `cargo test`) geçer. Masaüstünün web ayar dosyasını alması da `appearance.shell`'i söylemeden düşürür.
- **Yerleşim:** eski araç kutusu alanlarıyla yerleşimin okunduğu ortak durum `fixtures/shell/v1/layout.json`'da. Özellikler panelinin listeleri (renkler, çizgi tipleri, kalınlıklar, ölçekler) ve menü satırları `ribbon.json`'a taşındı; iki platform aynı değerleri sunar.
- **Arayüz:**
  - Duman testi 203 denetimle geçer.
  - `e2e:layout`'un 268 görünümü (1100×650 ve 1440×900, iki tema) geçer. Özellikler öğesi 1100 genişlikte katlanan paneli açarak dener.
  - Görsel kaynaklar 25 Eylül'den beri kaydedilmemişti. Farklar okundu: Açıklama'nın yeni araçları, Katmanlar'ın yazılı düğmeleri, Seçim'in bölünmüş düğmesi, büyük yazıda bir piksellik boy. Kaynaklar yeniden kaydedildi; sonraki karşılaştırma 19/19 aynı.
- **Envanter:** komutlar 202, ayarlar 60, pencereler 69. Araç kutusu bayrağı kalkınca da arayüzde yeri olmayan komut çıkmaz: her araç şeritte.
- **Masaüstü:** `pnpm rust:test:desktop` geçer. Yerleşim resimleri (`layout_tests::screens`) web'in kabuk resimleriyle yan yana karşılaştırıldı.
- **Kalan fark:** 1440 genişlikte web Özellikler alanlarını daraltır ve Katmanlar düğmelerini yazılı bırakır; masaüstü Katmanlar'ı simgeye indirir ve Özellikler'i geniş bırakır. Fark bu karardan önce de vardı; nedeni (iki platformun panel küçültme sırası ya da ölçülen genişlikler) araştırılmadı.
