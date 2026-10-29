# ADR 0748: Öznitelikler'de kısalan ad ve değer, “Çeşitli”

- **Durum:** kabul edildi (2026-10-29). Sahibin isteği (`/frontend-design` ile, “object inspector iyileştir daha da
  iyi hale getir”); ADR 0747'nin Kalan'ı.
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0747'den sonra panelde üç eksik kalmıştı:

- **Satırın adı** sığmayınca harfin ortasından kesiliyordu. Ad, kırpan bir kabın içinde düz yazıydı.
- **Uzun değer:** salt okunur değer ve liste “…” ile kısalıyordu, ama tamamını okumanın yolu yoktu. Düzenlenebilir
  hücre ise uzun değeri “…” koymadan, alanın kenarında kesiyordu: değerin devam ettiği görünmüyordu.
- **“Çeşitli”:** seçili nesnelerin değerleri farklıysa satır “Çeşitli” yazar (ADR 0063, 0286). Bu yazı bir değer gibi,
  metin renginde görünüyordu. Hücreye basınca düzenleme “Çeşitli” yazısıyla başlıyordu, kullanıcı onu silip yazıyordu.

Başvurulan yollar (29 Ekim 2026'da okundu):

- AutoCAD'in Özellikler paleti: seçili nesnelerin bir özelliği farklıysa *VARIES* gösterilir; değiştirilebilen bir
  özelliğe yazılan seçimin bütün nesnelerine uygulanır.
  <https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-DidYouKnow/files/GUID-94C065AB-FF9E-4752-B778-23D2FBB87E18.htm>
- Microsoft'un Windows yönergeleri: kısalan adın tamamı ipucunda gösterilir (“full name infotip”); “…” ayrı bir bağlantı
  gibi tıklanır yapılmaz.
  <https://learn.microsoft.com/en-us/windows/win32/uxguide/ctrl-tooltips-and-infotips>,
  <https://learn.microsoft.com/en-us/windows-server/manage/windows-admin-center/extend/guides/ui-text-style-guide>,
  <https://learn.microsoft.com/en-us/windows/win32/uxguide/text-ui>

## Karar

### 1. Kısalan ad ve değer “…” ile biter, tamamı ipucundadır

- **Satırın adı** `Elided`'dır: sığmayınca harf sınırında kesilir ve “…” ile biter.
- **Düzenlenebilir hücre** düzenlenmezken değerini alanına sığdırır (`elided::fit`, `Elided`'ın kuralı). Düzenlemeye
  geçince bütün değeri gösterir. Ölçüm yalnız değer, yer ya da yazı değişince yapılır.
- **İpucu yalnız kısalınca açılır** (`Tip::when_cut`): ad, salt okunur değer, liste ve hücre. `Elided` ve `EditCell`
  kısaldıklarını `Operation::custom` ile (`elided::Cut`) söyler. İpucu açılmadan önce bakılır; sığan yazının üstünde
  imleç çıkana dek ipucu açılmaz. Bekleme ve yer DESIGN.md §7.8'inkidir: 450 ms, altta, yer yoksa üstte.
- **Basış:** kısalmış değerin “…”sına ya da son görünen harfine basmak imleci bütün değerin sonuna koyar. İlk
  harflerine basmak imleci oraya koyar. “…” ayrı bir bağlantı değildir; hücrenin her yeri düzenlemeyi başlatır.
- Basış, hücre bütün değeri almadan önce gösterilen metne verilir. iced imlecin yerini önceki yerleşimin metninden
  bulur ve alanın değerini o bayt sınırından keser. “…”dan sonraki bir bayt Türkçe bir harfin ortasına düşebilir ve
  uygulamayı düşürebilirdi.

### 2. “Çeşitli” soluk yazılır

- Salt okunur değer, liste ve hücre “Çeşitli”yi (ortak özniteliğin “Çeşitli (2/3 nesnede)”sini de) soluk renkte yazar:
  bir değer değil, değerlerin farklı olduğu okunur (`property_grid::varies`).
- **Hücrede** “Çeşitli” boş alanın soluk yer tutucusudur. Düzenleme boş başlar ve yazılan bütün seçime gider (AutoCAD'in
  *VARIES*'ı). Boş bırakılan hücre hiçbir nesneyi değiştirmez.

### 3. İpucunun saati

İpucu arayüzün saatini okur (`clock::now`): uygulamada sistemin saati, testte ekransız görüntünün kendi anı. Böylece
bekleme testte makinenin hızından bağımsız sınanır. Açık ipucu yazısını işlemlere verir (bulmak, ekran okuyucu).

### 4. Yazarken bekleme (ADR 0747 §3'ün devamı)

Sahibin büyük çiziminde (71 820 nesne) prob yeniden koşturuldu: hata ayıklama yapısında ve release'te çizim ölçeği
değiştirilerek, wgpu ile. Bekleme yine yeniden üretilemedi.

| Yapı ve görünüm | Bir harf (ortanca) | En uzun |
|---|---:|---:|
| Hata ayıklama, bütün çizim | 27,5 ms | 28,5 ms |
| Release, bütün çizim (1:2517) | 13,5 ms | 14,2 ms |
| Release, ölçüye yakın (1:81) | 13,2 ms | 16,0 ms |
| Release, ölçünün beş katı dışı (1:405) | 13,3 ms | 13,8 ms |

Kaydedilmemiş çizimin kurtarma kopyası kendi iş parçacığında yazılır (`recovery.rs`). Hücreye yazılan harf uygulamaya
mesaj göndermez.

## Doğrulama

- `cargo test -p kentos-ui --features snapshot --lib -- edit_cell property_grid tip`:
  - kısalan değer, ad ve hücre fareyle 500 ms beklenince ipucunu açar, sığan açmaz;
  - uzun değer alanında “…” ile durur, genişleyince bütün görünür;
  - “…”ya basıp yazılan bütün değerin sonuna, ilk harflere basıp yazılan oraya gider;
  - “Çeşitli” hücresi boş başlar: Enter ya da başka yere basmak bir şey göndermez, yazılan gönderilir.
- KentOS UI'ın bütün testleri (298) geçer; `clippy -D warnings`.
- `cargo test -p kentos-desktop --bin kentos-cad inspector_screens -- --ignored`: resimler `.run/shots/oznitelik-*`
  (iki ölçünün “Çeşitli”si ve düzenlenirken; uzun not ve ipucu).
- `KENTOS_PROBE_FILE=… cargo test --release -p kentos-desktop --bin kentos-cad inspector_probe -- --ignored`: §4'ün
  süreleri.

## Kalan

- Gecikme derleme yokken de görülürse: uygulamanın kendi kare sürelerini günlüğe yazması.
