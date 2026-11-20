# 1108 — Veri profili tek geri almayla ayarlarını da geri alır

Tarih: 2026-11-20. gis-usage-report.md GIS-051. Durum: kabul edildi.

İlgili: ADR 0541/0558 (veri profili), ADR 0430 (değer listeleri, ilişkiler), ADR 8054 §3 (CBS ayarları geri alma
geçmişinde).

## Sorun

TUCBS temel belediye profili uygulanıp bir kez geri alınınca katmanlar gitti, ama dört değer listesi, iki ilişki ve
adres tanımı proje ayarında kaldı; ilişkiler (`layer-6 → layer-8`) ve adres tanımı artık olmayan katmanları gösteriyordu.
Pencere bunu önceden söylüyordu, yine de kayıtlı belgede geçersiz başvurular kalıyordu.

## Karar

1. Belgenin geri alma geçmişine iki CBS ayar adımı eklenir: `GisOp::Relations` ve `GisOp::AddressLocators` (önce ve
   sonra), değer listelerinin `GisOp::Domains`'i gibi. `Document::set_relations` ve `set_address_locators` açık işleme
   ya da gruba yazar, yoksa kendi adımıdır.
2. `cad.profiles.apply` (ve onu kullanan `cad.profiles.carry`) değer listelerini (`set_domains`), katmanları ve
   alanlarını, ilişkileri ve adres tanımını tek grupta yazar: tek geri alma hepsini geri alır, yinele hepsini getirir.
   Bir ret bütün grubu iptal eder (önceden ayarlar elle geri konuyordu). Başka işlemle önceden yazılmış tanımlar
   önceki durumun parçasıdır, geri almada korunur.
3. Pencerenin plan ve sonuç cümleleri “Hepsi tek adımda geri alınır” der.

## Kanıt

- Paylaşılan örnekler: `scripts/fixtures/profile_cases.py` ve `profile_carry_cases.py` geri almadan sonra projenin
  kendi ayarlarını bekler (üreteçler güncellendi, `--check` temiz); `cad.profiles.apply/carry/names` örnekleri geçer,
  paylaşılan oynatmada bilinen 41 dışında hata yok.
- `apps/desktop/src/tests/data_profile_tests.rs` `one_undo_takes_the_profile_s_settings_back_with_its_layers`:
  uygula, geri al (ilişkiler, adres tanımları, değer listeleri öncekiyle aynı), yinele (hepsi geri, her ilişki var olan
  katmanlarda).
