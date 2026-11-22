# ADR 9137: Bilgi yerel rasterin tıklanan hücresini söyler

- **Durum:** kabul edildi (2026-11-22); adım VY-36. Plan: [plan/veri.md](../../plan/veri.md) VY-36.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  24. maddesi (P2):
  - Harita › Bilgi ile poligonların dışında 42 m'lik ve 12,5 m'lik hücrelere tıklandı;
  - bant/değer kartı açılmadı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok (okuma).

## §1 Bağlam

- Bilgi'nin tıklaması çizimin bir nesnesine ya da veri katmanının satırına değerse kart açılıyordu (ADR 0225 §5, 9101 §6).
- Yerel raster çizimde bir nesnedir ama hücreleri dosyasındadır. Tıklama çerçeveye değmedikçe nesne sayılmıyordu.
- Koordinat oku gösterilen rasterlerin hücrelerini arayüzü bekletmeden okuyup günlüğe yazıyordu
  (`rasters::raster_values_at`). Bilgi bunu kullanmıyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasının bilgi sorgusu raster katmanında tıklanan pikselin bant değerlerini
  katmanın adıyla gösterir. Aynı yerdeki vektör kaydıyla raster değeri sonuç ağacında yan yana durur.
  Kaynak: [genel araçlar: bilgi sorgusu](https://docs.qgis.org/3.44/en/docs/user_manual/introduction/general_tools.html)
  (değerlendirmenin bağlantısı).

## §2 Karar

1. **Hücre okunur** (`gis/rasters/cell_info.rs`). Bilgi'nin tıklamasıyla gösterilen rasterler noktanın altında,
   arayüzü bekletmeden sorulur; veri katmanlarıyla aynı işte, nesneye değen tıklamada ayrı bir işte. Her hücre şunları
   söyler:
   - rasterin katmanı;
   - her bandın dosyadaki değeri;
   - hücrenin sütunu ve satırı;
   - noktanın koordinatları (Y, X).

   Dosyanın NoData değeri (ya da NaN) “değer yok (NoData)” diye yazılır. 32 bitlik kayan sayı kendi kısa yazımıyla
   gösterilir (110.52912), çiftinin bütün haneleriyle değil.
2. **Kart.**
   - Nesneye ve satıra değmeyen tıklamada en üstteki rasterin hücresi karttır: “Raster: Bölge rasteri”.
   - Nesneye ya da satıra değen tıklamada kartın altında “Raster değeri: …” düğmesi değere, raster kartındaki “Nesneye
     dön” geri geçirir.
3. **Kapsam dışı.** Gösterilen raster varken tıklama hiçbirinin üstünde değilse ileti satırı “Tıklanan nokta
   gösterilen rasterlerin kapsamı dışında.” der.
4. **Birim.** Okunan dosyalar bandın birimini vermiyor; birim tahmin edilip yazılmaz.

## §3 Kanıt

- **Masaüstü** (`gis/rasters/cell_info_tests.rs`), ortak raster çiziminin `bolge.tif`'i (12 × 10 Float32, 2 m,
  NoData NaN). GDAL'ın `gdallocationinfo`'su başvuru:
  - (0, 0) 110.529121398926 ve (11, 9) 131.867736816406: kart aynı 32 bitlik değeri veriyor;
  - (6, 2) NaN: kart “değer yok (NoData)” diyor;
  - rasterin solunda kapsam dışı söyleniyor;
  - nesneye değen tıklamada değer sunuluyor, geçiş iki yönde çalışıyor.
- Veri katmanının Bilgi sınaması yeni yanıt biçimiyle geçiyor. Raster, Bilgi, izler, şerit ve modül sınamaları (84)
  geçiyor.
- Resimler: `.run/gui-evidence/kullanim-9137/` (Git'e girmez).

## §4 Kalan

- Birden çok raster üst üsteyken yalnız en üsttekinin kartı gösteriliyor; ötekiler `cells`'te duruyor, kartta listesi
  yok.
- Uzak (COG, WMS) rasterlerde hücre değeri; bu adım yerel dosyalarındır.
- Web (TODOS.md §0.2).
