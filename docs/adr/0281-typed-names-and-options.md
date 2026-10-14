# ADR 0281: Yazılan adlar, kısaltmalar ve seçenekler

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-17`: “Araç seçenekleri fare, klavye ve yazılı komut
  yoluyla erişilebilir olsun; belirsiz kısaltmalar ve alias çakışmaları yakalansın.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0017 (komut satırı), 0147 §7 (yöntemlerin adları), 0164 (pafta kipinin komutları), 0280 (tutamaç kipleri).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Araçların seçenekleri fareyle (komut satırının seçenek düğmeleri) ve klavyeyle (harfleri) seçiliyordu; seçeneğin adını yazmak (“kopya”)
işlemiyordu. Komut satırı yazılan adı katalogda ilk bulduğu komutla çalıştırıyordu: iki komutun paylaştığı ad (pafta kipinin “Sil”i ile
çizimin Sil'i, İçe ve Dışa aktar menülerinin “DXF…” başlığı) sessizce ilkine gidiyordu; adın başı hiçbir şey çalıştırmıyordu.

## Karar

1. **Seçenek adıyla:** çalışan komutun ya da sıcak tutamağın seçeneği adının tamamıyla (büyük-küçük harf ve Türkçe işaretler önemsiz:
   “KOPYA”, “olcekle”) seçilir; ad yazılan değerden önce gelir, yalnız ad alan araçta (stil adı) aracın okuması önce gelir. Araç
   yazılanı almadıysa adın başı da (iki harften) seçer; birden çok seçeneğin başıysa hiçbiri seçilmez, adayları söylenir. Enter ve Esc
   seçenekleri onaylar ve vazgeçer. Harf ve düğme eskisi gibi.
2. **Komut satırı:** yöntem adı, sonra tam takma ad, sonra tam başlık, sonra kullanıcının modeli, sonra adın başı (iki harften, harfle
   başlayan). Önce önde olan bağlamda (pafta açıkken pafta komutları, değilse çizimin) sonra öbüründe aranır. Tek komutu anlatan baş
   çalıştırılır ve günlük hangi komut olduğunu söyler (“orta ha”: Orta hat.); birden çok komutu anlatan ad ya da baş çalıştırılmaz,
   adaylar başlığı yazılanla başlayanlar ve kısalar önde, en çok sekiz, ilk takma adlarıyla söylenir.
3. **Çakışmalar testte yakalanır:** bir bağlamdaki her takma ad tek komutu ya da aynı aracın yöntemini anlatır
   (`no_typed_name_names_two_things`); her aracın ilk isteminde iki seçenek aynı harfi ya da adı taşımaz
   (`no_first_prompt_has_two_options_on_one_key_or_name`). Başlıklar paylaşılabilir (menünün bağlamı ayırır); komut satırı onları
   belirsiz sayar.

## Sonuçlar

- AutoCAD takma ad profili (ADR 0282) tam adların önünde kendi kuralıyla gelir; kısaltma profil adlarına uygulanmaz.
- Kısayol listesinde “Seçeneği adıyla seç”.

## Doğrulama

- `cargo test -p kentos-interaction --test all typed_options`, `cargo test -p kentos-desktop typed_names`,
  `cargo test -p kentos-desktop no_typed_name_names_two_things`; bütün izler değişmeden geçer.
- Resim: `KENTOS_SHOTS_ONLY=komut-adlari cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
