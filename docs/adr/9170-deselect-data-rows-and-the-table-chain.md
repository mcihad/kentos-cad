# ADR 9170: Seçimi kaldır veri tablosunun seçimini de temizler; tablo zinciri uçtan uca sınanır

- **Durum:** kabul edildi (2026-11-23); adım VY-69. Plan: [plan/veri.md](../../plan/veri.md) VY-69.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 9. maddesinin
  kalanı (ADR 9142, 9166): “Bir kayıt seç → haritada bul → bilgi kartını aç → formu doldur → tabloyu filtrele →
  seçimi temizle zincirinde katman ve kararlı kayıt anahtarı korunmalı. Boş metin, NULL, 0, kodlu değer, Türkçe arama
  ve sıralama ayrı örneklerle sınanmalı.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok; `edit.deselect`'in kapsamı
  genişledi.

## §1 Bağlam

- Zincirin halkaları tek tek sınanmıştı:
  - İfadeyle seç veri katmanında anahtarlarla seçiyor ve Seçime yakınlaştır onları çerçeveliyor (ADR 9133);
  - Bilgi kartı satırın katmanını, tablosunu ve anahtarını söylüyor (ADR 9145);
  - Formla düzenle tablodan seçilen satırı açıyor (ADR 9149);
  - tablonun süzgeci kaynakta çalışıyor (ADR 9102).
- Zincirin son halkası kopuktu. Seçimi kaldır (`edit.deselect`) yalnız çizimin seçimini temizliyordu. Veri tablosunda
  seçilen satırlar kalıyordu; çizimde seçim yokken komut kapalı görünüyordu.
- **Araştırma.** Masaüstü CBS'lerin öznitelik tablosunda “Deselect all features from the layer” seçimi tablodan da
  haritadan da kaldırır. Tablo ile haritanın seçimi birdir. Kaynak:
  [QGIS “Working with the Attribute Table”, Selecting features](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/attribute_table.html).

## §2 Karar

1. **Seçimi kaldır her seçimi temizler.**
   - Çizimin seçimini ve veri tablosunda seçilen satırları birlikte temizler (`DataTable::clear_selection`).
   - Tablo kendi katmanında kalır.
   - Yalnız veri satırı seçiliyken de açıktır.
2. **Zincir uçtan uca sınanır.** Değerlendirmenin GeoPackage'ında `bakim` üzerinde şu adımlar yürür; seçimin katmanı
   ve satırın anahtarı (4) baştan sona korunur:
   - İfadeyle seç `bedel IS NULL` → B04;
   - Bilgi kartı “› bakim · fid 4”, NULL bedel boş hücre, mahalle M2;
   - Formla düzenle başlıkta “bakim · fid 4”;
   - süzgeç `mahalle = 'M1'` → 2 satır; seçim ve katmanı korunur;
   - Seçimi kaldır → seçili satır kalmaz, tablo `bakim`'de.
3. **Örnek değerler.**
   - **0 bir değerdir:** `bedel = 0` B01 ile 0 bedelli B07'yi bulur; `bedel IS NULL` yalnız B04'ü.
   - **NULL ve boş metin aynı boştur.** KentOS'un ifade dili ikisini tek bir boş değer sayar (ADR 0581 §3, 9109):
     `mahalle IS NULL` da `mahalle = ''` da NULL B06'yı ve boş metinli B07'yi bulur. Bu dilin kuralıdır; tabloda
     ikisinin ayrı işaretlenmesi 29. maddenin kalanıdır.
   - **Türkçe sıra:** ada göre sıralı mahalleler Çınar, İstasyon, Kale. Harf kodlarının sırası Kale'yi öne alırdı.
   - **Türkçe arama:** “istasyon” İstasyon'u bulur.

## §3 Kanıt

- **Masaüstü** (`gis/features/chain_tests.rs`, iki sınama): zincir (§2.2) ve örnekler (§2.3).
  - B07 sınamada dosyaya yazılan yedinci iştir: boş metin mahalle, 0 bedel.
- Öznitelik tablosu, İfadeyle seç, seçim, şerit kısayolları ve ortak masaüstü sınamaları geçiyor (200 ve 92).
- **Resim:** `.run/gui-evidence/kullanim-9170/01-secimi-kaldir-{light,dark}.png`. Tabloda B04 seçili (“1 seçili”),
  şeritte Seçimi kaldır açık.

## §4 Kalan

- Kodlu değerin (değer listesi) veri katmanı tablosunda kodu ve adıyla sınanması.
- NULL ile boş metnin tabloda ayrı işareti (29. madde).
- Aynı zincirin PostGIS katmanında gerçek sunucuyla denenmesi.
- Web (TODOS.md §0.2).
