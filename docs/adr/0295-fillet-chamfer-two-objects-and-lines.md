# ADR 0295: Köşe yuvarla ve Pah'ta iki nesnenin düz kenarları ve çizgilerin tüm köşeleri

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-09`: “Fillet/chamfer; iki nesne, polyline, all corners,
  radius 0, no-trim ve geçersiz köşe durumlarını kapsasın.” Kapsamı ben belirledim. Sahibin bildirimi (14 Ekim): Tüm köşeleri yuvarla
  ayrı çizgilerle (doğrularla) çizilmiş şekilde çalışmıyordu.
- **Bağlam belgesi:** ADR 0250 (anlamsal Köşe yuvarla ve Pah), 0047, 0258 (kapalı çoklu çizgi).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

`cad.entities.fillet` ve `.chamfer` yolun köşesini, iki çizgiyi ve çoklu çizgi ile alanın bütün köşelerini işliyordu. İki nesnenin köşesi
yalnız iki çizgi olabiliyordu: AutoCAD'in FILLET'inin çizgi ile çoklu çizginin ucu, iki çoklu çizginin uçları ve aynı çoklu çizginin iki
ucu (yolu kapatma) durumları yoktu. Tüm köşeler yalnız çoklu çizgide ve alanda çalışıyordu: dört ayrı çizgiyle çizilmiş bir dikdörtgen
seçilince araç “seçimde çoklu çizgi yok” deyip çıkıyordu.

## Karar

| Durum | AutoCAD | KentOS |
| --- | --- | --- |
| İki çizgi | kenarlar kırpılır ya da uzar, yay ya da pah yeni nesne | aynı (ADR 0250) |
| Çizgi ve açık çoklu çizginin uç kenarı | çizgi ve yolun ucu | çizgi kısalır ya da uzar, yolun serbest ucu yayın ya da pahın ucuna gelir; yay ya da pah ilk seçilen nesneden yeni nesne |
| İki açık çoklu çizginin uç kenarları | aynı | aynı |
| Aynı açık yolun ilk ve son kenarı | yol kapanır | yol kapanır: kapanan kenar yay (çıkıntı `tan(süpürme/4)`, son köşeden saat yönünde dönüyorsa eksi) ya da pah; sıfırda uçlar buluşur, son köşe düşer |
| Yolun iç kenarı, yay kenarı, kapalı yol ve alanın kenarı | yolun köşesi | ret `not_a_corner`; iç köşe kendi köşesiyle (vertex) verilir |
| Köşe yolun iç köşesine düşerse | — | ret `not_a_corner` (kenarın öbür yanına tıklanır) |
| Tüm köşeler: çoklu çizgi ve alan | Polyline seçeneği | aynı (ADR 0250) |
| Tüm köşeler: uçları buluşan çizgiler | yok (tek tek) | iki çizginin ucunun bit bit buluştuğu ve aynı doğrultuda olmadıkları her yer köşe; çizgilerin uçları geri çekilir, yay ya da pah önce verilen çizgiden yeni nesne; bir çizginin iki köşesi birlikte sığmazsa ikisi de atlanır; ikiden çok uç buluşan yer köşe değildir |
| Sıfır yarıçap ya da sıfır pah | kenarlar köşede buluşur | aynı; Kırp kapalıyken ve tüm köşelerde ret `invalid_distance` |
| Kırp kapalı | yalnız yay ya da pah | aynı; yayın açıları [0, 2π) |

Komutun girdisi değişmedi: `corner.kind = "lines"` iki “düz kenar” alır (`first`, `second`; her biri nesnenin kimliği ve tıklanan nokta;
çoklu çizgide noktaya en yakın kenar). Tüm köşeler `corner.kind = "all"` verilen kimliklerdeki çizgileri birlikte işler. Retler:
`not_a_corner` iletileri (“Çoklu çizgide yalnız ilk ya da son kenar…”, “Seçilen kenar yay…”, “Köşe çoklu çizginin serbest ucunda
değil…”, “Aynı uç iki kez seçildi…”, “Tüm köşeler çoklu çizgide, alanda ve uçları birbirine değen çizgilerde işlenir.”, “Çizgilerin
buluştuğu yerde köşe yok…”).

Hesap uygulama katmanında ortak çekirdekle: kenarın tarafı ve yay ya da pah çekirdeğin `fillet_lines`, `chamfer_lines`, `fillet_arc`,
`chamfer_line`'ı; yeni olan yalnız hangi köşenin ve hangi ucun değiştiği (`corner::side_of`, `lines_corner`, `line_corners`). Araçlar
aynı işlevleri kullanır: Köşe yuvarla ve Pah'ın iki tıklaması (`corner_of_picks`), Tüm köşeleri yuvarla ve Tüm köşelere pah'ın önizlemesi
ve yazması.

## Sonuçlar

- Doğrularla çizilmiş bir yapının köşeleri tek adımda yuvarlanır ya da pahlanır.
- Ajan ve Python iki farklı nesnenin köşesini köşe hesaplamadan tek çağrıyla yapar.

## Kalanlar

- Yay ve daireyle köşe yuvarlama (AutoCAD'in çizgi–yay, yay–yay FILLET'i).
- Çizgi ile yolun köşesinde yayın yola katılması (AutoCAD'in PEDITACCEPT'li birleştirmesi); KentOS'ta yay ayrı nesnedir.

## Doğrulama

- `python3 scripts/fixtures/corner_sides_cases.py --check` (iki dosyada 52 durum: `cad.entities.{fillet,chamfer}.sides.json`; düz
  geometri kesin, yarım açının sin ve tan'ı 60 basamakla), `python3 scripts/fixtures/edge_command_cases.py --check`;
  `cargo test -p kentos-native-application --test all`; `cargo test -p kentos-interaction --test all` (`a_chamfer_joins_a_line_and_a_polyline_end`,
  `every_corner_of_lines_meeting_end_to_end_is_rounded_in_one_step`); `pnpm py:test`; `python3 scripts/cad/manifest.py --check`.
- Resim: `KENTOS_SHOTS_ONLY=kose-cizgiler,kose-yol-ucu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
