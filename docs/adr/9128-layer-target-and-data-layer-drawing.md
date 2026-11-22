# ADR 9128: CBS projesinde katmana bir basış hedefi seçer; veri katmanına yalnız oturumunda ve geometrisiyle çizilir

- **Durum:** kabul edildi (2026-11-22); adım VY-27. Plan: [plan/veri.md](../../plan/veri.md) VY-27.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  4. maddesi (P1):
  - parsel satırına bir kez basınca vurgu değişti ama şerit ve denetçi hedefi Taslak göstermeye devam etti;
  - hedef ancak sağ tık › Etkin katman yap ya da çift tıklamayla değişti;
  - yeni Durak katmanında Nokta düğmesi düzenleme oturumu kapalıyken de seçilebildi, haritaya tıklayınca durum satırında
    kesik bir uyarı çıktı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- **Masaüstü CBS'ler:** QGIS'te katmanlar panelinde seçilen katman geçerli katmandır; düzenleme araçları ona yazar ve
  katman düzenlemeye açık değilse çalışmaz (QGIS 4.2 masaüstü komutları, değerlendirmede okunan kaynak). ArcGIS Pro'da
  düzenleme oturumu açık değilse çizim şablonları yazmaz (*Enable and disable editing*).
- **KentOS'ta:**
  - çift tıklama CAD alışkanlığıdır, katmanların seçilmesi çizilmesinden çok daha sıktır;
  - türlü katmanlarda çizim aracının uymadığı tür zaten reddediliyordu (`misfit`, ADR 0622 §5);
  - veri katmanları (ADR 9101) bu denetime girmiyordu.

## §2 Karar

1. **Bir basış.** Türü CBS olan projede Katmanlar'da bir katmana bir kez basmak onu etkin katman (çizim hedefi) yapar.
   - Grup ve harita servisi satırları yalnız seçilir.
   - Türü CAD olan ya da türü olmayan çizimde çift tıklama kuralı sürer.
2. **Veri katmanına çizim** (`layer_types::misfit`). Nesne yazan bir araç etkin veri katmanında şu durumlarda çalışmaz,
   nedenini söyler:
   - kaynak salt okunursa;
   - düzenleme oturumu kapalıysa: “… düzenlemeye kapalı: çizmek için önce Düzenlemeyi başlat'ı seçin …”;
   - tablo geometrisizse;
   - tablonun geometrisi aracın yazdığını almıyorsa: “… bir alan katmanı; nokta bu katmana yazılmaz …”.

   Düğme kapalı görünür, ipucu nedeni söyler. Yine de çağrılırsa (komut satırı, kısayol) uyarı ve **Düzenlemeyi başlat**
   düğmeli bir bildirim çıkar.
3. **Şeridin Etkin katman alanı** etkin katmanı gösterir. İpucu veri katmanının oturumunu söyler (düzenleniyor,
   düzenlemeye kapalı, salt okunur) ve bir basışla seçildiğini anlatır.

## §3 Kanıt

- **Masaüstü** (`gis/layer_target_tests.rs`):
  - CBS projesinde bir basış hedefi değiştiriyor;
  - CAD projesinde bir basış yalnız seçiyor, çift tıklama hedefi değiştiriyor;
  - parsellerin veri katmanı oturum kapalıyken: alan aracı kapalı, neden “düzenlemeye kapalı”, bildirim Düzenlemeyi
    başlat'ı sunuyor;
  - oturum açılınca alan aracı çalışıyor, nokta aracı “alan katmanı” diye reddediliyor.
- Katman ağacı (`layering::tests`), şerit, modül ve veri düzenleme sınamaları geçiyor.

## §4 Kalan

- Düzenle sekmesinde hedef katmanı, kalemi ve Düzenlemeyi başlat/bitir'i sürekli gösteren ayrı bir denetim. Bugün
  katmanın menüsünde, bildirimde ve Katman değişiklikleri'nde var.
- Yetki yokken gerekçe ve iki açık oturumda yanlış katmana yazımın sınanması (raporun 4. maddesinin kalanı).
- Web (TODOS.md §0.2).
