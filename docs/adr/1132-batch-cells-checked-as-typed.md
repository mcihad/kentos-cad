# 1132 — Toplu çalıştırmanın hücreleri yazılırken denetlenir

Tarih: 2026-11-20. gis-usage-report.md GIS-061. Durum: kabul edildi.

İlgili: ADR 0551 §1 (Toplu çalıştır, Hata olursa: Dur / Devam et).

## Sorun

İki satırlı tablonun ikinci uzaklığına “ölçülmedi” yazılınca açıklama, Devam et seçiliyken yalnız sorunlu satırın geri
alınacağını söylüyordu; oysa geçersiz hücre iki politikada da tabloyu çalışmadan durdurdu ve ilk satır da çıktı
üretmedi. Hata ancak çalıştırınca görünüyordu.

## Karar

1. `batch_table::problems` okunamayan her dolu hücreyi, `input`'un reddettiği sözlerle, tablo sırasıyla verir.
   Pencere bunu tablonun altında yazılırken gösterir: “Satır 2: “Uzaklık”: sayı bekleniyor; “ölçülmedi” yazılmış.”,
   başında hücre denetiminin tabloyu bütünüyle durdurduğu ve Dur/Devam et seçiminden bağımsız olduğu.
2. Okunamayan hücre varken Toplu çalıştır düğmesi kapalıdır.
3. Açıklama iki hatayı ayırır: okunamayan hücre tabloyu bütünüyle durdurur; Dur/Devam et çalışırken oluşan hatalar
   içindir.

## Kanıt

`apps/desktop/src/gis/processing/batch_tests.rs` `a_cell_that_cannot_be_read_is_said_as_it_is_typed`: Devam et
seçiliyken “ölçülmedi” tablonun altında satırı ve sütunuyla söylenir; 5 yazılınca sorun kalmaz; var olan “çalışmadan
durur” testi değişmeden geçer. Resim `.run/gui-evidence/cbs-kullanim/gis-061-toplu-hucre.png`.
