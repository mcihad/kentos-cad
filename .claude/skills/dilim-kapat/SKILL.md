---
name: dilim-kapat
description: KentOS'ta bir dilimi (plan maddesini, düzeltmeyi, refactor adımını) kapatırken izlenecek sıra — biçim, hedefli sınama, bekçiler, belgeler, commit. Bir iş bitti denmeden ve commit atılmadan önce kullanın.
---

# Dilimi kapatma

Bir dilim, sırasıyla aşağıdakiler yapılmadan bitmiş sayılmaz. Koşulmayan adım ve nedeni kullanıcıya söylenir.

1. **Biçim.** `cargo fmt --all`. Dosyayı sınırın altında tutmak için satır sıkıştırılmaz; dosya bölünür (ADR 9400 §3).
2. **Hedefli sınama.** Yalnız dokunulan crate, tek koşu, ağır iş kilidiyle:
   `scripts/dev/svc.sh heavy cargo test -p <crate> <süzgeç> [<süzgeç> …]`. Birden çok süzgeç tek koşuda verilir; tam
   takım ve arka arkaya uzun koşular yok.
   - Bütünleşik sınamalar tek programdadır: `--test all <modül>::`.
   - Masaüstüne dokunan dilim ayrıca şu süzgeçleri koşar: `module_tests ribbon_tests history::text catalog settings
     other_names source_size shell_boundary` (`-p kentos-desktop --lib`); modül paketine dokunan o paketin sınamalarını.
   - Sunucunun veritabanı sınamaları için `sunucu-db-sinamasi` skill'i.
3. **Komut, araç ya da ayar değiştiyse.** Tarif `docs/kilavuz/09-yeni-is-tarifleri.md` §9.2'dedir; üretilenler:
   `make catalog`, `python3 scripts/python/sdk.py`, `python3 scripts/ui/ribbon.py`, `python3 scripts/cad/manifest.py`.
   Modül × kanal karar tablosu işin ADR'sine yazılır.
4. **Bekçiler.** `make guard` yeşil (bağımlılık yönü, kaynak boyu, modül bağları, CAD manifesti, biçim).
5. **Arayüz değiştiyse resim.** İlgili `screens` sınaması, resim `.run/gui-evidence/` altında; resme yakından bakılır,
   kusur düzeltilir, yeniden çekilir.
6. **Belgeler.** İşin ADR'si; plan maddesi `[x]` ya da `[~]` (kalanıyla); `docs/kilavuz/00-guncel-karar-ozeti.md`'ye bir
   madde; gerekiyorsa `TODOS.md`.
7. **Commit.** `git status` ile `??` satırlarına bakılır; yalnız bu dilimin dosyaları eklenir (`git add -A` yok).
   Başlık İngilizce, ne yapıldığını söyleyen tek cümle, sonunda madde ve ADR: `… (BK-10, ADR 9401)`. Refactor ile
   doğruluk düzeltmesi ayrı commit'tir; doğruluk düzeltmesi, düzeltmeden önce kırmızı olan bir sınamayla gelir.
