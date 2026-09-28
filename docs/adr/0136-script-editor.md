# ADR 0136: Python sekmesinde betik düzenleyicisi

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0132 (Python konsolu), ADR 0135 (tamamlama ve imza); TODOS.md PY-05.

## Bağlam

Konsol kodu satır satır çalıştırır. Birkaç düzine satırlık bir iş için insan betik yazar, dosyada saklar, dener, düzeltir, yeniden çalıştırır. Konsolun küçük giriş kutusu buna uygun değil.

## Karar

**Python sekmesinin iki yüzü vardır: Konsol ve Betik.** İkisinin çubuğunun başında aynı geçiş durur.

- Betik yüzünde solda betik ve çubuğu, sağda çıktı vardır.
- Çıktı iki yüzde ortaktır.
- Betik de konsolun Python'unda, aynı adlarla çalışır: `doc`, `cad` ve konsolda tanımlananlar.

**Betik.**

- **Görünüm:** eş aralıklı yazı ve Python renkleri (`python/code.rs`): anahtar sözcükler (vurgu, kalın), sabitler ve yerleşik işlevler, dizeler (üç tırnaklısı satırlar boyunca), sayılar, yorumlar, süsleyiciler, `def` ve `class`'ın verdiği adlar.
  - Renkleri çözümleyici kendisi hesaplar; Iced'in `highlighter` özelliği ve syntect eklenmedi.
- **Tuşlar:**
  - Enter, satırın girintisini korur; `:` ile biten satırdan sonra dört boşluk ekler.
  - Tab dört boşluktur.
  - F5 betiğin tamamını çalıştırır.
  - Ctrl+Enter seçimi, seçim yoksa imlecin satırını çalıştırır; seçimin ortak girintisi atılır.
  - Ctrl+S kaydeder, Ctrl+Shift+S farklı kaydeder.
- **Dosya:** Yeni, Aç… (`.py`), Kaydet, Farklı kaydet….
  - Kaydedilmemiş betiğin üstüne Yeni ya da Aç önce sorar: Kaydet, Kaydetme, Vazgeç. Soru betiğin üstünde bir satırdır; uygulamanın ortak penceresi çizimin sorusu için kalır.
- **Hata:** betik bütün çalıştırılıp hata verirse imleç betikte durduğu satıra gider ve o satır seçilir.
  - Ad, hata dökümünde betiğin dosyasıdır; adsız betik `<betik>`'tir.
  - Seçimin çalıştırması `<betik seçimi>` adını taşır. Satırları seçimin ilkinden sayılır, bu yüzden imleç oraya gitmez.
- **Taslak:** yazılan betik programın öbür geçmişinin yanında tutulur (`$XDG_STATE_HOME/kentos-cad/python-betik.json`). Metni, dosyası ve kaydedilmemiş olduğu yazılır. Uygulama kapanıp açılınca betik aynen gelir.
  - Okunamayan taslak boş betiktir.
  - Testlerde taslak yoktur (`main.rs` kurar).

Çalıştırmanın kuralları konsolunkidir (ADR 0132): tek geri alma adımı, hata ya da Durdur yazdıklarını geri alır, çalışırken çizim başka düzenleme almaz.

## Sonuçlar

- Betik yazmak, saklamak ve çalıştırmak masaüstünün içinde; konsol, tamamlama ve imza yardımıyla birlikte.
- **Açık kalanlar:**
  - betik düzenleyicisinde tamamlama (konsoldaki kutuda var);
  - satır numarası sütunu;
  - birden çok betik sekmesi;
  - betiği bir düğmeye ya da komuta bağlamak;
  - taslak birden çok pencerede son yazanındır.

## Doğrulama

- **`cargo test -p kentos-desktop python::`, 25 test** (4'ü gerçek Python'la):
  - satırların Python olarak okunması ve üç tırnağın satırlar boyunca açık kalması;
  - girinti kuralı, hata satırının bulunması, seçimin girintisi;
  - taslağın yazılıp geri gelmesi;
  - Yeni'nin kaydedilmemiş işte sorması (Vazgeç, Kaydetme);
  - kaydetme ve yeni satırın girintisi;
  - gerçek Python'la: betik çalışır, çıktısı gelir, hata satırına imleç gider (3. satır).
- **Resimler:** `python-konsol-betik-*`, iki tema ve iki boyut. Gösterilen gerçek bir çalıştırmadır: parsellerin alanı ve ağırlık merkezlerine nokta.
- **Genel:** `cargo test -p kentos-desktop` (528) ve clippy.
