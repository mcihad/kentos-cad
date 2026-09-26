# ADR 0061: Masaüstünde Ölçülendirme aracı

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.6, §4.7; TODOS.md `UX-01`, `UX-06`, `UI-11`; ADR 0021 (native araç oturumu), 0055 (çizimin yazıları), 0057 (`cad.entities.create`), 0060 (yazı kutusu)
- **Sahibin yönü (26 Eylül):** web'deki araçlar ve düzenleyiciler masaüstüne birebir taşınır.
- **Web ajanının tarifi (26 Eylül):**
  - `tools/dimensionTool.ts` koddan okundu.
  - Web'in dört düzeltmesi aynı gün main'e alındı (`30ffa33`):
    - Ctrl+Z önce seçilen daireyi ya da son kenarı bırakır;
    - açıda yazılan yarıçap işaretsiz alınır;
    - yarıçap ve çap yazılan sayı almaz;
    - ileti "Açı ölçüsü eklendi" biçimindedir.
  - Hizalı ve doğrusal ölçüde yazılan sayı, ölçü çizgisinin işaretli uzaklığıdır. Tarif bunu ayrıca doğruladı.

## Bağlam

- Masaüstünde Ölçülendirme (`tool.dimension`) "web'de var; masaüstüne henüz taşınmadı" diyordu. Şeridin Açıklama grubunda soluktu.
- Masaüstü ölçüleri zaten çiziyor (ADR 0055) ve değerlerini çift tıkla düzenliyordu (ADR 0060). Eksik olan, ölçüyü koyan araçtı.

## Karar

### Araç (`crates/native/interaction/src/dimension.rs`)

- **Biçimler ve tuşları** web'in sırasıyla: Hizalı (H), Doğrusal (D), Açı (A), Yarıçap (R), Çap (Ç; Ç'siz klavyede C).
  - Biçim yalnız bir şey seçilmemişken değişir.
  - Biçim, doğrusal kilidi ve Köşeden uygulama açık kaldıkça hatırlanır (`Memory`, web'in statik alanları).
- **İstemler** web'inkilerdir, kelimesi kelimesine. Önce adım, ardından aracın kendi seçenekleri, en sonda öbür biçimler gelir:
  - "Ölçü: hizalı ölçünün ilk noktasını belirtin [Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç)]";
  - "ikinci ölçü noktasını belirtin"; "ölçü çizgisinin yerini gösterin ya da mesafe yazın";
  - doğrusalda iki noktadan sonra "[Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): imleçten]";
  - açıda "açı ölçüsü için birinci kenara tıklayın [Köşeden (K) / …]", "ikinci kenara tıklayın", "yayın yerini gösterin ya da yarıçap yazın";
  - Köşeden: "açının köşesini gösterin", "birinci kolun üzerinde bir nokta gösterin", "ikinci kolun üzerinde bir nokta gösterin";
  - "yarıçapı (çapı) ölçülecek daireye ya da yaya tıklayın", "ölçünün doğrultusunu gösterin; daireden dışarı çekince yazı dışarı alınır".
- **Hizalı ve doğrusal:** iki nokta, sonra ölçü çizgisinin yeri.
  - Doğrusalın doğrultusu kilitli değilse ölçü çizgisinin konduğu yerden seçilir: noktaların yanı ΔX, üstü ya da altı ΔY. Bu, AutoCAD'in kuralıdır (`linear_angle_for`).
  - Yazılan sayı ölçü çizgisinin işaretli uzaklığıdır: ölçülen doğrultunun solu artıdır.
- **Açı:** iki düz kenar tıklanır. Kenarın katmanı kilitli olsa da ölçülür, çünkü web'in `pickEdge`'i süzmez.
  - Boş yer ya da düz kenarı olmayan nesne: "Açının kenarı olarak düz bir çizgiye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin."
  - Paralel ikinci kenar: "Kenarlar paralel; aralarında açı yok."
  - Yay, imlecin bulunduğu açıya çizilir. Kollar, kenarların tıklandığı yere kadar uzanır (`edge_arms`).
  - Köşeden seçilirse köşe ve her koldan bir nokta verilir (`vertex_arms`).
  - Yazılan yarıçap işaretsiz alınır.
- **Yarıçap ve çap:** bir daire, yay ya da çoklu çizginin yay parçası tıklanır.
  - Başkası tıklanırsa: "Bir daireye, yaya ya da çoklu çizginin yay parçasına tıklayın."
  - Sonra doğrultu gösterilir; daireden dışarı çekmek yazıyı dışarı alır (`radial_dimension`). Yazılan sayı alınmaz.
- **Seçme:** kenar ya da daire beklenirken kenet kapalıdır. İmlecin altındaki nesne vurgulanır.
- **Yazı yüksekliği** 2,5 kâğıt milimetresidir; projenin pafta ölçeği onu metreye çevirir.
- **Yazma:** ölçü `cad.entities.create` ile etkin katmana yazılır, tek geri alma adımıdır ("Ekle"). Hizalı ölçü web'deki gibi biçimsiz yazılır.
  - İleti ölçünün değeriyle söylenir: "Hizalı ölçü eklendi: 10.000", "Açı ölçüsü eklendi: 100.0000 g", "Yarıçap ölçüsü eklendi: R 4.000", "Çap ölçüsü eklendi: Ø 8.000".
  - Değer projenin birimindedir: uzunluk birimsiz, açı projenin açı biriminde.
  - Kilitli katmanda komutun iletisi söylenir. Yazılsın ya da reddedilsin, araç sonraki ölçüye başlar; biçim kalır.
  - Ölçü oluşmayan yerde (çakışan noktalar, sıfır uzunluk) şu söylenir ve araç bekler: "Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır."
- **Tuşlar:**
  - Enter, Boşluk ya da kısa sağ tık: bir şey seçilmişse yeni ölçüye başlar, seçilmemişse araçtan çıkar.
  - Esc araçtan çıkar; web'in aracının kendi iptali yoktur.
  - Ctrl+Z en yeni adımı geri alır:
    - önce seçilen daireyi ya da son kenarı bırakır;
    - noktalar varsa ölçü baştan başlar;
    - yoksa çizimin geri alması az önce eklenen ölçüyü kaldırır ("Geri alındı: Ekle", web'deki gibi).
- **Önizleme:**
  - Seçilen kenarlar ve daire kenet renginde, 2 px çizilir.
  - Ölçünün yeri gösterilirken ölçü çizgileri vurgu renginde, değeri imlecin yanında görünür.
  - Noktalar alınırken araç öbür nokta araçları gibi çizgiyi, uzunluğu ve semti gösterir.

### Ortak iz

- `fixtures/interaction/v1/dimensions.json` şunları iki platformda oynatır:
  - hizalı ölçü ve çizimin geri alması;
  - Düşey (X) kilidiyle yazılan mesafeli doğrusal ölçü ve Yön (O);
  - iki kenar arasında açı: boş yer ve paralel kenar uyarısı, kilitli katmandaki kenar da;
  - dairenin yarıçapı ve Ctrl+Z'nin önce daireyi bırakması.
- Oynatıcılar ölçünün ölçülen noktalarını, açıda köşesiyle, okur (`interaction.mjs` `shape`, `traces/player.rs` `Seen`).

### Komut satırında değerler

- Masaüstü, çalışan komuta verilen değeri ya da seçeneği komutun kısa adı sanıyordu. X seçeneğine basınca geçmişte "X Patlat" yazıyordu; A "Yay", O "Ötele" oluyordu.
- Web "› X" yazar. KentOS UI'ın komut satırına bu yüzden yeni bir satır türü eklendi: `Entry::Value`, çalışan komuta verilen değer ya da seçenek.
  - Yazıldığı gibi gösterilir, bir komutun adıyla eşlenmez.
  - Yukarı okla eskisi gibi geri çağrılır.
- Değer alanı, komut satırı ve seçenek düğmeleri bununla yankılar (`App::echo_value`).

## Web'den ayrılanlar

- **Kenar ya da daire beklenirken yazılan nokta** (kapandı): web bu noktayı aracın noktalarına ekliyordu. Nokta hiçbir kenarı seçmediği hâlde araç "taze" olmaktan çıkıyordu.
  - Masaüstü yazılanı anlaşılmamış sayar.
  - Web ajanı aynı gün web'i de böyle yaptı (`bc4f8ad`, duman testiyle).
- **Yazma yolu:** web ölçüyü belgeye doğrudan yazar (`doc.add`), masaüstü `cad.entities.create` ile yazar.
  - Geri alma adımının adı ikisinde de "Ekle"dir.
  - Web ajanının önerisi web'i de ürün komutuna taşımaktır.

## Doğrulama

- `crates/native/interaction/tests/dimension.rs` (10 test; beklenenler elle hesaplandı):
  - web'in istemleri ve seçenekleri;
  - hizalı ölçü ve işaretli yazılan mesafe;
  - doğrusalın imleçten ve kilitle doğrultusu;
  - ölçü oluşmayan yer;
  - kenarlar arasında ve köşeden açı (dik ve geniş açı, işaretsiz yarıçap);
  - yarıçap ve çap (sayı alınmaz, C);
  - Ctrl+Z ve Enter'ın sırası;
  - kilitli etkin katman;
  - kenar beklenirken yazılan nokta.
- İz `dimensions`: web `pnpm e2e:interaction` (31 iz × 3 varyant) ve masaüstü `cargo test -p kentos-desktop traces` geçti.
- `pnpm rust:test`, `pnpm rust:test:desktop` (clippy temiz), `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm inventory:check`.
- Görüntüler (`preview::screens`, `.run/shots/olcu-*`), koyu ve açık, 1440×900 ve 1100×650:
  - hizalı ölçünün önizlemesi;
  - Düşey kilitli doğrusal ölçü ("› X" yankısıyla);
  - iki kenar arasında açı;
  - dairenin yarıçapı.
