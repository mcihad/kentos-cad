# 1119 — Kaynaklar araması hazır altlıkları kapalı dallarda da bulur

Tarih: 2026-11-20. gis-usage-report.md GIS-060. Durum: kabul edildi.

İlgili: ADR 0599 §5 (Kaynaklar paneli araması).

## Sorun

Servisler ve Hazır altlıklar dalları kapalıyken “Ortofoto” araması eşleşme bulmadı (“yalnız açılmış satırlarda
aranır”); dal elle açılınca aynı arama HGM ATLAS Ortofoto kaydını buldu.

## Karar

Arama yazılıyken panelin okumadan bilinen üst dalları açık sayılır: KentOS, Servisler, Hazır altlıklar, Kayıtlı
servisler, eklentilerinkiler, Veritabanları ve Klasörler (adlarıyla). Eşleşen satır üst dallarının yoluyla gösterilir
(var olan atalar kuralı). Klasörlerin ve sunucuların henüz okunmamış içeriği aranmaz; not satırı bunu söyler. Arama
temizlenince dallar kullanıcının bıraktığı gibi.

## Kanıt

`apps/desktop/src/gis/sources/tests.rs` `a_ready_base_map_is_found_in_closed_branches`: bütün dallar kapalıyken ilk
hazır altlığın adı “Hazır altlıklar” yoluyla bulunur, arama silinince dal kapalı kalır; var olan arama testi yeni not
metniyle geçer.
