# 3020 — Sınır grafiği'nde tablodaki satırın çizimde öne çıkarılması

Tarih: 2026-11-09. plan/map-cad.md MAP-08, beşinci tur. Durum: taslak. ADR 2620'nin “Sonra”sındaki “tablodaki satırın
çizimde vurgulanması”.

İlgili:
- [ADR 2000](2000-parcel-boundary-graph.md): sınır grafiği (`cad.parcels.boundaries`), kimlikleri geometriden;
- [ADR 2220](2220-boundary-graph-on-the-drawing-and-arc-corners.md): Çizimde göster, grafiğin işaretleri;
- [ADR 2620](2620-boundary-arc-chord-mismatch.md): yay ile kiriş uyuşmazlığı.

## Dayanak

- **ArcGIS Pro, Validate and fix topology errors**
  ([belge](https://pro.arcgis.com/en/pro-app/3.6/help/editing/validate-and-fix-geodatabase-topology.htm), 9 Kasım
  2026'da okundu): Error Inspector'ın tablosunda seçili satırların hatalarına yakınlaştırılır, harita onların kutusunda
  ortalanır (Ctrl+Shift+=); yalnız ortalamak da vardır. Liste ile harita aynı hatayı gösterir.
- **KentOS bugün** (ADR 2220):
  - Çizimde göster bütün grafiği işaretleyip bütün köşelere yakınlaştırıyordu;
  - tablodaki bir satırın (bir sınır, köşe ya da sorun) çizimde nerede olduğunu bulmak için kullanıcı numarayı
    gözüyle arıyordu. Sorun sayısı büyüdükçe zorlaşır.

## Karar

1. **Satır seçimi:** pencerenin tablosunda satıra basılınca satır seçilir ve işaretlenir (kitin veri tablosu, seçili
   satır). Başka tabloya (Sınırlar, Köşeler, Sorunlar) geçmek seçimi kaldırır.
2. **Çizimde göster** seçili satırla:
   - görünüm satırın yerine gider: yerlerinin kutusu, en az grafiğin kutusunun onda biri genişliğinde, onda bir payla;
   - işaretler satırı vurgu renginde öne çıkarır: sınır eğrisi boyunca kalın çizgiyle (yay, iki binde birlik sapmayla
     noktalarla), köşe ve sorun çift halkayla. Yanında adı: sınırın ya da köşenin kimliği, sorunun türü;
   - satır kimliğiyle tutulur: sınır ve köşe kimliğiyle, sorun türü ve yeriyle. Çizim değişip grafik yeniden
     hesaplanınca yeniden bulunur, artık yoksa öne çıkarılmaz;
   - seçili satır yoksa eskisi gibi bütün grafik.
3. Yazılan bir şey yoktur; komut ve sözleşme değişmez. Kitte seçilebilir tablo `road_dialog::sheet_chosen`.
4. **Kapsam işareti:** ADR 2000'deki gibi **Harita (CAD) + CBS'nin tipli parsel katmanı**.

## Kanıt

- **Masaüstü** (`parcel_boundaries_window_tests.rs`, `the_chosen_row_is_brought_forward_on_the_drawing`):
  - sorunlu adada son sorun (çakışan kenar) seçilir; Çizimde göster onu yerinde tek noktayla ve adıyla öne çıkarır;
    görünümün kutusu yeri içerir ve grafiğinkinden dardır;
  - Sınırlar'da seçim, Köşeler'e geçince kalkar; ilk sınır seçilince eğrisi boyunca öne çıkar;
  - seçimsiz Çizimde göster bir şey öne çıkarmaz.

  ADR 2000/2220/2620'nin pencere testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/satir-cizimde-3020/`:
  seçili yay sınırıyla pencere ve çizimde öne çıkan S4.
- **MCP ve Python:** komut değişmedi.

## Sonra

- Aynı yerdeki birkaç sorunun sözleri üst üste biniyor (resimde görüldü; ADR 2220'nin yerleştirmesi): tek sözde
  birleşmeleri.
- Kalıcı parsel kaydı ve sınır kimliği (MAP-08'in ana açığı).
- Yayın kirişlerle yaklaşık çizildiği komşu (ADR 2620 Sonra).
- Web (TODOS §0.2).
