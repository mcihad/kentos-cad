# 1103 — Paket eksik bağlı dosyayı söyler, eksikken “bütün” demez

Tarih: 2026-11-20. gis-usage-report.md GIS-046. Durum: kabul edildi.

İlgili: ADR 0596 (Paketle), ADR 2780 (devir dosyaları), ADR 1111 (kaynak raporu).

## Sorun

Bir parsele bağlı iki PDF'ten biri eksikken paket yapıldı; manifest onu `why: missing` diye yazdı ama sonuç iletisi
“Klasör başka bilgisayarda açılınca … kaynaklarını bulur” dedi. Eksik, paket açılınca durum çubuğunda anlaşıldı.

## Karar

1. Paketin bıraktığı bağlantılardan bulunamayan (`Missing`) ve okunamayanlar (`Unreadable`) ayrı sayılır. Hiçbiri
   yoksa ileti eskisi gibi kaynakların bulunacağını söyler; varsa “Paket eksik: n bağlı dosya bulunamadığı ya da
   okunamadığı için girmedi; aşağıda tek tek.” der ve düzey Uyarı'dır.
2. Her eksik dosya ayrı satırda: yolu, nedeni (bulunamadı/okunamadı), onu tutan nesne (etiketi ve katmanı; nesne değilse
   başvurunun adı) ve yol: dosyayı o yola koymak ya da eki yeniden bağlamak, sonra yeniden paketlemek.
3. Veritabanı tablosu, adres, yerine yazı tipi, tutulan ve dışarıda bırakılan bağlantılar eksik sayılmaz (paketin
   bilerek dışında kalırlar; zaten tek tek söylenir).

## Kanıt

`apps/desktop/src/shell/exchange/package_tests.rs` `a_missing_document_is_said_and_the_package_is_not_called_whole`:
eksiksiz pakette “kaynaklarını bulur”; `belgeler/tapu-101-5.pdf` eksikken bu cümle yok, “Paket eksik: 1 bağlı dosya”
ve dosyanın yolu, nesnesi ve yeniden bağlama yolu ayrı satırda.
