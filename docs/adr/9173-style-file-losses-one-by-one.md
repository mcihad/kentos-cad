# ADR 9173: Stil dosyası alışverişinde her kayıp ayrı satırda söylenir

- **Durum:** kabul edildi (2026-11-23); adım VY-72. Plan: [plan/veri.md](../../plan/veri.md) VY-72.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 10. maddesinin
  kalanı (ADR 9143): “Stil yükle/kaydet ve QML alışverişinde kayıp özellikleri tek tek bildir.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** yok. `file.export.qml/sld` ve `cad.layers.qml/sld` çıktılarının `lost` listesi zaten vardı.

## §1 Bağlam

- QML ve SLD alışverişinin komutları kaybolan her parçayı ayrı bir cümleyle çıktının `lost` listesine yazar (ADR 0555,
  0635).
- Uyarı ise hepsini tek bir cümlede birleştirir: “QML'in bazı parçaları alınamadı: a; b; c.”
- Masaüstü yalnız bu birleşik uyarıyı söylüyordu. Dört kayıplı bir dosyada kişi uzun tek bir satırı okumak zorundaydı;
  kayıplar sayılamıyordu.
- **Araştırma.** Masaüstü CBS'lerde stil dosyası alışverişinde okunamayan ya da yazılamayan her özellik ayrı bir
  ileti olarak raporlanır ve kişi hangisinin kaybolduğunu tek tek görür. Kaynak:
  [QGIS vektör katman özellikleri, Style menüsü ve stil yönetimi](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html).

## §2 Karar

1. **Satır satır** (`App::say_losses`). Bir stil dosyası yazılınca ya da alınınca kayıplar şöyle söylenir:
   - önce bir başlık ve sayı: “QML'den alınamayan 4 özellik:”, “QML'in söyleyemediği 2 özellik:”, “SLD'den alınamayan
     …”, “SLD'nin söyleyemediği …”;
   - sonra her kayıp kendi numarasıyla ayrı satırda: “1. katmanın saydamlığı alınmadı (Katman çizimi ayrıdır)”.
2. **Yinelenmez.** Kayıpları yalnız toplayan uyarı (`import_losses`, `export_losses`) kayıplar listelenince yeniden
   söylenmez. Öbür uyarılar eskisi gibi söylenir.
3. **Kapsam.** QML'in ve SLD'nin yazma ve alma yollarının dördü de aynı yardımcıyı kullanır.
4. **Kalan.** Uyarının kendisi komutun çıktısındadır ve platformlar arası sözleşmedir; birleşik cümlesi değişmedi.
   Python ve MCP `lost` listesini okur.

## §3 Kanıt

- **Masaüstü** (`gis/layer_qml_tests.rs`):
  - ortak `kayipli.qml` parsel katmanına alınınca başlık “QML'den alınamayan N özellik:” çıkıyor ve ardından N numaralı
    satır geliyor;
  - birleşik “bazı parçaları alınamadı” cümlesi yinelenmiyor;
  - SLD yazımında başlık “SLD'nin söyleyemediği 2 özellik:”, iki satır.
- Stil, katman, alışveriş ve ortak masaüstü sınamaları geçiyor (480).
- **Resim:** `.run/gui-evidence/kullanim-9173/01-qml-kayiplari-{light,dark}.png`. Komut geçmişinde “QML'den
  alınamayan 4 özellik:” ve dört numaralı satır.

## §4 Kalan

- 10. maddenin öbür kalanı: yolun planlı→kontrol geçişinde harita, sınıf sayacı, tablo ve çıktının aynı politikayla
  uçtan uca sınanması.
- Kayıpların bir pencerede tablo olarak gösterilmesi; şimdilik komut geçmişinde satır satır.
- Web (TODOS.md §0.2).
