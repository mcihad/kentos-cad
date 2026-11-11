# 5160 — Hat profilinde kesişen borunun taraması seçilir

Tarih: 2026-11-11. plan/map-cad.md MAP-16, onuncu tur. Durum: taslak. ADR 4780'in “Sonra”sındaki “taramanın türünün
seçilmesi (iç alan, bütün boru, yalnız duvar)”.

İlgili: [ADR 2760](2760-crossing-pipes-in-the-pipe-profile.md) (kesişen borular), [ADR 4360](4360-crossing-pipe-outer-wall-and-live-check.md)
(dış duvar ve halka), [ADR 4780](4780-pipe-outer-diameter-own-walls-and-sewer-rules.md).

## Dayanak

- **Civil 3D**, “Outside Pipe Walls” ve profil görünüşünde borunun tarama seçenekleri (ADR 4360 ve 4780'in araştırması,
  `.run/research/map-cad/uygulama/MAP-16.9.md`): taramanın borunun iç alanına, bütün boruya ya da yalnız duvarına
  uygulanması.
- Yönetmelikler (Atıksu Toplama ve Uzaklaştırma Sistemleri, Yağmursuyu) profilde taramanın biçimini düzenlemez: seçim
  çizenindir.

## Karar

1. **`cad.pipes.profile`'ın `crossing.hatch`'i** (`CrossingHatch`): `wall` (öntanım; iki duvarın arasındaki halka,
   et bilinmiyorsa tarama yok: bugünkü davranış), `inside` (iç duvarın içi), `whole` (dış duvarın içi; et yoksa iç
   duvarınki), `none` (tarama yok). Duvarların çizgileri her seçimde çizilir. Taramanın sınırı elipslerin kendisidir.
2. **Masaüstü** (Boru denetimi › Hat profili, Diğer ağ seçili ve kesişen borular açıkken): **Kesişen borunun
   taraması** satırı (Yalnız duvar, İç alan, Bütün boru, Taramasız).
3. **Kapsam işareti:** ADR 1640 gibi **Harita + Yol + CBS**.

## Sınırlar

- Hattın kendi boruları taranmaz (ADR 4780). Taramanın deseni dolgu; desen seçimi yok.
- Yol kaplamasının profilde gösterimi bu turda yapılmadı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/pipes_profile_cases.py`): 65 durum (6 yeni: etli boruda dört seçim, etsiz
  boruda bütün boru ve yalnız duvar), çizilen nesnelerin sayısı seçime göre. Rust geçer.
- **Masaüstü** (`pipes_check_window_tests::the_crossing_pipes_fill_is_chosen`): Bütün boru ortak durumun nesne sayısını
  ve her kesişen boruya tek döngülü bir tarama yazar; çapraz denetimler 157 test. Resimler
  `.run/gui-evidence/kesisen-boru-taramasi-5160/`.
- Python ve MCP ayrıca yazılmadı (komut aynı; yeni alan şemadan gelir).

## Sonra

- Yol kaplamasının (hendeğin üstündeki yol ve kaldırım kaplaması, EK-1 §1.3.6'nın tip katmanları) profilde gösterimi.
- Uç çizgisi; hidrolik (hız, doluluk), EPANET/SWMM.
- Web (TODOS §0.2).
