# ADR 0427: Sürümleri karşılaştır

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AJ`; `plan/form.md` FORM-DESIGN-17 (sürüm karşılaştırması).
- **Bağlam belgesi:** ADR 0401 §9 (taslak ve yayınlar), 0403 (tasarımcı), 0411 (yayınların saklanması).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Form yayınlanmadan önce taslağın son yayından ne kadar ayrıldığı görülmüyordu; yayınlar arasındaki fark da sorulamıyordu.

## Karar

### 1. Kural

`kentos_forms::compare`: eski ve yeni gövdenin öğeleri kimlikleriyle eşlenir; her öğe **eklendi**, **kaldırıldı**, **taşındı** (başka
sekme ya da gruba, ya da iki sürümde de olan kardeşleri arasında başka sıraya) ya da **değişti** (kendi özellikleri, içindeki öğeler
hariç; değişen özellikler JSON adlarıyla `properties`'te, sözde Türkçe adlarıyla). Değişiklikler yeni sürümün sırasıyla, kaldırılanlar
eski sürümün sırasıyla sonda; ardından formun Gizlenenleri sil'i ve senaryoları.

### 2. Komut

`cad.forms.compare` v1 (salt okunur, “Sürümleri karşılaştır”): `form`, `from` (yoksa son yayın), `to` (yoksa taslak) → `from`, `to`,
`changes`. Retler: `unknown_form`, `form_unpublished` (karşılaştırılacak yayın yok), `unknown_release` (formun o sürümde yayını yok,
`from` ya da `to`'da). Python `kentos.cad.forms.compare`, MCP'de araç.

### 3. Tasarımcıda

Üçüncü sütunda **Özellikler | Önizleme | Değişiklikler**: taslağın son yayından farkı, her satır bir değişiklik (eklenen yeşil `+`,
kaldırılan kırmızı `−`, taşınan `↕`, değişen `•`); satıra basmak öğesini seçer. Yayınlanmamış formda “karşılaştırılacak yayın yok”.

## Denetimler

- `scripts/fixtures/forms_compare_cases.py --check`: 21 durum (`fixtures/commands/v1/cad.forms.compare.json`), kuralın bağımsız
  Python yazımıyla; `kentos-forms`'un `compare` testi.
- Masaüstü: `the_designer_says_what_changed_since_the_release`; resim `form-degisiklikler`.

## Sonuçlar

- Yayından önce neyin değiştiği bir bakışta; aynı fark Python ve ajanla sorulur.
- Açık (FORM-DESIGN-17'nin kalanı): taslağın kendiliğinden kaydı ve gözden geçirme yorumları.
