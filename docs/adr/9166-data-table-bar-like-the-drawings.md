# ADR 9166: Veri katmanının tablo çubuğu çizim katmanınınkiyle aynı düzendedir

- **Durum:** kabul edildi (2026-11-23); adım VY-65. Plan: [plan/veri.md](../../plan/veri.md) VY-65.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 9. maddesinin
  kalanı (ADR 9142): “Aynı terim ve klavye davranışlarını iki sağlayıcıda kullan. Farklı yetenek için araç üzerinde
  neden ve sonraki adımı göster.” Proje içi yolun tablosunda Tablo/Form geçişi, arama ve düzenleme eylemleri vardı;
  aynı yolun GeoPackage görünümünde başka bir araç düzeni çıkıyordu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Çizim katmanının Öznitelik tablosu çubuğu şunları sunar: Tablo | Form, Katman, Ara, Göster, İfade süzgeci; Seçime
  yakınlaş, Sütunlar, Alan hesapla, Kopyala, Dışa aktar, İncele ve ⋯ (Kayıt ekle, Çoğalt, Yapıştır, Alanlar…).
- Veri katmanının çubuğunda yalnız Katman, Ara, Göster, İfade süzgeci ve yakınlaştırma vardı. Bir araç yoksa kişi
  onun neden olmadığını ya da işin nasıl yapılacağını göremiyordu.
- **Araştırma.** Masaüstü CBS'lerin öznitelik tablosu her katmanda aynı araç çubuğunu gösterir. Belge, verinin
  biçimine ve sürücüye göre bazı araçların kullanılamayabileceğini ayrıca not eder. Kaynak:
  [QGIS “Working with the Attribute Table”, Table 12.1 Available Tools](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/attribute_table.html).

## §2 Karar

1. **Aynı araçlar, aynı sıra** (`features/data_bar.rs`). Veri katmanının çubuğu çizim katmanınınkiyle aynı araçları
   aynı sırada gösterir. Ara ve İfade süzgeci Enter'la sorar; kaynak her harfte sorulmaz ve kutunun yazısı bunu söyler.
2. **Veri katmanında çalışanlar:**
   - Seçime yakınlaş;
   - Sütunlar: kaynağın alanlarını tabloda gösterir ya da gizler, çizim tablosundaki gibi;
   - Dışa aktar ▾: “Farklı kaydet…” ve tablonun seçimiyle “Seçilenleri farklı kaydet…”. Katmanlar'daki Dışa aktar
     ile aynıdır;
   - İncele ▾: İstatistik ve Grafik; bunlar kaynağın satırlarını okur (ADR 9130);
   - ⋯ › Alanlar….
3. **Çalışmayanlar kapalıdır.** İpuçları nedeni ve yerine yapılacak işi söyler:
   - Form: “Veri katmanında satırın formu Formla düzenle ile açılır: satırı seçip Veri sekmesinde Formla düzenle'ye
     basın.” Basılırsa tablo kalır ve aynı söz söylenir;
   - Alan hesapla: düzenleme oturumunda hücreye çift tıklanır;
   - Kopyala: satırlar sayfa sayfa okunduğu için panoya gitmez; Dışa aktar kullanılır;
   - Kayıt ekle, Çoğalt, Yapıştır: yeni satır düzenleme oturumunda çizim araçlarıyla eklenir.

## §3 Kanıt

- **Masaüstü** (`gis/features/data_bar_tests.rs`):
  - Form'a basınca tablo kalıyor ve “Formla düzenle ile açılır” deniyor;
  - Sütunlar “kullanim”ı gizleyince veri tablosu “ada, alan” gösteriyor, Hepsini göster geri getiriyor;
  - İncele › İstatistik veri katmanında açılıyor.
- Öznitelik tablosu sınamaları (39) geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9166/01-veri-tablosu-cubugu-{light,dark}.png`. Veri katmanının çubuğunda
  Tablo | Form, Katman, Ara (Enter), Göster, İfade süzgeci (Enter) ve araçlar çizim tablosundaki sırayla.

## §4 Kalan

- 9. maddenin öbür kalanları: seç→bul→kart→form→süz→temizle zincirinin ve boş/NULL/0/kodlu/Türkçe örneklerin bir
  sınamada uçtan uca denenmesi.
- Veri katmanında Form bölmesi (seçili satırın formu bölmede) ve kopyalama.
- Web (TODOS.md §0.2).
