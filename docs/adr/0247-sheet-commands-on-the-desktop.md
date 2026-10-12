# ADR 0247: Pafta komutlarının masaüstündeki yerleri ve Ad ver

- **Durum:** kabul edildi (2026-10-12). [plan/cad.md](../../plan/cad.md) §24.1'in ikinci işi (TODOS.md `CADP-02`, plan
  `CAD-AUD-08`). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0164 (pafta düzeni), ADR 0228 (web'e ara, donmuş envanter), ADR 0244 (CAD işlem manifesti),
  `docs/inventory/README.md` (`apps/desktop/equivalents.json`).
- **Web:** gerekmez; web'de bu komutlar zaten vardır.

## Bağlam

Envanterin masaüstü sütununa göre `sheet.*`'in 59 komutundan yalnız `sheet.model` masaüstündeydi. Oysa pafta tasarımcısı
(`kentos-sheet-ui`) masaüstündedir. Kendi şeridi, sekmeleri, paftalar listesi ve öğe ağacı, denetçisi ve kısayolları vardır; ama
web'in komut kimliklerini kullanmaz. Bu yüzden `ported.json`'a girmemiştir.

Manifest (ADR 0244) bu sütunu okuduğu için CAD tarafında 62 öğe masaüstünde yok görünüyordu.

Kaynak tek tek okununca iki gerçek eksik çıktı: “Paftaya ad ver” ve “Öğeye ad ver”in işleyicileri (`Message::RenameSheet`,
`Message::RenameItem`) vardı, ama onları tetikleyen bir düğme ya da tuş yoktu.

## Karar

1. **Yerler `equivalents.json`'da.** 58 komutun her biri masaüstündeki yeriyle yazılır: şeridin düğmesi ya da menüsü, kısayolu, liste ya
   da ağacın sağ tıkı, kaynağın dosyası. Durumlar:
   - 54 komut `implemented`;
   - `sheet.atlas` ve `sheet.grid` `pending`: şeritteki düğmeleri soluk, atlasın düzenleyicisi yok;
   - `sheet.zoomIn` ve `sheet.zoomOut` `partial`: yalnız tekerlekle, düğmesi ve kısayolu yok.
2. **Ad ver yerinde yazılır.** Web'in listesi ve ağacı gibi:
   - paftalar listesinin sağ tıkında **Ad ver**: paftanın adı listede bir kutu olur;
   - öğe ağacının sağ tıkında **Ad ver** ya da tek öğe seçiliyken **F2**: öğenin adı ağaçta yazılır (KentOS UI ağacının `rename`
     düzenleyicisi);
   - Enter ya da dışarı tıklamak adı yazar; boş ad yazılmaz. Esc adı olduğu gibi bırakır; yazarken Esc seçimi bırakmaz.
   - Her ad bir geri alma adımıdır (pafta kipinin kendi geri alması). Kutu açılınca klavye ona geçer: ev sahibine yeni bir etki
     `Effect::FocusRename` gider.

## Uygulama

- `crates/sheet-ui`:
  - `designer.rs`: `Renaming` ve `Effect::FocusRename`;
  - `message.rs`: `StartRenameSheet`, `StartRenameItem`, `RenameText`, `RenameSubmit`, `RenameCancel`;
  - `update.rs`: işleme, F2 ve Esc;
  - `panels.rs`: liste ve ağaç.
- `apps/desktop/src/sheets.rs`: odak.
- `apps/desktop/equivalents.json`: 58 girdi.
- Manifest yeniden yazıldı. Masaüstünde olmayan CAD tarafı öğeleri 62'den 8'e indi: dört pafta komutu ile beklemedeki
  `file.export.pdf`, `file.print`, `workspace.disaster` ve `workspace.plan3d`.
- `package.json`'ın `rust:test:desktop`'ına `kentos-sheet-ui` eklendi.

## Doğrulama

- `cargo test -p kentos-sheet-ui`: 81 test geçti. Yeni `a_sheet_and_an_item_are_renamed_in_place` şunları sınar: adın yazılması ve
  geri alınması, Esc, boş ad, F2 ve yazarken Esc.
- Yolda bulunan eski hata: `duplicate_delete_align_and_keys`'in iki tuşu Ctrl'dü, tasarımcı platformun komut tuşunu (macOS'ta ⌘)
  bekler. Bu test hiçbir takımda koşmadığı için macOS'a geçişte görülmemişti; artık `Modifiers::COMMAND` gönderir.
- `cargo test -p kentos-desktop sheet` 17 test geçti. `kentos-sheet-ui` ve `kentos-desktop` için clippy `-D warnings` temiz.
- Resimler: `cargo test -p kentos-sheet-ui --test screens rename_screens -- --ignored --nocapture` →
  `.run/shots/sheet-desktop/{pafta,grafit}-ad-ver-{pafta,oge}-{1440,1100}.png`.
- `python3 scripts/cad/manifest.py --check` geçer.

## Sonuçlar

- `kentos-sheet-ui`'ın testleri hiçbir takımda koşmuyordu; bu yüzden macOS hatası görülmemişti. Artık `pnpm rust:test:desktop`'ta
  koşar.
- Pafta komutları katalogda değildir: Python ve AI paftayı düzenleyemez. Manifest onları `none` olarak sayar; çözüm `CADP-09` ve
  planın §17'sidir.
