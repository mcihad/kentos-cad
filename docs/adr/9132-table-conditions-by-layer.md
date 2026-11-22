# ADR 9132: Tablonun arama, ifade ve sırası katmanındır; katmanda olmayan alan söylenir

- **Durum:** kabul edildi (2026-11-22); adım VY-31. Plan: [plan/veri.md](../../plan/veri.md) VY-31.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  20. maddesi (P1):
  - `bakim` tablosunda `bedel >= 100` etkinken Katman ▾'dan `mahalleler (veri)` seçildi;
  - koşul yeni katmanda da sorulmaya devam etti ve tablo “Kaynakta soruya uyan satır yok” dedi;
  - koşul silinince M1 Çınar, M2 İstasyon ve M3 Kale geldi;
  - ikinci projede envanterden parsele geçerken `sira >= 1201` ile aynısı görüldü.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `cad.layers.rows`, katmanda olmayan alanı anan süzgeci artık `field_not_found` ile reddeder. Önceden
  satırsız yanıt verirdi.

## §1 Bağlam

- Öznitelik tablosunun Ara, İfade süzgeci ve sırası panelin tek alanlarıydı (`FeaturesPanel`). Veri katmanında
  ayrıca Enter'la sorulan karşılıkları vardı (`DataTable`). Katman değişince sıralama sıfırlanıyor, arama ve süzgeç
  yeni katmana taşınıyordu.
- Satırın değerleri yalnız kendi alanlarından okunur. Katmanda olmayan alanı anan koşul her satırda boş değerle
  karşılaşır ve hiçbir satırı tutmaz. Bu boş yanıt verinin yanıtı gibi görünüyordu.
- Kaynak okuyucu sıralamadaki bilinmeyen alanı zaten reddediyordu (“… alanı … katmanında yok”). Süzgeçteki alanı ise
  denetlemiyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasında:
  - öznitelik tablosunun sıra ifadesi ve yönü katmanın tablo ayarında (`QgsAttributeTableConfig`) saklanıyor ve proje
    açılınca geri geliyor. Yönün kalıcılığı 2016'daki bir düzeltmeyle eklendi (hata kaydı 15235);
  - süzgeç ifadesi o katmanın tablo penceresinin süzgeç modelinde (`QgsAttributeTableFilterModel`) durur, başka katmanın
    tablosuna geçmez.

  Kaynaklar: [tablo ayarı kaynağı](https://api.qgis.org/api/3.44/qgsattributetableconfig_8cpp_source.html),
  [süzgeç modeli kaynağı](https://api.qgis.org/api/3.44/qgsattributetablefiltermodel_8cpp_source.html),
  [hata kaydı 15235](https://issues.qgis.org/issues/15235.pdf).

## §2 Karar

1. **Koşullar katmanındır** (`gis/features/conditions.rs`). Tablo bir katmandan ayrılınca o katmanın koşulları
   katmanın altında saklanır:
   - Ara;
   - İfade süzgeci, hem yazılan metin hem Enter'la sorulan;
   - sıra ve yönü.

   Anahtar katmanın kimliği ve kaynağıdır (veri katmanında dosya ya da veritabanı ile tablo). Kaynağı değişen katman
   başka tablodur. Tablonun geldiği katman kendi koşullarını getirir; ilk kez gösterilen katmanda koşul yoktur.
2. **Her geçişte.** Koşullar tablonun katmanını izler:
   - Katman ▾ seçince hemen;
   - seçim ya da etkin katman tabloyu başka katmana götürdüğünde ilk iletide.
3. **Ömür.** Koşullar çizim ekrandayken saklanır. Başka çizim tabloyu baştan başlatır (GIS-022). Proje dosyası onları
   tutmaz.
4. **Katmanda olmayan alan söylenir.**
   - Veri katmanında okuyucu (`layers_rows`) süzgeçteki alanları katmanın alanlarıyla karşılaştırır. Eksik alanda
     `field_not_found` döner: “Süzgeç uygulanmadı: “bedel” alanı “mahalleler” katmanında yok. Katmanın alanları:
     mahalle, ad.” Geniş tablonun ilk on alanı yazılır, kalanı sayıyla. Bu kural Python ve MCP çağrılarında da
     geçerlidir.
   - Çizimin katmanında, ne katmanın alanlarında ne nesnelerinde bulunan alan aynı sözle söylenir.
   - Tablonun ayağı söz dizimi hatasını “Süzgeç okunamadı: …” diye yazar. Eski “süzgeç okunamadı:” öneki ikisine
     birden konmaz.
5. **Reddedilen soru kendiliğinden yinelenmez.**
   - Veri katmanının tablosu, kaynağın reddettiği ya da okuyamadığı soruyu her iletiden sonra yeniden soruyordu. Bu,
     yanıt vermeyen kaynağa durmadan sormak demekti.
   - Artık soru Enter'la, başka bir soruyla ya da bir yazımdan sonra yeniden sorulur.
   - Neden satırların yerinde yazılır, durum satırı “Satırlar okunmadı.” der.

## §3 Kanıt

- **Masaüstü** (`gis/features/conditions_tests.rs`):
  - raporun verisi: `bakim`'de `bedel >= 100` B02, B03 ve B06'yı tutuyor;
  - `mahalleler`'e geçince koşul yok, M1–M3 geliyor;
  - `bakim`'e dönünce koşul ve üç satır geri geliyor;
  - koşul `mahalleler`'e yazılınca neden söyleniyor, satır sayısı verilmiyor;
  - çizimin Parsel ve Yapı katmanlarında koşul ve sıra katmanla gidip geliyor;
  - Yapı'da olmayan `Ada` söyleniyor, bir nesnenin taşıdığı `[Not]` soruluyor.
- **Uygulama** (`tests/all/layers_rows_fields.rs`): eksik alan `field_not_found` ve tam cümle; kendi alanlarıyla gerçek
  sıfır yanıt hâlâ sıfır; çoğul söz ve geniş tablo.
- Öznitelik tablosu, veri katmanı, izler, şerit, modül, katalog ve ayar sınamaları (168 + 75) geçiyor.
- Resimler: `.run/gui-evidence/kullanim-9132/` (Git'e girmez).

## §4 Kalan

- Koşulları proje dosyasında saklamak (sahibin isteğine bağlı).
- Çizimin katmanında reddedilen süzgecin kutusunun kırmızı çerçevesi veri katmanının kutusunda yok; neden tabloda ve
  durumda yazılı.
- Web (TODOS.md §0.2).
