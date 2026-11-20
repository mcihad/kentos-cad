# 1106 — Kayıtları denetle hata nedenini pencerede tutar

Tarih: 2026-11-20. gis-usage-report.md GIS-049. Durum: kabul edildi.

İlgili: ADR 0406 (Kayıtları denetle), ADR 0441 (form tasarımcısı).

## Sorun

Kuralı okunamayan formda pencere yalnız “Denetim yapılamadı” dedi; neden (kural ifadesinin 12. karakterinde
beklenmeyen kapanış parantezi, tasarımcıda düzeltin) yalnız komut geçmişindeydi. Yeniden denetle aynı sonucu verdi.

## Karar

`cad.forms.audit`'in reddi pencerenin durumunda saklanır ve sözüyle gösterilir (formun adı, kural, konum, düzeltme).
Yanında “Form tasarımcısında aç”: pencere kapanır, form tasarımcısı o formla açılır. Yeniden denetle düzeltilen
kuralla sayıları (denetlenen, hatalı, uyarılı) gösterir; boş katman (“0 kayıt denetlendi”), hatasız veri (“Bütün
kayıtlar formun kurallarına uyuyor”) ve geçersiz kural birbirinden ayrı söylenir.

## Kanıt

`apps/desktop/src/gis/forms/audit.rs` `a_rule_that_cannot_be_read_is_said_in_the_window`: bozuk kurallı formda neden
pencerede (form adı ve karakter konumu), Form tasarımcısında aç tasarımcıyı açar. Resim
`.run/gui-evidence/cbs-kullanim/gis-049-form-denetim.png`.
