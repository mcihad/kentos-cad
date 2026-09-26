# ADR 0060: Masaüstünde Yazı aracı ve çizimin üstündeki yazı kutusu

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.6, §4.7; TODOS.md `UX-01`, `UI-11`; ADR 0021 (native araç oturumu), 0055 (çizimin yazıları), 0057 (`cad.entities.create`)
- **Sahibin yönü (26 Eylül):** web'deki araçlar ve düzenleyiciler masaüstüne birebir taşınır.
- **Web ajanının tarifi (26 Eylül):** `tools/annotateTools.ts` (`TextTool`), `ui/shell/InlineTextEditor.ts`, `tools/SelectTool.ts` (`maybeEditText`) koddan okundu. Web'in bugünkü dalındaki bir düzeltme de alındı: etkin katman kilitliyse kutu açılmaz.

## Bağlam

- Masaüstünde Yazı (`tool.text`) "web'de var; masaüstüne henüz taşınmadı" diyordu. Şeridin Açıklama grubunda soluktu.
- Web'de yazı, tıklanan yerde açılan bir kutuya yazılır: Enter ekler, Esc vazgeçer, odak kaybı ekler.
- Aynı kutu, seçim aracındayken bir yazıya ya da ölçüye çift tıklayınca değerini yerinde düzenler.

## Karar

### Araç (`crates/native/interaction/src/text.rs`)

- Adımlar, istemler ve seçenekler web'inkilerdir, kelimesi kelimesine:
  - "yazının başlangıcına tıklayın [Yükseklik (Y): 2.5 mm / Açı (A): 0°]";
  - "kâğıt üzerindeki yazı yüksekliğini mm olarak yazın";
  - "açıyı yazın (derece) ya da doğrultu için iki noktaya tıklayın";
  - "doğrultunun ikinci noktasına tıklayın";
  - "yazıyı tıkladığınız yere yazın; Enter ekler, Esc vazgeçer".
- Yükseklik kâğıt milimetresidir; projenin pafta ölçeği onu metreye çevirir (2,5 mm, 1:1000'de 2,5 m).
  - Yazılan sayıda ondalık virgül noktadır. Sıfır ve eksi yükseklik alınmaz.
- Açı derece olarak yazılır ya da iki tıkla verilir. Sola bakan doğrultu okunur kalsın diye çevrilir; bunu ortak çekirdek hesaplar (`text_angle`).
  - Önizlemenin etiketi web'deki gibi çevrilmemiş açıyı gösterir.
- Yükseklik ve açı uygulama açık kaldıkça hatırlanır (`Memory`).
- **Tık** noktayı söyler. Araç ev sahibinden o noktada bir yazı kutusu ister (`ViewChange::Text`); ev sahibi yazılanı `Tool::text_typed` ile geri verir.
  - Her tık yeni bir nesne başlatır.
  - Etkin katman kilitliyse kilit iletisi tıkta söylenir ve kutu açılmaz (web'in bugünkü düzeltmesi).
- **Yazma:** yazı `cad.entities.create` ile tek geri alma adımıdır ("Ekle"); ardından "Yazı eklendi: “…”" söylenir.
  - Web bugün belgeye doğrudan yazıyor. Web ajanının önerisi web'i de ürün komutuna taşımak.
  - Gizli katmanda komutun uyarısı söylenir, yazı yine yazılır.
- **Tuşlar:**
  - Enter, Boşluk ya da kısa sağ tık: başlangıçta araçtan çıkar; bir soru sorulurken ya da kutu açıkken başlangıca döner.
  - Esc araçtan çıkar; web'in aracının kendi iptali yoktur. Kutunun içindeki Esc yalnız yazıdan vazgeçer.
  - Ctrl+Z, az önce eklenen yazıyı geri alır.
- **Önizleme:** imleçte yazının yüksekliğinde (en az 8 px) ve dört katı eninde, açısıyla dönmüş kesikli bir kutu.

### Yazı kutusu (`apps/desktop/src/text_field.rs`)

- Çizimin üstünde, yazının başladığı yerde açılır.
  - Yazı boyu, yazının ekrandaki yüksekliğidir (13–48 px, web gibi). Kutu çizimle birlikte kayar ve yakınlaşır.
  - Altında ipucu vardır: "Enter: ekle · Esc: vazgeç", düzenlemede "Enter: kaydet · Esc: vazgeç".
- **Kaydetme:** Enter, kırpılmış yazıyı kaydeder; boş yazı hiçbir şey yazmaz. Esc vazgeçer.
- **Odak kaybı** (web'deki blur): kutu açıkken çizime basmak, orta tuşla kaydırmak, şeritten ya da bir menüden komut çalıştırmak yazıyı önce kaydeder.
  - Yazı çalışırken başka bir yere tıklamak bu yazıyı kaydeder ve yeni kutuyu oraya açar; yazılar art arda tek tıkla yazılır.
  - Tekerlek kaydetmez.
- **Çift tık:** komut çalışmıyorken aynı yazıya ya da ölçüye 450 ms içinde iki kez basmak kutuyu onun değeriyle açar. Yazının çizimdeki hâli bu sırada gizlenir.
  - Kilitli katmanda: "Kilitli katmandaki yazı düzenlenemez."
  - Yazı: boş ya da aynı değer değiştirmez.
  - Ölçü: kutu değerinin yerinde ortalanır, ipucu ölçülen değerdir. Boşaltmak ölçülen değere döndürür.
  - Değişiklik web'deki gibi tek geri alma adımıdır ("Değiştir").
- Kutu açıkken tuşlar ona gider; uygulamanın kısayolları çalışmaz.

## Web'den ayrılanlar

- **Kutu yazıyla birlikte dönmez.** Iced'in yazı kutusu döndürülemiyor; web kutuyu CSS ile döndürür. Kutu yazının başladığı yerde, düz durur. Yazı kaydedilince doğru açıyla çizilir.
- **Odak kaybının kapsamı:** Iced yazı kutusunun odağını yitirdiğini bildirmiyor. Kutu; çizime basışta, orta tuşla kaydırmada ve komut çalışınca kaydedilir. Paneldeki bir düğmeye tıklamak kutuyu açık bırakır.

## Doğrulama

- `crates/native/interaction/tests/text.rs`:
  - web'in istemleri;
  - tıkta istenen kutu, Enter ile yazı, tek adım ve Ctrl+Z;
  - Esc ya da boş yazı hiçbir şey yazmaz;
  - yükseklik ondalık virgülle, açı iki tıkla (çevrilmiş) ve hatırlanması;
  - kilitli katmanda kutunun açılmaması;
  - Enter'ın anlamları.
- `text_field::tests`:
  - Yazı'nın kutusu ve Enter;
  - Esc; başka yere tıklamanın kaydedip yeni kutu açması;
  - çift tıkla düzenleme ("Değiştir") ve kilitli katmanın iletisi.
- `pnpm rust:test`, `pnpm rust:test:desktop` (clippy temiz), `pnpm inventory:check`.
- Görüntüler (`text_field::screens`, `.run/shots/yazi-kutusu-*`), koyu ve açık, 1440×900 ve 1100×650: Yazı'nın kutusu ve "Çınar sokağı"nın yerinde düzenlenmesi.
