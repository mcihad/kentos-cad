# ADR 9300: Şeridin ana araçları büyük kalır, düğmeler arasında boşluk kalmaz

- **Numara:** bu ADR 24 Kasım 2026'da 9200 olarak yazıldı (commit `740502a25`). `render-motoru` dalı 9200–9219'u
  kullandığı için aynı gün 9300 oldu; eski commit iletilerindeki “ADR 9200” budur.

- **Durum:** kabul edildi (2026-11-24). DESIGN.md §7.3.1'in “Pencere daralınca” kuralını ve 20 Kasım'ın UR2-004
  kuralını (küçük bölünmüş düğmenin ailesinin en uzun adı kadar yer tutması) değiştirir.
- **İstek:** sahibin sözleri (24 Kasım 2026):
  - “nokta çizgi çoklu çizgi alan gibi ikonlar ribbon üzerinde büyük olmalıdır”;
  - “ribonda sadece gis değil tüm projelerde butonlar arasında çok boşluk var bazı yerlerde nedenin bilmiyorum, yani
    mantıksal olarak ribonda en önemli araçlar genelde büyük olur”.
- **Kural:** CLAUDE.md §25 kural 2: şerit her modülün profilinden gelir; bu iş yerleşimi değil, sığdırmayı ve düğmenin
  genişliğini değiştirir. Dört modülün (CBS, Mimari, Harita, Yol) şeridi aynı kuralla çizilir.
- **Web:** iptal edildi (sahibin sözü, 24 Kasım 2026; ADR 0228).

## §1 Araştırma (24 Kasım 2026'da okundu)

- **Microsoft, *Ribbons* (Windows tasarım kılavuzu):** pencere büyükken açık sekmede en sık kullanılan komutlar en çok
  görsel ağırlığı taşır ve tek tıkla çalışır; öbürleri en çok dört tıkla.
  <https://learn.microsoft.com/en-ie/Windows/Win32/uxguide/cmd-ribbons>
- **Jensen Harris, *Scaling Up, Scaling Down*:** büyük hâller bütün komutları adıyla ve önemli komutları büyük
  düğmeyle gösterir; grup küçülünce komut kaybolmaz, menüde aynı düzenle durur.
  <https://learn.microsoft.com/en-gb/archive/blogs/jensenh/scaling-up-scaling-down>
- **Microsoft, *Customizing a Ribbon Through Size Definitions and Scaling Policies*:** her grubun boyutları sırayla
  (büyük, orta, küçük, açılır) iner; sırayı tasarımcı verir, önemli grup en son iner.
  <https://msdn.microsoft.com/en-us/library/dd316927.aspx>

**Ortak örüntü:** sekmenin ana araçları en son küçülür; daha az önemli gruplar önce adıyla tek düğmeye katlanır.

## §2 Bulgu

1. **Sığdırma ana araçları erken küçültüyordu.** `kentos_ui::widget::ribbon::fit` korunan panelin (`keep`: Giriş'te
   Çizim ve Değiştir, CBS'nin Düzenle'sinde Oluştur ve Değiştir) büyük düğmelerini başka bir paneli katlamadan önce
   küçük etiketliye indiriyordu. Sonuç tersine döndü: Mimari, Harita ve Yol'un Giriş'inde Çizim'in araçları 1440 px'te
   küçük, 1100 px'te (öbür paneller katlandığı için) büyüktü.
2. **Bölünmüş düğme yer ayırıyordu.** UR2-004'ten beri küçük bölünmüş düğme ailesinin en uzun adı kadar yer tutuyordu.
   Dikdörtgen ▾ “Döndürülmüş dikdörtgen”, Köşe yuvarla ▾ “Tüm köşeleri yuvarla” kadar yer ayırıyordu. Sütunun
   yanında her sekmede, her modülde 40–90 px boşluk kalıyordu (resim: `once/mimari-home-1440.png`,
   `once/cbs-edit-1100.png`).

## §3 Karar

1. **Ana araçlar en son küçülür.** Korunan panelin her adımı, öbür panellerin bütün adımlarından sonra gelir: öbür
   paneller sağdan sola önce küçük etiketliye iner, sonra adıyla tek düğmeye katlanır. Ancak ondan sonra korunan panel
   küçük etiketliye iner; en son o katlanır. Artan yer yine önce korunan panellere, sonra soldan sağa öbürlerine geri
   verilir. Korunan paneli olmayan sekme (CBS'nin Harita'sı, Veri'si) eskisi gibi küçülür.
2. **Düğme kendi adı kadar yer tutar.** Küçük bölünmüş düğmenin yer ayırması (`Button::reserve`) kaldırıldı: yüzünde
   hangi üye varsa onun adı kadardır. Listeden daha uzun adlı üye seçilince yalnız ondan sonraki düğmeler yer açar.
   Çizim'in büyük araçları yerinde kalır: korunan panel öbürleri katlanmadan küçülmez. UR2-004'ün sınaması buna göre
   değişti (`choosing_a_method_leaves_the_main_tools_in_place`).
3. **CBS'nin dört büyük aracı.** Düzenle sekmesinin Oluştur paneli katmanın nesne türlerini dört büyük araçla çizer:
   Nokta, Çizgi, Çoklu çizgi, Kapalı alan. Çizgi önceden küçüktü. Panelde en çok dört büyük düğme kuralı korunur.

## §4 Kanıt

- **Kit:** `fit_tests::a_kept_panel_stays_large_until_the_others_fold` korunan panelin öbürleri katlanana kadar büyük
  kaldığını, sonra küçülüp en son katlandığını sınar; korunan panel yoksa eski sıra sürer.
  `layout::tests::a_small_split_is_as_wide_as_its_own_face`: yer ayırma yok.
- **Masaüstü:**
  - `ribbon_tests::panels_step_down_from_the_right_and_giris_keeps_its_drawing_tools`: korunan panel küçüldüyse öbür
    her panel katlanmıştır; Çizim 1100 ve 1440 px'te büyüktür.
  - `ribbon_tests::cbs_draws_a_layers_four_kinds_with_large_tools`: Oluştur'un dört büyük aracı; 1100 ve 1440 px'te
    büyük.
  - `every_tab_fits_1100_pixels_without_scrolling` ve `no_ribbon_button_is_an_icon_alone` yine geçer.
  - Mimari paketinin şerit sınamaları (8) ve programlar arası sınamalar (37) geçer.
- **Resim:** `ribbon_tests::module_screens` dört modülün üçer sekmesini 1100, 1440 ve 1600 px'te çeker
  (`.run/gui-evidence/kullanim-9300/once` ve `sonra`, karşılaştırma `karsilastirma/`, Git'e eklenmez).
- **Yan düzeltme:** `settings_look_tests::a_click_on_a_card_a_swatch_or_a_step_writes_the_draft` paylaşılan yazı
  boyutu kilidini almıyordu; başka sınama aynı anda yazı boyutunu değiştirince tıklama boşa düşüyordu (iki koşudan
  birinde). Kilit eklendi; üç koşu üst üste geçti.
- **Not:** `scripts/ui/ribbon.py` geride kalmış: `catalog/v1/ribbon.json` son haftalarda doğrudan düzenlenmiş
  (`ifc.view`, `terrain.sheetContours`, nokta bulutu komutları, Mimari'nin kolon ve kiriş aileleri …). Betikle
  üretmek bunları siler; bu iş JSON'u doğrudan düzenledi. Betiğin JSON'la eşitlenmesi ayrı iştir (TODOS.md).
