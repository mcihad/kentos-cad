# ADR 9133: İfadeyle seç veri katmanında koşulu kaynağa sorar, seçimi satır anahtarlarıyla kurar

- **Durum:** kabul edildi (2026-11-22); adım VY-32. Plan: [plan/veri.md](../../plan/veri.md) VY-32.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  18. maddesi (P1):
  - alt tabloda `bedel >= 100` B02, B03 ve B06'yı getirdi;
  - Harita › İfadeyle seç › Katman › bakim › aynı koşul için önizleme “uygun nesne yok” dedi;
  - Çalıştır başka katman seçilmesini istedi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `selection.byExpression` aracının kendisi değişmedi. Masaüstü veri katmanında onu çizim yerine kaynakta
  çalıştırır.

## §1 Bağlam

- İfadeyle seç bir işlem aracıdır. Girdisini çizimin nesnelerinden çözer, seçimi nesne yuvalarıyla (`Slot`) verir.
- Veri katmanının satırları çizimde nesne değildir, kaynaklarında durur (ADR 9101). Bu yüzden araç hiçbir şey
  bulamıyordu.
- Tablonun süzgeci aynı koşulu satır okuyucusuyla (`layers_rows::read`) kaynağa soruyordu (ADR 9109). Tablonun seçimi
  satır anahtarlarıdır (`DataTable::selected`). Bu seçim haritada çerçeveyle çizilir (ADR 9102 §5).
- **Araştırma.** Başvuru alınan CBS uygulamasında katmanın ifadeyle seçimi:
  - ifadeyi bir özellik sorgusuna verir, gereken alanları ister ve katmanın kayıtlarını dolaşır;
  - seçimi kayıt kimlikleriyle kurar: yeni, ekle, çıkar, seçim içinde.

  Kaynaklar: [vektör katmanı kaynağı, satır 563–569](https://api.qgis.org/api/4.2/qgsvectorlayer_8cpp_source.html#l00563)
  (değerlendirmenin bağlantısı), [ifadeyle seçim kılavuzu](https://docs.qgis.org/3.44/en/docs/user_manual/introduction/general_tools.html).

## §2 Karar

1. **Kaynakta sorulur** (`gis/processing/data_select.rs`). İfadeyle seç'in girdisi Katman ve bu katman veri katmanıysa
   Çalıştır koşulu tablonun okuyucusuyla kaynağa sorar:
   - arayüzü bekletmeden;
   - koşulu tutan satırların anahtarlarını (`all_keys`) ve satır sayısını alır.
2. **Seçim anahtarlarla.** Seçim biçimi tablonun o katmandaki seçimine uygulanır:
   - Yeni seçim: tutan anahtarlar;
   - Seçime ekle: birleşim;
   - Seçimden çıkar: fark;
   - Seçim içinde ara: kesişim.

   Tablo katmana gelir, satırlar orada seçili olur, harita onları çerçeveler. Durum “3 / 6 satır koşulu sağladı;
   seçimde 3 satır var.” der. Hiç satır tutmazsa uyarı rengindedir.
3. **Seçime yakınlaştır.** Pencerenin düğmesi ve Görünüm'ün Seçime yakınlaştır komutu çizimde seçim yokken veri
   katmanının seçili satırlarını çerçeveler. Komut bu durumda da kullanılabilir.
4. **Girdi ve önizleme.**
   - Girdi satırı “uygun nesne yok” yerine ““bakim” veri katmanı: 6 satır kaynağında; koşul kaynakta sorulur.” der;
     katmanın alanları koşula düğme olarak sunulur.
   - Önizleme İstatistik'in okuduğu satırları sayar (ADR 9130): “3 / 6 satır koşulu sağlıyor.”
   - Satırlar okunurken bunu söyler. Katmanda olmayan alanı ADR 9132'nin sözüyle söyler.
5. **Durdur.** Kaynağa sorulan iş durdurulabilir. Durdurulan iş seçimi değiştirmez: “Durduruldu; seçim değişmedi.”
6. **CAD özellik süzgeci** yalnız çizim nesnelerine sorar. Veri katmanının satırlarının CAD rengi ve yazı yüksekliği
   yoktur, bu yüzden açıkken çalıştırma nedeniyle reddedilir.

## §3 Kanıt

- **Masaüstü** (`gis/processing/data_select_tests.rs`), raporun bakım verisiyle:
  - `bedel >= 100` B02, B03, B06'yı seçiyor; tablo katmanda, harita çerçeveliyor;
  - Seçime yakınlaştır üç satırın çizimdeki yerlerini (dosyanın sisteminden projeninkine taşınmış) görüntüye alıyor;
  - `bedel IS NULL` yalnız B04;
  - ekle, çıkar ve seçim içinde ara aynı anahtarlarla;
  - sıfır eşleşme uyarı;
  - önizleme önce “okunuyor”, sonra “3 / 6”;
  - eksik alan söyleniyor;
  - durdurulan iş seçimi koruyor;
  - CAD süzgeci reddediliyor.
- İşlemler, ifade oluşturucu, gezinme, öznitelik tablosu, veri katmanı, izler, şerit, modül, katalog ve ayar
  sınamaları (351) geçiyor.
- Resimler: `.run/gui-evidence/kullanim-9133/` (Git'e girmez).

## §4 Kalan

- Girdinin Seçili kapsamı veri katmanının seçili satırlarıyla. Bugün Katman kapsamı ve Seçim içinde ara bu işi görür.
- Veri katmanında `$alan` gibi ölçüler tablonun okuma birimiyle sorulur, İşlemler'in metre birimiyle değil. Metre dışı
  çizim biriminde ikisi ayrılır.
- Web (TODOS.md §0.2).
