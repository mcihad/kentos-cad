# ADR 0075: Masaüstünde katman ağacının araması ve klavyesi; Hesap tablolarında ↑ ↓

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.6, §4.10; TODOS.md `UI-11`; ADR 0018 (tuşların sırası), 0071 (ölçü tablosu), 0072 (katman silme)
- **Kaynak:** web kodu, koddan okundu:
  - `ui/widgets/TreeView.ts`: `onKey` ve `render`'daki süzme;
  - `ui/layers/LayersPanel.ts`: “Katman ara” kutusu ve ağacın bağdaştırıcısı;
  - `ui/calc/common.ts`: tablonun `key`'i.

## Bağlam

- Web'in Katmanlar panelinin üstünde “Katman ara” kutusu var. Ağaç odaktayken klavyeyle de gezilir.
- Masaüstünde ikisi de yoktu. TODOS.md bunu "Kalan: ağacın klavyesi (F2, Delete, oklar)" diye tutuyordu.
- Hesap pencerelerinin ölçü tablosunda ↑ ve ↓ yoktu (ADR 0071: "Açık: tabloda ↑/↓ ile gezinme").

## Karar

### Katman ara (`apps/desktop/src/layer_tree.rs`)

- **Yer:** kutu panelin üstündedir, panel genişliğindedir. Büyütecin simgesi kutunun içinde, soldadır (web'in `panel__toolbar`'ı).
- **Ne kalır:**
  - Adında yazılan metni taşıyan düğümler ve onların üstündeki gruplar.
  - Karşılaştırma küçük harfle ve Türkçe kurala göredir (`toLocaleLowerCase('tr-TR')`: I → ı, İ → i). Metnin başındaki ve sonundaki boşluk sayılmaz.
  - Yalnız grubun adı tutarsa grubun çocukları kalmaz; web'de de böyledir.
- **Açık gruplar:** arama sürerken kalan her grup açık görünür. Grubun kendi açık/kapalı durumu değişmez; arama silinince eski hâline döner.
- **Boş sonuç:** ağaç “Aramayla eşleşen katman yok.” der. Aramasız boş ağaç “Katman yok.” der.

### Ağacın klavyesi

- **Klavye ağaca ne zaman geçer:** bir satıra basılınca (web'de odaklanan ağaç).
- **Ne zaman geri alınır:**
  - çizim alanına sol ya da sağ basış;
  - komut satırının odağı alması;
  - şeritten ya da bir menüden çalıştırılan komut; web'de düğme odağı alır.
  - Katman ara'ya yazmak da ağacın klavyesi sayılmaz.
- **Tuşlar** (web'in `onKey`'i):
  - ↑ ↓ Home End satır seçer.
  - → kapalı grubu açar, açık grubun içine girer.
  - ← açık grubu kapatır; öbür satırlarda üstteki gruba gider.
  - Enter katmanı etkin yapar, grubu açar ya da kapatır. Bu, çift tıkla aynı kuraldır.
  - Boşluk gösterir ya da gizler.
  - F2 yeniden adlandırır.
  - Delete satırın menüsündeki Sil'dir: retleri ve sorusuyla (ADR 0072).
  - Değiştirici tuşlar sonucu değiştirmez; web'de de değiştirmez.
- **Başlangıç satırı:** tuşlar çizimin seçiminin seçili gösterdiği satırlardan başlar, çünkü görünen onlardır. Yoksa son seçilen satırdan başlar. Tuşların seçtiği satır görünüme kaydırılır.
- **Satır seçili değilken:** Delete ağacın değildir, çizimin seçimini siler (web'deki gibi).
- **Öbür tuşlar** her yerdeki gibi çalışır: kısayollar, komut satırına yazmak.
- **Tuş sırasındaki yeri** (ADR 0018): odaktaki alanlar gibi ilk basamaktadır. Uygulama menüsünden, pencerelerden, çizimin üstündeki yazı kutusundan, değer alanından ve komut satırından sonra, komutun seçenek harflerinden önce gelir.

### Hesap tablolarında ↑ ↓ (`apps/desktop/src/calc/grid.rs`)

- ↓ Enter'dır: aynı sütunu açık olan sonraki satır, yoksa yeni bir satır.
- ↑ aynı sütunu açık olan en yakın üst satıra gider.
- Odaktaki hücre, Iced'in `find_focused` işlemiyle bulunur.

## Web'den ayrılanlar

- **Sabit hücre:** ↑ sabit hücreyi (bilinen istasyonun adı gibi) atlar. Web o satırın salt okunur alanına odaklanır. Masaüstünde sabit hücre bir yazıdır, odak almaz.

## Doğrulama

- `layer_tree::tests`:
  - arama: grup altındaki bulgu, kapalı grup, yalnız grubun adı, boş sonuç, Türkçe I;
  - basılan satırın tuşları: oklar, Home, End, ←, →, Enter, Boşluk, F2, Delete'in sorusu;
  - tuşların seçimin satırından başlaması ve satırsız Delete;
  - klavyeyi geri alanlar: çizim, komut satırı, şeritten komut, arama.
- `calc::grid::tests::the_arrows_find_the_cell_and_the_row_above`.
- Görüntüler (`cargo test -p kentos-desktop layer_tree::screens -- --ignored --nocapture`): `.run/shots/katman-ara-*`, `katman-ara-yok-*`, `katman-klavye-*`; koyu ve açık, 1440×900 ve 1100×650.
