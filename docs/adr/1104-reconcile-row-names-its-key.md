# 1104 — Uzlaştırmanın çatışma satırı anahtarı ve katmanı söyler

Tarih: 2026-11-20. gis-usage-report.md GIS-047. Durum: kabul edildi.

İlgili: ADR 0533 (bağın anahtar alanı), ADR 0538/0553 (uzlaştırma).

## Sorun

Aynı 101 adasındaki dört parselden 6 numaralının Mahalle'si iki yanda farklı değişince karar satırı “Ada 101 ·
Mahalle …” diye başladı; tekil anahtar (Parsel = 6) yazmıyordu. Operatör doğru parsele baktığını pencereden
doğrulayamıyordu.

## Karar

Çatışma satırının adı önce bağın anahtar alanını ve değerini (`parsel_no 6`), sonra katmanın alan sırasıyla iki
özniteliği yazar, sonunda nesnenin katmanının adı parantez içinde (birden çok katman birlikte uzlaşırken de ayırır).
Anahtarı olmayan bağda eski düzen: ilk iki öznitelik; hiç değer yoksa kalıcı kimliğin başı (“kimlik …”).

## Sınır

Anahtarsız bağda sağlayıcının satır kimliği (fid) bu pencerede yazılmaz: nesnenin kalıcı kimliği satır kimliğinden
türetilir, geri çözülmez; kalıcı kimliğin başı ayırt etmeye yeter.

## Kanıt

`apps/desktop/src/shell/exchange/gpkg_tests.rs` `a_conflict_s_row_names_the_key_and_the_layer`: anahtar `parsel_no`
iken çatışma satırları “parsel_no 6 … (katman)” biçiminde.
