# ADR 0400: Form elemanları ve klavye

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim):
  - formlarda Tab ile alanlar arasında gezilemiyor;
  - bileşenler daha kullanılabilir form elemanları olmalı.

  Form ADR'leri 0400'den başlar (sahibin kararı). Bu ADR formların (ADR 0401) dayandığı ortak temeldir.
- **Bağlam belgesi:**
  - ADR 0016 (KentOS UI);
  - ADR 0018 (tuş ve fare anlamları);
  - DESIGN.md §7;
  - `plan/form.md` §4.4 (klavye, odak ve hata gezinmesi).
- **Web:** ertelendi (ADR 0228). Tarayıcının kendi Tab sırası var; web'in özel açılır kutuları geri dönüşte aynı kuralları alır (TODOS.md §0.2).

## Bağlam

- **Tab hiçbir şey yapmıyordu.** iced'in metin kutusu Tab'ı yakalamaz, uygulamanın tuş yönlendiricisine bırakır. Yönlendirici de onu
  pencerelerde hiçbir işe bağlamıyordu. Kod tabanında `focus_next` hiç yoktu.
- **KentOS UI'ın kendi form öğeleri odak almıyordu.** Seçim kutusu, açılır kutu, anahtar, onay kutusu, parçalı seçim ve radyo yalnız
  fareyle kullanılıyordu. Klavyeyle değer seçmenin yolu yoktu.
- **iced'in `focus_next`'i bütün ağacı dolaşır.** Bir iletişim kutusunun son alanından sonra odak arkadaki pencerenin komut satırına
  kaçardı.

## Karar

### 1. Kapsamlı odak geçişi (`kentos_ui::focus`)

- **Tab ve Shift+Tab:** `focus::next(back)` sonraki ya da önceki odaklanabilir öğeyi seçer.
- **İki geçiş:**
  - Önce odaklanabilirler, sınırlarıyla ve odak durumlarıyla sayılır; kapsamlar da sayılır.
  - Sonra yalnız seçilen öğe odak alır, odaktaki öbürleri bırakır.
- **Kapsam:** `focus::scope_id()` kimlikli kap. `Dialog` kendini kapsam olarak işaretler.
  - Odak en üstteki kapsamın içinde döner; sondan başa geçer, arkadaki pencereye kaçmaz.
  - Kapsam yoksa bütün pencere dolaşılır.
- **Atlananlar:** sıfır boyutlu (gizli, sığmayan) öğeler.
- **Panelde Tab:** `focus::next_if_focused(back)` yalnız kapsamda bir öğe odaktayken çalışır. Böylece çizim alanındaki Tab (kenetin
  sıradaki adayı, ADR 0271) bozulmaz.
- **Masaüstünün yönlendiricisi** (`input.rs`):
  - Pencere açıkken Tab odak geçişidir. Pencerenin kendi tuşları (Tabloyu düzenle'nin ızgarası, SVG düzenleyicisi, İfade oluşturucu)
    önce gelir.
  - Pencere yokken Tab önce kenet adayıdır; o yoksa odaktaki panel kutusunun sonrakidir.
  - Hücre düzenleyicilerinin Tab'ı (Noktalar, Öznitelik tablosu, Köşe tablosu) değişmez.

### 2. Odaklanabilir form öğeleri (`widget::key_focus`)

- **Ortak parçalar:**
  - odak bayrağı (`Flag`, iced'in `Focusable`'ı);
  - odak halesi: metin kutusunun `focus_ring`'iyle aynı, vurgu renginde 3 piksel;
  - tuşun okunması;
  - fareyle basışın odağı vermesi ya da alması.
- **`key_focus(öğe)` sarmalayıcısı:** düğme tabanlı öğeleri odaklanabilir yapar.
  - Boşluk ve Enter `on_activate`'i gönderir.
  - Oklar `on_step`'i gönderir: sol ve yukarı −1, sağ ve aşağı +1.
  - Mesajı olmayan (devre dışı) öğe odak almaz.
- **Öğelerin tuşları:**

| Öğe | Odak | Tuşlar |
|---|---|---|
| Onay kutusu (`check_box`) | sarmalayıcı | Boşluk, Enter: işaretle ya da kaldır |
| Parçalı seçim (`Segmented`) | sarmalayıcı | oklar: seçilebilen önceki ya da sonraki parça |
| Radyo (`RadioGroup`) | sarmalayıcı | oklar: seçilebilen önceki ya da sonraki seçenek |
| Anahtar (`Switch`) | kendi durumunda | Boşluk, Enter: aç ya da kapat |
| Açılır kutu (`Dropdown`) | kendi durumunda | Enter, Boşluk, F4, Alt+↓: aç; ↑ ↓: `on_step` |
| Seçim kutusu (`Select`) | açılır kutunun | kapalıyken ↑ ↓ önceki ya da sonraki seçenek (başlıklar ve devre dışılar atlanır); açıkken ↑ ↓ Home End satırı işaretler, Enter seçer |

- **Seçim kutusunun açık listesi:**
  - Arama yazıldıkça işaret ilk eşleşmeye gider.
  - Enter işaretli satırı, yoksa ilk eşleşmeyi seçer.
  - İşaretli satır, imleç üstündeymiş gibi çizilir.
  - Açılır kutunun `keys`'i, panel açıkken tuşu panelin öğelerinden önce alır.

## Denetimler

- `cargo test -p kentos-ui --features snapshot`:
  - `focus::tests`: kapsam, döngü, gizli öğe.
  - `focus::window_tests::tab_goes_round_the_dialog_s_fields_and_space_ticks_the_box`: pencere açmadan çizilen arayüzde Tab ve Shift+Tab
    pencerenin dört alanında döner, arkadaki kutuya geçmez; Boşluk onay kutusunu işaretler; ↓ seçim kutusunda sonraki seçeneği seçer.
  - `select::tests::the_keyboard_marks_and_steps_the_shown_choices`.
- `cargo clippy -p kentos-ui -p kentos-desktop --all-targets`.

## Sonuçlar

- Her iletişim kutusunda Tab çalışır. Yeni bir pencere `Dialog`'la kurulduğu sürece ayrıca bir şey gerekmez.
- Yeni bir form öğesi ya iced'in odaklanabilir bir öğesidir ya da `key_focus` ile sarılır. Fareyle çalışıp klavyeyle çalışmayan form
  öğesi eklenmez.
- Düğmeler (Kaydet, Vazgeç) iced'de odak almaz. Pencerenin Enter'ı ve Esc'i onların yerini tutar.
