# 3040 — Yol genişliğine giren kısımların çizimde taranması ve parsel başına toplam

Tarih: 2026-11-09. plan/map-cad.md MAP-09, beşinci tur. Durum: taslak. ADR 2640'ın “Sonra”sındaki “parselin yola giren
kısmının sahnede taranması; parsel bazında toplam (birden çok yoldan)”.

İlgili:
- [ADR 2240](2240-zoning-decision-inside-road-width.md): yol genişliğine giren karar (`roadWidth`);
- [ADR 2640](2640-zoning-parcel-inside-road-width.md): yola giren parsel (`roadParcel`);
- [ADR 2260](2260-scenario-access-on-the-drawing.md): Erişimi çizimde göster'in işaret deseni (İşareti kaldır'a dek kalan,
  çizim değişince yeniden hesaplanan işaret).

## Dayanak

- **ArcGIS Pro** (ADR 3020'nin ve 2640'ın okumaları): Error Inspector seçili satırların hatalarına gider; “Must Not
  Overlap With” kuralının hatası iki katmanın nesnelerinin bindiği yerdir. Hata, yeriyle haritada görülür.
- **KentOS bugün** (ADR 2240, 2640):
  - denetim, yol aksının iki yanında genişliğin yarısı kadar şeride giren kararları ve parselleri alanlarıyla buluyordu;
    bölgeyi çekirdeğin kuralı (`topology_rules`) hesaplıyor, komut yalnız alanı veriyordu;
  - Seç ve göster bulgulardaki nesneleri seçip yakınlaştırıyordu. Hangi kısmın yola girdiği görünmüyordu;
  - bir parsel iki yola giriyorsa iki ayrı satırdı, toplamı söylenmiyordu.

## Karar

1. **Bölgeler** (`zoning_check::with_regions`, masaüstünün okuduğu yan işlev): denetimin cevabıyla birlikte her bulgunun
   bölgeleri. Yol genişliği ve yola giren parsel bulgularında çekirdeğin kuralının örtüşme bölgeleri, öbür türlerde boş.
   Komutun cevabı değişmez.
2. **Çizimde taranması:**
   - pencerenin Seç ve göster'i (bulgular grubunda yol genişliği ya da yola giren parsel varsa) bu bölgeleri çizimin
     üstünde tehlike renginde, açık dolgu ve 45° tarama ile işaretler;
   - her bölge kendi başına çizilir: aynı yerdeki iki bölge (yolun içindeki karar parçası ve parsel parçası) birbirini
     silmez. İlk resimde çift-tek kuralı tüm bölgelere birden uygulanınca alttaki blok taranmamıştı; düzeltildi;
   - işaret İşareti kaldır'a (`data.unmark`) ya da başka çizime dek kalır;
   - çizim değişince aynı girdiyle yeniden hesaplanır: yoldan çekilen parselin taraması kalkar;
   - ileti kaç kısmın tarandığını söyler.
3. **Parsel başına toplam:** özet satırı yola giren her parseli yol genişliklerindeki toplam alanıyla, büyükten küçüğe
   sayar; birden çok yola girende “(2 yoldan)”. En çok altı parsel yazılır, kalanı sayıyla.
4. Yasal sonuç ya da resmî uygunluk iddia edilmez (ADR 2640).
5. **Kapsam işareti:** ADR 2640'taki gibi **Harita + CBS**.

## Kanıt

- **Masaüstü** (`zoning_window_tests.rs`, `a_parcel_in_two_roads_is_summed_and_the_parts_hatched`): ortak durumun
  çizimine 101/2'yi kesen üçüncü bir yol (Y3, 10 m) eklenir:
  - toplamın ilki 101/2: Y1'den 25, Y3'ten 250 m², 275 m², iki yoldan;
  - Seç ve göster bölgeleri tarar, toplam alan 275 m²'den büyüktür (kararların parçaları da); İşareti kaldır sunulur ve
    kaldırır.
- **Tarama** (`zoning_marks` birim testi): delikli karede tarama çizgisi çift-tek kuralıyla iki parçaya kesilir.
- `cad.zoning.check`'in 46 ortak durumu aynen geçer (komutun cevabı değişmedi). İmar penceresi testleri ve çapraz
  denetimler geçer. Resimler `.run/gui-evidence/yola-giren-3040/`.
- **MCP ve Python:** komut değişmedi.

## Sonra

- Bölgelerin komutun cevabına girmesi (Python ve MCP'de çizilebilir halkalar; ortak durumların üreteciyle). Yapıldı:
  [ADR 3440](3440-zoning-regions-in-the-answer.md).
- Plan değişikliği taslağı; sembolün karardan seçilmesi; resmî şema ve eşleme (MAP-09'un öbür maddeleri).
- Web (TODOS §0.2).
