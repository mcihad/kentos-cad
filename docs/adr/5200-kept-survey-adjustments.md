# 5200 — Dengelemenin sonucu projede adıyla saklanır

Tarih: 2026-11-11. plan/map-cad.md MAP-18, onuncu tur. Durum: taslak. ADR 4400 ve 4820'nin “Açık”ındaki “total station ve
nivelman kalitesi”nin ilk adımı: “önce dengeleme doğruluklarının saklanması; `.kcad` bölümü”.

## Dayanak

- **BÖHHBÜY** m. 98(1) (`.run/research/map-cad/mevzuat/MAP-18.9.md`, 10 Kasım 2026): teslimde ölçü çizelgeleri,
  dengeleme ve hesap sonuçları, koordinat özet çizelgesi bulunur. Ölçü kalitesinin paftaya yazılması istenmez; paftadaki
  kalite tablosu kullanıcının seçimidir. Önce sonuçların saklanması gerekir: pafta onları bir yeni dengelemeden değil
  kayıttan okur.
- **Benzer uygulamalar** (`.run/research/map-cad/uygulama/MAP-18.9.md`): dengeleme sonuçlarının projede kalıcı kaydı
  (nokta başına σ'lar, elips, güven düzeyi; ağ için σ0, χ² sonucu, serbestlik derecesi), kaydın girdinin sürümüne bağlı
  olması.

## Karar

1. **`cad.survey.adjust`'ın `keep`'i**: dengelemenin sonucu verilen adla (en çok 120 karakter) projede saklanır
   (`settings.surveyAdjustments`): adı, türü (`horizontal`: Yatay ağ, `height`: Kot ağı), yapıldığı çizim sürümü, yeni
   noktaları (adı, yeri ya da kotu, σ'sı), f, m0 ve model testi. Aynı adlı kayıt yenisiyle değişir. Kayıt, noktaların
   yazılmasıyla aynı geri alma adımındadır; cevapta `kept`. Boş ad `invalid_input` (`keep`).
2. **Alan modeli:** ayarlardaki kayıtlar geri alınır (`Op::SurveyAdjustments`).
3. **`.kcad` şema 244**: `surveyAdjustments`; 243 ve öncesi `unknown_field`; boş liste yazılmaz; bağımsız okuyucu
   (`tools/kcad/kcad.py`) kuralları denetler. Bilinmeyen sürüm örneği `broken/schema-version-245.kcad`.
4. **Masaüstü:** Yatay ağ ve Kot ağı dengelemesi pencerelerinin alt satırında **Projede sakla** (kaydın adı; boşsa
   saklanmaz); ileti “sonuç “…” adıyla projede saklandı”.
5. **Kapsam:** dengeleme pencereleri olan her çalışma alanı.

## Sınırlar

- Paftadaki kalite tablosu ve atlas sayfasında gösterim bu turda yapılmadı.
- Kayıt elips eksenlerini ve güzergah kapanmalarını tutmaz (poligon hesabı ayrı komuttur); yalnız ağ dengelemesinin
  cevabındakileri.
- Kayıt girdinin sürümüne bağlıdır ama eskime denetimi yoktur.

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/survey_adjust_command_cases.py`): 30 durum (4 yeni: yatay, kot ve ağırlıklı
  bilinenli kayıt, boş ad reddi); kayıttaki sayılar cevabın kendisidir (cevapta 1e-9 ile denetlenir, ayarda yerleri).
  Rust ve Python geçer.
- **Python** (`python/tests/test_kept_adjustment.py`): kaydın noktaları, f, m0 ve testi cevapla birebir; bağımsız okuyucu
  okur; aynı ad değişir.
- **`.kcad`:** `kept-adjustments.kcad` (+ JSON), dört bozuk örnek; `kcad_v2_reference.py --check` 1818 dosyada tutarlı;
  kcad testleri.
- **Masaüstü** (`calc::network::tests::projede_sakla_keeps_the_adjustment_with_the_write`): pencerenin sonucu kayıtta,
  tek geri alma ikisini birden alır; çapraz denetimler 214 test. Resim `.run/gui-evidence/dengeleme-kaydi-5200/`.

## Sonra

- Paftada kalite tablosu (güzergah, kapanma, sınır, durum) kayıttan; atlas sayfasında; kaydın eskimesi.
- Web (TODOS §0.2).
