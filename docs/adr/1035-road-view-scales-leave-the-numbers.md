# 1035 — Görünüşün ölçeği hesabı değiştirmez

Tarih: 2026-11-07. Durum: kabul (Y100). Kapsam ROAD-SEC-06, ADR 0903 §7 (enkesit görünüşü), ADR 0908 §10 (paftalar),
ADR 0990 (kot taşıma).

## Sorun

ROAD-SEC-06, görünüşün yatay ve düşey ölçeğinin, yerinin ve yazılarının gerçek hesap değerlerini değiştirmemesini
istiyor. KentOS'ta abartma yalnız çizimdeydi, ama bunu sınayan bir durum yoktu (2026-11-05 denetimi: "ölçekler arası
değişmezlik durumu yok").

## Araştırma (7 Kasım 2026)

- **Civil 3D'nin profil ve kesit görünüşleri:** düşey abartma yalnız görünüşün özelliğidir. Profilin ve kesitin
  kotları, alanları ve hacimleri modelden gelir (ADR 0990 ve 0903'ün kaynakları).
- **KGM paftası:** boykesit 1:2000 yatay, abartmalı düşeyle; enkesit 1:200 ya da 1:100 çizilir. Paftada yazılan
  kotlar ve alanlar ölçekten bağımsızdır; kot taşıma (ADR 0990) yalnız çizginin yerini değiştirir.
- **Matematik:** hesap yok. Görünüş, hesap değerlerinin çizim koordinatlarına bir eşlemesidir; değişmezlik aynı
  değerlerin her eşlemede aynı yazılmasıdır.

## Karar

- **Kod değişmedi.** Değişmezlik üç sınamayla tutulur (`tests/all/road_view_scales.rs`):
  - **Enkesit görünüşü:** abartma 1 ile 5, satırda 5 ile 3 görünüş, 2,5 ile 3,5 mm yazı, başka bir yer. Km, kıyas
    ve tepe kotu, yarma ve dolgu alanı ile ızgara aralıkları aynı; çerçeve farklı; yazılan her yazı aynı.
  - **Boykesit görünüşü:** abartma 10 ile 20, 2,5 ile 3,5 mm yazı, başka bir yer. Bantların km'leri, kırmızı
    kotları, eğimleri ve eğrilikleri ile kıyas kotu, tepe ve ızgara aynı; yazılan her yazı aynı.
  - **Enkesit paftaları** 1:500 ile 1:1000: km, eksen kotu ve alan yazıları aynı.
- **Paftada ızgara etiketi ayrımı:** bir paftanın görünüşleri o paftadaki en yüksek kot aralığını paylaşır. 1:1000'de
  bütün kesitler bir paftaya sığar, çerçeveler uzar ve ızgara daha çok kot yazar. Fazla etiketler yalnız kottur;
  1:500'dekilerin hepsi 1:1000'de de var. Bu bir hesap değişikliği değil, çerçeve boyudur.
- **Resim** yok: arayüz değişmedi.

## Ölçüm

- **Enkesit:** 8 görünüş, 176 yazı aynı.
- **Boykesit:** 15 bant sütunu, 98 yazı aynı.
- **Enkesit paftaları:** 24 hesap yazısı (km, eksen kotu, yarma ve dolgu) aynı; 1:1000'de 8 kot etiketi fazla.
- **İlk deneme:** paftada bütün yazıları eşit bekleyen sınama 1:1000'deki 8 fazla kot etiketiyle düştü ("106.00",
  "107.00", "108.00"). Nedeni paylaşılan çerçeve boyuydu; sınama hesap yazıları ve ızgara etiketleri diye ayrıldı.
