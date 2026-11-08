# 2560 — GNSS ağında noktaların güven bölgesi (hata elipsi)

Tarih: 2026-11-08. plan/map-cad.md MAP-05, dördüncü tur. Durum: taslak. Plan satırı: “Analiz sonucu önce fark vektörü,
artık, kontrol noktası etkisi ve güven bölgesiyle gösterilsin.”

İlgili:
- [ADR 1740](1740-gnss-network-from-archive.md): arşivden GNSS ağı, iki aşama;
- [ADR 1380](1380-gnss-baseline-network-adjustment.md): çekirdeğin GNSS dengelemesi (beyazlatma, istatistik);
- [ADR 2160](2160-network-similarity-article-15-1-c.md): Madde 15(1)(c) benzerliği.

## Neden bu parça

MAP-05'in “Sıradaki”lerinden robust kestirim ve ağırlıklı benzerlik bu turda yapılmadı:
- robust kestirimin üretici belgelerinde (Trimble, Leica) karşılığı bulunamadı. Literatür, kirli gözlemin iç
  güvenirliği düşükse yanılttığını söylüyor. Plan da aykırı gözlemin otomatik silinmemesini istiyor; ayrı bir
  araştırmayla tasarlanacak;
- ağırlıklı benzerlik için serbest aşamanın kovaryansı gerekir. Serbest aşama bir noktayı sabit tutar; o noktanın
  kovaryansı sıfırdır, yani datuma bağlıdır (S-dönüşümü gerekir).

Güven bölgesi planın açık bir isteğidir ve istasyon kovaryansından doğrudan çıkar.

## Dayanak

- **Trimble Business Center yardımı, “error ellipse”** ([sözlük](https://help.fieldsystems.trimble.com/tbc/2367.htm),
  8 Kasım 2026'da okundu):
  - hata elipsi, bir noktanın konum hatasının bütün hataların yayılmasıyla hesaplanan istatistiksel ölçüsüdür. Büyük
    ve küçük yarı eksenleri ve kovaryansıyla verilir;
  - koordinat hata elipsi, dengelenmiş noktaların hatasının büyüklüğünü ve yönünü gösterir;
  - iki boyutlu hata bir standart sapmada %39,4 olasılıktır. Sözlük %95 için “2,1447 × standart sapma” yazar. İki
    boyutlu normal dağılımda %95 çarpanı √χ²₂(0,95) = √(−2 ln 0,05) = 2,4477'dir; sözlükteki sayı rakamları yer
    değiştirmiş gibi duruyor. KentOS dağılımın değerini kullanır.
- **BÖHHBÜY Madde 15(1)(d)** (ADR 1740'ın okuması): istatistik güven 1 − α = 0,95.
- **KentOS bugün:** ağın cevabı yeni noktaların yalnız σ kuzey, doğu, yukarı'sını veriyordu. Kovaryansın köşegen
  dışı terimleri kayboluyordu; yön ve güven düzeyi yoktu.

## Karar

1. **Çekirdek** (`survey::adjust::gnss`): her dengelenmiş istasyon doğu, kuzey, yukarı kovaryansını da verir
   (`cenu`: R·Q·Rᵀ·m0²). σ'lar eskisi gibi bu matrisin köşegeninden çıkar, sayıları değişmedi.
2. **Güven bölgesi** (`NetworkStation.ellipse`, `ErrorEllipse`). Bilinen noktalara dayalı aşamanın sabit tutmadığı
   her işaret için verilir:
   - güven: sınıfın güveni, sınıf yoksa 0,95 (Madde 15(1)(d));
   - yatay elipsin yarı eksenleri: kuzey-doğu 2×2 kovaryansının özdeğerlerinin kökleri çarpı √χ²₂(p);
   - büyük eksenin yönü: kuzeyden saat yönünde, ½·atan2(2σne, σnn − σee), [0°, 180°);
   - kotun aralığı: √χ²₁(p)·σu (0,95'te 1,96 σ);
   - yön, eksenler büyük eksenin milyonda birinden az farklıysa verilmez: dairede yön yuvarlamadan ibarettir.
3. **Masaüstü** (Arşivden ağ dengelemesi):
   - özette “Güven bölgesi (%95): en büyük hata elipsi … noktasında, yarı eksenleri … ve … mm; kotun aralığı ±… mm.”;
   - Noktalar tablosunda **Güven bölgesi a/b, yön, kot (mm)** sütunu. Yön projenin açı biriminde semttir; dairede
     “yönsüz” yazar.
4. **Kapsam işareti:** ADR 1740'taki gibi **Harita + CBS** (`map.survey.network` ikisinde de salt okunur).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_survey_network_cases.py`, KentOS kodu yok.
  - Elips numpy'nin özdeğer ayrıştırmasından, yön büyük özvektörden, çarpanlar scipy'nin χ²'sinden. Rust'taki kapalı
    biçimden bağımsızdır.
  - Yeni durum: bazların doğu ve kuzey sapmaları 1,5 ve 4 mm, eksenleri 30° dönük. Elipsler o yöne uzanır: N1, N4 ve N5
    30,00°, N5'te 3,5 × 1,3 mm, kot ±3,5 mm.
  - Var olan durumlarda elipsler daire gibidir (eksenler 10⁻¹⁰ m farklı), yön verilmez.
  - 42 durum (41'den). Rust ve Python geçer; arşiv durumları değişmedi.
- **MCP** (`crates/native/mcp/tests/network_confidence_region.rs`): yeni durumun çizimi `.kcad` olarak açılır, her
  noktanın elipsi durumunkidir.
- **Masaüstü** (`survey_region_tests.rs`):
  - her yeni noktanın elipsi durumunkidir;
  - özet N5'i söyler;
  - sütun hücresi semtiyle çizilir; temel durumda yön yoktur.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/guven-bolgesi-2560/`.

## Sonra

- Elipslerin çizimde gösterimi (ölçekli, katmanda).
- Robust kestirim (aykırı gözlem silinmeden, ağırlık değişimi raporda).
- Ağırlıklı benzerlik (S-dönüşümüyle serbest ağ kovaryansı).
- Seyrek çözüm ve büyük ağ.
- Web (TODOS §0.2).
