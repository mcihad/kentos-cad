# 3006 — Mimari düzenleyicileri sorarak kapanır; seçimle çalışan pencerede Çizimden seç

Tarih: 2026-11-07. Durum: kabul (M7, ADV-09; plan S3, G11).

## Sorun

- **Düzenleme sorulmadan atılıyordu.** Kendi kaydı olan dört Mimari düzenleyicisi pencerenin yanına tıklama, × ya da Esc
  ile kapanınca kaydedilmemiş değişikliği sormadan atıyordu:
  - Bina programı ve Duvar stilleri (Kaydet);
  - Kapı ve pencere tipleri ve Malzemeler (Uygula; alttaki "Kapat" da sormuyordu).

  DESIGN.md §7.9: "Kendi kaydı olan düzenleyiciler … yalnız kaydedilmemiş bir değişiklik varsa sorar." Tablo düzenleyicisi
  ve iki tasarımcı soruyordu, her biri kendi kopyasıyla.
- **Seçim değiştirilemiyordu.** Oda modülleri ve Tasarım seçenekleri "Seçimden tanımla", "Seçimi ekle" diyor, ama modal
  oldukları için açıkken seçim değiştirilemiyordu.
- **F1 Esc'le kapanmıyordu (G11).** Arama kutusundayken Esc'i kutu yutuyordu.
  - Yazı alanının yuttuğu Esc (`CapturedEscape`) yalnız iki pencereyi kapatıyordu.
  - Esc, Mimari düzenleyicilerini de `close_dialog`'da sormadan kapatıyordu.

## Araştırma (7 Kasım 2026)

- **AutoCAD — Block Definition iletişim kutusu** (help.autodesk.com):
  - "Select Objects" düğmesi "closes the Block Definition dialog box temporarily while you select the objects for the
    block. When you finish selecting objects, press Enter to return to the dialog box."
  - Seçimle çalışan bir pencerenin CAD'deki yerleşik yolu budur.
- **DESIGN.md §7.9.1:** soru, soran pencerenin üstünde açılan küçük bir penceredir. Başlık "Kaydedilmemiş değişiklikler";
  solda gidişin cevabı, sağda Vazgeç ve önce kaydeden cevap.
- **Mevzuat:** bu işe doğrudan bir hüküm yok.

## Karar

- **Ortak soru:** `kentos_ui::widget::unsaved(editor, discard, stay, save)`.
  - Metin: "“{editor}” içinde kaydedilmemiş değişiklikler var. Pencere kapanırsa bu değişiklikler kaybolur."
  - Cevaplar: "Kaydetmeden kapat" solda, "Vazgeç" ve "Kaydet ve kapat" sağda.
  - Sorunun yanına tıklamak Vazgeç'tir. Kaydet yapılamıyorsa "Kaydet ve kapat" kapalıdır.
- **Dört düzenleyici** (`unsaved_close::Answer`):
  - Pencerenin yanına tık, × ve Esc pencerenin Kapat'ıdır. Değişiklik varsa sorar, yoksa hemen kapanır.
  - Soru açıkken Esc, Vazgeç'tir.
  - Alttaki "Vazgeç" düğmesi sözünü söyler, sormadan atar. Malzemeler'deki ve tiplerdeki "Kapat" ise sorar.
  - "Kaydet ve kapat" Kaydet'i ya da Uygula'yı çalıştırır. Yazılamadıysa pencere kalır ve nedenini söyler.
  - Değişiklik ölçüsü:
    - Duvar stilleri `wall_styles_changed`;
    - tipler `opening_type_change` ya da yeni ad;
    - Malzemeler `change_of`;
    - Bina programı ise açılışta kendi yazacağı programla karşılaştırır (`opened`), çünkü satırlar sayıları metin olarak
      tutar ve dokunulmamış bir pencere çizimdeki programla bayt bayt aynı çıkmayabilir.
- **Esc:** yazı alanındayken basılan Esc, F1'i ve bu dört düzenleyiciyi kapatır; düzenleyiciler önce sorar.
- **Çizimden seç** (`window_pick.rs`), Oda modülleri ve Tasarım seçenekleri penceresinin sol altında:
  - Pencere çekilir, durumu korunur.
  - Sahneden seç aracı (`pick_objects`) çalışır: tık ters çevirir, kutu ekler. Seçim, var olan seçimden başlar.
  - Enter ya da Boşluk seçimi tutar ve pencereyi geri açar; Esc önceki seçimi geri verir.
- **Sözcükler** (ADR 3003'ün sözlüğü): Duvar stilleri'nde stilin "Genişlik"i "Kalınlık", ipucunda "duvarın genişliğinden"
  "duvarın kalınlığından"; patlatmanın uyarısı "bir yayı duvarın kalınlığından dar".

## Ölçüm

- **Testler** (`unsaved_close_tests.rs`, 5 durum):
  - dokunulmamış Duvar stilleri ve Bina programı sormadan kapanır;
  - değişmiş pencere sorar, Vazgeç tutar, Esc sorar ve sorunun üstünde Vazgeç'tir, Kaydetmeden kapat kapatır;
  - Bina programında Kaydet ve kapat birimi yazıp kapatır;
  - F1 yazı alanında Esc ile kapanır;
  - Çizimden seç pencereyi çeker, Esc'te seçimi geri verir, Enter'da tutar.
- **Test takımları:** arayüz kitaplığı 353 test, masaüstü 1973 geçti, 1 düştü. Düşen
  `module_surface_tests::each_program_docks_its_own_panels`, `main`'den gelen eski hata.
- **Gerçek pencere** (ADR 3000; `fixtures/interaction/real/pencere-kapatma.txt`, 52 etkileşim, KLM 38 s):
  - Duvar stilleri'nde Yeni ve pencerenin yanına tık soru açtı; Vazgeç pencereyi yeni stille bıraktı; Esc yine sordu;
    Kaydetmeden kapat kapattı.
  - F1'in arama kutusuna "duvar" yazılıp Esc'e basılınca F1 kapandı.
  - Oda modülleri'nde Çizimden seç pencereyi çekti; komut satırı "Oda modülünün nesneleri: nesneleri tıklayın ya da
    pencereyle seçin (0 seçili) · Bitti Enter · Vazgeç Esc" dedi; duvara tık ve Enter pencereyi duvar seçili olarak geri
    açtı.
- **Kalanlar** (ADV-09):
  - Pencerelerin yüzer ya da dok paneli olması. Bu dilim modal kalıp seçimi Çizimden seç'le açtı.
  - Pick sürerken başka bir komut başlatılırsa pencere geri açılmaz; komutu yeniden çalıştırmak açar.
