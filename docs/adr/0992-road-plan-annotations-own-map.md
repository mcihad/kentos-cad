# 0992 — Plan yazıları yalnız gösterdikleri haritada

Tarih: 2026-11-07. Durum: kabul (Y86). Kapsam ROAD-SEC-11, ADR 0984 (plan haritası, some kutusu) ve ADR 0991'in bulgusu.

## Sorun

Yol paftasının iki haritası da aynı çizimin kopyasıdır ve kopyaya aks yazıları eklenir (ADR 0982): aks, km,
some kutusu, PI üçgeni, PROJE BAŞI/SONU. Y78'den beri PI'yı göstermeyen harita kutuyu kendine çekmiyordu (ADR 0984),
ama kutuyu PI'nın yanında bırakıyordu.

Y85'in resminde (KGM A1, 1:2000) bu bırakılan kutu (S-D, S-X, S-Y) ve PROJE SONU yazısı boykesit haritasında, şev ve
platform satırlarının üstünde göründü. Nedeni şu: plan nesneleri modelde yok, yalnız kopyada var. Kutu ve uç yazısı
çizimin kutusunun dışına taşar. Boykesit görünümü çizimin hemen altına inince (ADR 0991) boykesit haritasının
penceresine girdiler.

## Araştırma (7 Kasım 2026)

- **KGM Haziran 2024 örnek paftaları** (ADR 0984): her some kutusu PI'sının bulunduğu paftanın plan haritasındadır.
  Boykesit haritasında plan yazısı yoktur. PROJE BAŞI/SONU yalnız ucun bulunduğu paftada yazılıdır (s. 8 başta, s. 12
  sonda).
- **Civil 3D:** plan ve profil görünümleri ayrı nesnelerdir; hizalama etiketleri plan görünümündedir, profil
  görünümünde çıkmaz.
- **Matematik:** yok. Haritanın döndürülmüş dikdörtgeninin bir noktayı içerip içermediği (ADR 0984'teki aynı sınama).

## Karar

- **`Room::holds(p, turn)`:** haritanın, içeriği `turn` derece döndürülmüş dikdörtgeninin `p`'yi içerip içermediği.
- **Some kutusu** yalnız PI'sını içeren haritada çizilir. İçermeyen haritada kutu hiç çizilmez; önceki kural
  "PI'nın yanında bırak, harita keser" idi.
- **PROJE BAŞI/SONU** (çizgi ve üç satır) yalnız ucu içeren haritada.
- **Aks, km, PI üçgeni ve ana nokta yazıları** değişmedi: aksın üstündedirler, harita onları kendi sınırında keser.
- **Room verilmezse** (pafta dışı kullanım) her şey eskisi gibi çizilir.

## Ölçüm

- **Test** `a_map_that_does_not_show_the_pi_has_no_box` (eski `…_leaves_its_box_by_the_pi`'nin yerine):
  - Uzak haritada "D =", "S-Y =" ve "PROJE" yazısı yok.
  - Her şeyi gösteren haritada S-Y ve PROJE BAŞI var.
- **Testler:** uygulama 141 + 521, plot 86, masaüstü 1856.
- **Resim** `.run/gui-evidence/yol-y86/y86-satirlar-temiz.png`: Y85'in paftası. Boykesit haritasının satırları
  temiz; plan haritasındaki S-1 kutusu yerinde.

## Açık

- **Ardışık plan paftaları:** bindirme bölgesindeki bir PI'nın kutusu iki paftada da çizilir. KGM örneğinde tek
  paftada mı yazıldığı ayrıca okunmalı.
