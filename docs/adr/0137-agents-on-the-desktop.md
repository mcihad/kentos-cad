# ADR 0137: Ajanlar masaüstündeki açık çizimde

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0133 ve 0134 (MCP sunucusu), ADR 0132 (Python konsolu); TODOS.md §15 (AI-14, AI-15).

## Bağlam

MCP sunucusunun çizimleri dosyadan ya da buluttan açılan, ekranda olmayan çizimlerdi. Kullanıcının asıl istediği şudur: yapay zekâ ekrandaki çizimde çalışsın, yaptığını kullanıcı o anda görsün ve geri alabilsin.

- AI-14: yapay zekânın değişiklikleri geri alma ve geçmişte görünmeli.
- AI-15: ekrana tıkladığı varsayılmadan, masaüstünün oturumuna bir bağlanma modeli olmalı.

Konsolun yolu hazırdır. Masaüstü bir programın isteklerini açık çizimde başsız sunucunun `rpc`'siyle yanıtlıyor (ADR 0132).

## Karar

**Ajan bağlantısını kullanıcı açar.** Python sekmesindeki bağlantı düğmesi açar ve kapatır; açıkken yanıktır, ipucu yolu ve bağlı ajan sayısını söyler. Kapatınca bağlantılar da kesilir, soket silinir.

**Yerel soket, yalnız bu kullanıcının.**

- Yer: `$XDG_RUNTIME_DIR/kentos-cad/masaustu.sock`. Klasör 0700 açılır, soket 0600'dür; kullanıcının runtime klasörü de yalnız kendisinindir. Başka kullanıcının süreci bağlanamaz; ağ yoktur.
- Runtime klasörü yoksa geçici klasörün altında kullanıcıya özgü bir klasör kullanılır.
- Soketi başka bir KentOS penceresi kullanıyorsa açılış reddedilir. Kapanmış bir pencerenin bıraktığı dosya yeniden kullanılır.
- Unix'e özgüdür. Başka işletim sisteminde düğme "henüz yok" der.

**İstekler** JSON satırlarıdır: `{"id", "method", "params"}` → `{"id", "ok", "result"}` ya da `{"id", "ok": false, "code", "message"}`.

- **Yöntemler:** `hello` (çizimin adı, yolu, revizyonu, nesne sayısı), `run`, `summary`, `layers`, `entities`, `entity`, `measure`.
- **Yanıtlayan:** UI iş parçacığı, açık çizimde, konsolun `kentos_headless::rpc`'siyle.
- **Görünürlük:**
  - Her yazma kendi geri alma adımıdır (komutun adıyla).
  - Komut geçmişine "Ajan: <komut>" diye girer.
  - Kullanıcı hemen görür, Geri al ile geri alır.
- **Masaüstü meşgulken ajan bekler (`busy`):** konsolda kod çalışırken (çalıştırmanın grubu karışmasın diye) ve çizim açılırken.
- **Açık çizim yoksa** cevap `no_document`'tır.
- **Kullanıcıya kalanlar:** geri alma, yineleme ve kaydetme bağlantıdan yapılamaz (`unknown_method`).

**MCP'de `desktop.attach`.**

- Bağlantıya bağlanır; `path` verilmezse `KENTOS_DESKTOP`, yoksa masaüstünün varsayılan yolu kullanılır.
- `hello` ile doğrular ve bir tutamaç verir (`desktop-…`).
- **Aynı araçlar masaüstüne gider:**
  - `drawing.summary`, `drawing.layers`, `drawing.entities` (sayfası 100, en çok 1 000), `drawing.entity` ve `drawing.measure` bu tutamaçla masaüstüne gider;
  - `cad.*` komutları da gider;
  - `drawing.list` masaüstü tutamaçlarını da listeler.
- `drawing.close` bağlantıyı bırakır; çizime dokunmaz.
- `drawing.save`, `drawing.undo` ve `drawing.redo` reddedilir (`desktop_user`): bunlar kullanıcının işidir.
- Masaüstü giderse cevap `desktop_gone` olur ve tutamaç düşer. Yanıtı bir dakika gelmeyen istek zaman aşımıyla döner.

## Sonuçlar

- Bir ajan kullanıcının ekrandaki çizimini okuyup ürün komutlarıyla yazabilir. Değişiklikler kullanıcının geri alma adımlarıdır; komut geçmişinde görünürler (AI-14).
- Bağlantı açık bir karardır; açıkken bu kullanıcının her programı çizime yazabilir. Bu, güvenilir yerel bir kiptir.
- **Açık kalanlar:**
  - ajanın birden çok komutunu tek geri alma adımında toplamak;
  - değişiklikten önce onay (AI-07);
  - görünür alanın ve seçimin ajana verilmesi (AI-15'in görsel bağlamı);
  - Windows'ta adlandırılmış boru.

## Doğrulama

- `cargo test -p kentos-desktop python::`, 26 test:
  - **Ajanın istekleri:** okuma, tek adım olarak yazma ve komut geçmişi; konsol çalışırken `busy`; geri almanın reddi; kapanınca ret.
  - **Soket:** izni 0600; ikinci pencere kullanılan bağlantıyı almaz; gerçek bir bağlantıyla gidiş-dönüş; kapanınca soket silinir.
- **Gerçek ajan** (`-- --ignored`): gerçek `kentos-mcp` süreci MCP'den `desktop.attach` ile bağlanır. Açık çizime `cad.polygon.create` yazar, `drawing.measure` 250 m² verir. Ekrandaki çizim bir nesne artar; komut geçmişinde "Ajan: cad.polygon.create" vardır.
- `cargo test -p kentos-mcp`, 7 test: sahte bir masaüstü soketine bağlanma, okuma, yazma, komutun reddi, kaydetmenin reddi, liste, bırakma, olmayan masaüstü.
- **Resim:** `python-konsol-ajan-*`.
- **Genel:** `cargo test -p kentos-desktop` (530) ve clippy.
