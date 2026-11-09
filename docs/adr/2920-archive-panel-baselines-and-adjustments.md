# 2920 — GNSS arşivi panelinde bazlar, bilinen noktalar ve dengelemeler

Tarih: 2026-11-09. plan/map-cad.md MAP-03, beşinci tur. Durum: taslak. ADR 2520'nin “Sonra”sındaki “arşiv panelinde
bazlar ve dengelemeler”.

İlgili:
- [ADR 1340](1340-gnss-campaign-archive.md) §6: GNSS arşivi paneli;
- [ADR 1740](1740-gnss-network-from-archive.md): arşivden GNSS ağı (bazlar `bz-N`, bilinen noktalar `bn-N`, dengeleme
  `da-N`), Ağ dengelemesi penceresi;
- [ADR 2520](2520-known-point-own-velocity.md): bilinen noktanın hızı.

Bu turun öbür sıradaki işi, GVX oturumlarının çapraz korelasyonları, korelasyonlu gözlemde bazın uyuşumsuzluk testi
için ayrı bir araştırma ve karar istiyor (ADR 2520). Sonraki tura kaldı.

## Dayanak

- **Trimble Business Center, Export GNSS Vector Exchange Files**
  ([belge](https://help.fieldsystems.trimble.com/tbc/26412.htm), 9 Kasım 2026'da okundu): dışa aktarılacak GNSS
  vektörleri Project Explorer'da ya da bir grafik görünümde seçilir. Vektörler projenin ağacında birer öğedir.
- **ADR 1740'ın okuması** (TBC, Leica Infinity): uyuşumsuz vektör dışlanır ve dengeleme yeniden yapılır; kabul edilen
  dengeleme noktaların son değeri olur.
- **KentOS bugün:**
  - `map.survey.query` bazları, bilinen noktaları ve kabul edilen dengelemeleri zaten veriyordu;
  - GNSS arşivi paneli yalnız arazi ölçümlerini, oturumları, işaretleri ve dayananları gösteriyordu;
  - bir bazı dışlamak ya da hangi dengelemenin hangi noktaları yaptığını görmek için Ağ dengelemesi penceresini açmak
    gerekiyordu.

## Karar

1. **Üç yeni grup** (İşaretler'in altında, Dayananlar'ın üstünde; kaydı olanlar):
   - **Bazlar:** satır “BN1 → N1 · bz-1”, bilgi uzunluk. Hap: Dışlandı, Dosyası yok, Dosyası değişti, Dengelemede ya da
     Arşivde. İpucu: σ X/Y/Z mm, dışlama nedeni, kaynak, dosya, onu kullanan dengelemeler;
   - **Bilinen noktalar:** satır “BN1 · bn-1”, bilgi epok. Hap: Sabit, Ağırlıklı ya da Dışlandı. İpucu: enlem, boylam,
     elipsoit yüksekliği, çerçeve, σ kuzey/doğu/yukarı, hız, kaynak;
   - **Dengelemeler:** satır “da-1 · C2”, bilgi baz ve nokta sayısı. Hap:
     - kullandığı bir baz ya da bilinen nokta sonradan dışlandıysa **Güncel değil** (dikkat), ipucu hangilerinin
       dışlandığını söyler;
     - sınıf tutmadıysa **Sınıf tutmadı**;
     - değilse **Kabul edildi**.

     İpucu iki aşamanın m0'ını ve serbestliğini, tutulanı, çarpanı, çerçeveyi, epoku ve sınıfı söyler.
2. **Satırlar:**
   - bazın satırı iki ucunun kabul noktalarını, bilinen noktanınki işaretinin kabul noktasını seçer;
   - dengelemenin satırı yaptığı noktaları seçer;
   - bazın ve bilinen noktanın düğmesi araç çubuğundaki nedenle dışlar ya da geri alır. `map.survey.edit`'in
     `exclude`/`include`'u, çözümlerdeki gibi, tek geri alma adımıdır; neden boşsa dışlanmaz, söylenir;
   - dengelemenin düğmesi Ağ dengelemesi'ni açar.
3. **Sözleşme değişmez:** satırlar `map.survey.query`'nin cevabından çizilir. Python ve MCP aynı alanları zaten okur.
4. **Kapsam işareti:** ADR 1340'taki gibi panel **Harita**'nındır; CBS kabul noktasını Öznitelikler'de okur.

## Kanıt

- **Masaüstü** (`map_survey_network_rows_tests.rs`, `map.survey.network`'ün ortak durumlarının çizimi):
  - Ağ dengelemesi'nden C2'de kabul edilen dengeleme Dengelemeler'de, satırı yaptığı üç noktayı seçer;
  - nedensiz dışlama yapılmaz; nedenle dışlanan bz-8 dengelemeyi “Güncel değil” yapar, ipucu bz-8'i söyler; Geri al
    düzeltir;
  - bilinen nokta da aynı yolla dışlanır ve geri gelir;
  - dengelemenin düğmesi Ağ dengelemesi'ni açar.

  Arşiv paneli, Ağ dengelemesi penceresi ve çapraz denetimler geçer. Resimler `.run/gui-evidence/arsivde-bazlar-2920/`.
- **MCP ve Python:** cevap değişmedi (yalnız masaüstü kodu değişti). GNSS ağının MCP testleri (`known_velocity`,
  `network_confidence_region`) aynen geçer.

## Sonra

- GVX oturumlarının çapraz korelasyonları ve korelasyonlu gözlemde bazın uyuşumsuzluk testi (araştırma ve karar).
- ~~Bazın satırından haritada bazın çizilmesi (iki uç arasında geçici çizgi).~~ ADR 3320.
- Yeni noktaların hızı (ADR 2520 Sonra).
- Web (TODOS §0.2).
