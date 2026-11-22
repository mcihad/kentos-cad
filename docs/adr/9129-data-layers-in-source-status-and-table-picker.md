# ADR 9129: Kaynak durumu veri katmanlarını gerçek satır sayısıyla gösterir; tablo seçicisi aynı adlıları ayırır

- **Durum:** kabul edildi (2026-11-22); adım VY-28. Plan: [plan/veri.md](../../plan/veri.md) VY-28.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  5. ve 28. maddeleri (P1):
  - doğrudan açılan GeoPackage'ın geometrileri görünürken Kaynak durumu bağlı katman olmadığını söyledi;
  - bir satırı olan veri tablosu açıkken denetçinin Nesne sayısı sıfırdı;
  - yeni Durak çizim hedefi olduğu hâlde alttaki tablo Yol ekseni olarak kaldı;
  - GeoPackage'tan `bakim` ile PostGIS'ten `denetim.bakim` tablo seçicisinde ikisi de “bakim (veri)” göründü.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Kaynak durumu (ADR 0543) yalnız eski “bağlı katmanları” (`settings.sources`) okuyordu. Veri katmanları (ADR 9101)
  çizimde nesne değildir, satırları kaynaklarında durur.
- Denetçinin Nesne sayısı çizimin nesnelerini sayar. Değerlendirmenin istediği, satırların ayrı adlandırılmasıdır:
  bellek nesnesi, sorgu sonucu, görünümde yüklenen satır, gerçek tablo sayısı.
- Tablonun katmanı bilerek sabitlenebilir (Katman ▾). Çizim hedefi başka olduğunda ikisi de okunmalı.

## §2 Karar

1. **Kaynak durumu'nda “Veri katmanları”** (`gis/source_status_data.rs`). Bağlı katmanların altında bir tablo, katman
   başına bir satır:
   - Katman;
   - Kaynak: dosya › tablo ya da veritabanı › şema.tablo;
   - Erişim: yazılabilir ya da salt okunur;
   - Düzenleme: kapalı, düzenleniyor ya da kaynağa yazılmayan satırlar;
   - Satır.

   Satır sayısı tablonun kaynaktaki gerçek sayısıdır. Pencere açılınca ve Yenile'de arayüzü bekletmeden sorulur;
   kaynak yanıtlayana kadar “hesaplanmadı”, okunamazsa “okunamadı” yazar, nedeni altta söylenir. Haritada ya da
   tabloda o an yüklenen satırların bu sayı olmadığı yazılıdır.
2. **Boş bildirimi yok.** Bağlı katman olmayıp veri katmanı olan projede ilk satır “Projede bağlı katman yok; verisi
   aşağıdaki veri katmanlarında, kaynaklarında.” der.
3. **Denetçi.** Hiçbir şey seçili değilken Çizim bölümüne “Veri katmanları: 2 katman; satırları kaynaklarında (sayısı
   Kaynak durumu'nda)” satırı eklenir. Nesne sayısı yalnız çizimin nesneleridir.
4. **Tablonun hedefi.** Tablo çizim hedefinden başka bir katmanı gösterirken araç çubuğu “Çizim hedefi: Durak” der.
5. **Aynı adlı katmanlar.**
   - Katman ▾'da veri katmanı “(veri)” diye yazılır. Aynı adla başka bir katman varsa kaynağı eklenir:
     “bakim (veri · Mahalle-Verileri.gpkg › bakim)”.
   - Salt okunur kaynak “salt okunur” diye yazılır.

## §3 Kanıt

- **Masaüstü** (`gis/source_status_tests.rs`):
  - iki GeoPackage'ın iki parsel tablosu: Kaynak durumu iki veri katmanını listeliyor, sayılar önce boş, sonra kaynağın
    sayısı (4 ve 2);
  - denetçi “2 katman; satırları kaynaklarında” diyor;
  - aynı adlı iki katman seçicide kaynaklarıyla ayrılıyor;
  - tablo başka katmandayken “Çizim hedefi:” yazıyor.
- Kaynak durumu, öznitelik tablosu ve denetçi sınamaları (104) geçiyor.

## §4 Kalan

- Projeye alınmış kopyaların ve eski bağlı katmanların aynı katalogda fiziksel kaynağa bağlanması (5. maddenin
  kalanı).
- Form başlığında salt okunur durumu ve iki PostGIS bağlantısı ile bir GeoPackage arasında on geçişlik yazım sınaması
  (28. maddenin kalanı).
- Web (TODOS.md §0.2).
