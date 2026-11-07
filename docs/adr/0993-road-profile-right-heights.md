# 0993 — Boykesit çerçevesinin sağında da kotlar

Tarih: 2026-11-07. Durum: kabul (Y87). Kapsam ROAD-SEC-11 ve ADR 0990 §Açık ("sağ kenarda kotlar").

## Sorun

Boykesitin kot ölçeği yalnız çerçevenin solundaydı. Uzun bir görünümün sağ ucunda kot okumak için gözün paftanın
öbür ucuna gitmesi gerekiyordu. Kot taşımadan sonra (ADR 0990) soldaki ölçek yalnız ilk parçanınkidir; son parçanın
kotları yalnız kırılma çizgisinin yanında, çerçevenin ortasında kalıyordu.

## Araştırma (7 Kasım 2026)

- **KGM Haziran 2024 örnek paftaları:**
  - s. 9: 875–900 hem solda hem sağda.
  - s. 8: kot taşımalı boykesitte sağda son parçanın ölçeği (990–1015), solda ilkininki (955–980).
  - s. 10–12 aynı.
- **KGM 2019 teknik şartnamesi:** ölçeğin yerini yazmaz.
- **Matematik:** yok.

## Karar

- **`road.profiles.draw`:** son parçanın (kot taşıma yoksa tek parçanın) kotları çerçevenin sağına da yazılır.
  - Yer: sağ kenarın bir yazı yüksekliği dışı, sola yaslı; soldakinin aynası.
  - Maskesizdir; çerçevenin dışında, üstünden çizgi geçmez.
- **Nesne sayısı:** her çizimde son parçanın kot sayısı kadar artar. Bağımsız başvuru (`road_profile_command_cases.py`)
  aynı sayıyı ekler. Boykesit, pafta ve proje durumları yeniden üretildi (komut girdisi ve çıktısı değişmedi).

## Ölçüm

- **Ortak durumlar:** `road.profiles.draw` 56, `road.sheets.plan` 36, `road.project.*` yeniden üretildi; bütün `road.`
  durumları geçti.
- **Resim** `.run/gui-evidence/yol-y87/y87-sag-kenar.png`: Y84'ün KGM A1 paftası. Sağda son parçanın 108–120
  kotları; PROJE SONU yazısı çerçevenin içinde kaldı, çakışma yok.
