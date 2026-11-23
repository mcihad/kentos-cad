# ADR 9178: GeoPackage teslimi yarıda kesilince dosya bütün kalır; öbür tabloları korumanın yolu söylenir

- **Durum:** kabul edildi (2026-11-23); adım VY-77. Plan: [plan/veri.md](../../plan/veri.md) VY-77.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”; 23 Kasım: “küçük kalanlarla”.
  Değerlendirmenin 6. maddesinin kalanı ([ADR 9168](9168-writing-over-a-geopackage-asks.md) §4):
  - “Var olan dosyada yalnız hedef tabloların üstüne yazmak ya da tabloya eklemek.”
  - “Yarıda kesilen dışa aktarmanın sınaması.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** değişmedi; `file.export.gpkg`'nin
  girdisi aynı.

## §1 Bağlam ve araştırma

- **Teslim nasıl yazılıyor.** `file.export.gpkg` dosyanın tümünü yazar (ADR 9168):
  - dosya bellekte kurulur;
  - yanına geçici adla yazılır (`.<ad>.<kimlik>.yaziliyor`);
  - geri okunup denetlenir;
  - adın yerine konur (`files::write_bytes`).

  Yarıda kesilmenin sınaması yoktu. Teslimin Durdur'u da yok.
- **Katmanın kendi dışa aktarması** (`cad.layers.export`, ADR 9106) var olan dosyada yalnız kendi tablosunu yeniden
  yazar (“Katmanı değiştir”) ya da satır ekler (“Katmana ekle”). Öbür tablolar dokunulmaz.
- Aynı seçenekleri proje teslimine eklemek komutun girdisini değiştirir; ortak durumlar, Python ve MCP de değişir. Bu
  ayrı bir iştir (§4).
- **Araştırma.**
  - QGIS 3.44 kullanıcı kılavuzunun “Creating Layers” bölümüne bakıldı
    ([var olan dosyaya kaydetme](https://docs.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html);
    ADR 9168'de okundu). Masaüstü CBS'lerde var olan dosyaya kaydederken bütün dosyanın üstüne yazmak, yalnız
    katmanın üstüne yazmak ya da katmana eklemek seçilir.
  - Kesilmeyi gerçek süreç ölümüyle sınama yöntemi çizimin kaydından alındı (ADR 1006).

## §2 Karar

1. **Kesilme sınaması** (`crates/native/headless/tests/headless/gpkg_export_crash.rs`).
   - Sınama kendini çocuk süreç olarak başlatır.
   - Çocuk, bir çizimin nokta katmanının üç sürümünü (22.000, 24.000 ve 26.000 nokta daha) aynı dosyaya,
     `file.export.gpkg`'nin yazdığı gibi durmadan yazar.
   - Ana süreç çocuğu on ayrı anda SIGKILL ile öldürür.
   - Her ölümden sonra adın altındaki dosya SQLite'ın bütünlük denetiminden geçmeli ve eski teslimin ya da bir
     sürümün bütün noktalarını tutmalıdır.
   - Geçici dosya hiçbir zaman teslimin adını almamalıdır.
2. **Öbür tabloları korumanın yolu.** Üstüne yaz onayında dosyadan silinecek tablo varsa, altına şu satır gelir:
   “Öbür tabloları korumak için Geri'ye basın ve katmanları tek tek yazın: katmanın menüsündeki Dışa aktar ▸ Farklı
   kaydet… bu dosyada yalnız o tabloyu yeniden yazar (Katmanı değiştir) ya da satır ekler (Katmana ekle).”
3. **Kapsam dışı:**
   - teslimin kendisine “yalnız bu tablolar” ya da “ekle” seçeneği (komut girdisi değişir);
   - uzun teslimin durdurulması.

## §3 Kanıt

- **Headless** (`gpkg_export_crash`):
  - on ölümün her birinde adın altında bütün bir GeoPackage kaldı;
  - öldürmeler en az iki farklı teslime denk geldi;
  - geçici dosya adı almadı;
  - 27 s.
- **Masaüstü** (`shell/exchange/gpkg_tests.rs`): ADR 9168'in onay sınaması, silinecek `eski_tablo`nun altında
  korumanın yolunu söyleyen satırı da arıyor. Dışa aktarma, GeoPackage ve ortak masaüstü sınamaları geçiyor (329).
- **Resim:** `.run/gui-evidence/kullanim-9178/01-uzerine-yaz-{light,dark}.png`. Onayda silinecek “eski_tablo”,
  altında korumanın yolu, yeni tablolar ve yazılanlar görünüyor.

## §4 Kalan

- Teslimde “yalnız bu tabloları yeniden yaz” ve “tablolara ekle”. Bunun için `file.export.gpkg`'nin girdisine bir alan
  eklenir; ortak durumlar, Python ve MCP de değişir.
- Uzun teslimin Durdur'u. Bugün teslim bellekte, arayüzün iş parçacığında kuruluyor.
- Teslimatın başka bir CBS okuyucusunda doğrulanması; web (TODOS.md §0.2).
