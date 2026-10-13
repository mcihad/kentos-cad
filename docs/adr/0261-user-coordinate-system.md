# ADR 0261: Kullanıcı koordinat sistemi (2B UCS)

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §6.1 `CAD-DATA-09`, `CAD-DATA-10`, `CAD-DATA-11`. Kapsamı
  ben belirledim (sahibin sözleri: “CAD data bölümüne geç ve orayı tamamla”, “durmadan devam et”; “Araç = komut” kuralı, 13 Ekim).
- **Bağlam belgesi:** ADR 0165 (proje türleri; eksen ve açı düzeni, koordinat ekseni simgesi), 0166 (sayısallaştırma kilitleri),
  0163 (kenet ekleri: karelaj), 0256 §4–§5 (ölçünün `frame`'i, `conventions`), 0253 (komut protokolü), 0248 (ayar komutları),
  0025 ve `docs/specs/kcad-v2.md`.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

- Çizimin tek ekseni vardı: dünya (x doğu, y kuzey; CAD'de X, Y; CBS'de Y, X). Döndürülmüş bir bina, eğik bir parsel ya da yolun
  eksenine göre çizmek için kullanıcı her noktayı kafasında çeviriyordu.
- Ölçünün `frame`'i (ADR 0256 §4) UCS gelene dek hep `world`'dü; özetin `conventions`'ı UCS söylemiyordu.
- Masaüstünün kamerası dönmez: ekranın dönüşü yoktur, UCS onun yerine geçmez; kamera ile UCS ayrı kalır.

## Karar

### 1. UCS: projenin ayarı

- **UCS**, dünyada bir başlangıç noktası (`origin`) ve x ekseninin dünyanın x'inden (doğudan) saat yönünün tersine açısıdır (`angle`,
  radyan). y ekseni x'e diktir, saat yönünün tersine. Ölçek ve aynalama yoktur: UCS dik ve birim uzunlukludur, uzunluk ve alan değişmez.
- **Dünya** UCS'i `origin` (0, 0), `angle` 0'dır. Dönüşümler: `to_world(p) = origin + R(angle)·p`, `to_ucs(w) = R(−angle)·(w − origin)`.
- **Proje ayarı** `ucs` (yoksa dünya) ve **adlı UCS'ler** `namedUcs` (ad, başlangıç, açı; adlar kırpılmış, 1–64 karakter,
  denetim karaktersiz, tekil (büyük küçük harf ayrıdır), en çok 100). Açı [0, 2π) aralığına indirgenerek saklanır; sayılar sonlu, başlangıç
  10 000 km'den yakın.
- UCS değişikliği bir **ayar değişikliğidir** (ADR 0049, 0248): proje kirlenir, revizyon artar, geri alma adımı değildir. Nesnelerin
  koordinatları değişmez: UCS yalnız girişin, gösterimin ve yardımcıların eksenidir (CAD-DATA-10'un “mevcut kaynak geometriyi
  sessizce taşımasın”ı).
- **Önceki UCS** oturumun geçmişidir (en çok 10): masaüstü her değişiklikte (araç, Python, ajan) eskisini saklar, Önceki onu açık bir
  çerçeve olarak verir. Geçmiş çizimde ve dosyada değildir; başka bir çizim açılınca sıfırlanır.

### 2. `cad.ucs.set` v1

Girdi `to` (tür etiketli) ve isteğe bağlı `expectedRevision`:

| `kind` | Alanlar | Yeni UCS |
|---|---|---|
| `world` | — | dünya |
| `frame` | `origin`, `angle` | verildiği gibi (Önceki, Python) |
| `origin` | `point` | başlangıç `point`, açı aynı |
| `rotate` | `angle` | açı + `angle`, başlangıç aynı |
| `twoPoints` | `origin`, `xPoint` | başlangıç `origin`, x ekseni `xPoint`'e doğru |
| `object` | `uid`, `at` | nesneye hizalı (§3) |
| `named` | `name` | adlı UCS'in kendisi |
| `save` | `name` | geçerli UCS adla saklanır; aynı adlı varsa yerine (`ucs_replaced` uyarısı) |
| `delete` | `name` | adlı UCS silinir; geçerli UCS değişmez |

Çıktı: `ucs` (geçerli UCS: `origin`, `angle`, varsa `name`), `named` (adlı UCS'lerin adları, sırasıyla), `changed` (ayar değişti mi)
ve `revision`. Aynı UCS'e geçmek bir şey değiştirmez (`changed: false`, revizyon aynı). Geçerli UCS adlı bir UCS'e eşitse `name` onu
söyler (gösterimde “UCS: Bina”).

Retler (sırasıyla): girdinin kendisi `not_finite` (bir sayı sonlu değil), `far_point` (10 000 km'den uzak), `invalid_ucs_name` (boş,
64'ten uzun ya da denetim karakterli ad), `same_points` (iki nokta 1 µm'den yakın), `invalid_uid`; sonra `invalid_revision`,
`revision_conflict`; sonra çizime göre `entity_not_found`, `object_without_axis` (§3'te olmayan tür), `unknown_ucs` (adı olmayan UCS),
`too_many_ucs` (101. ad). Uyarı: `ucs_replaced`. Yollar `to.origin`, `to.angle`, `to.point`, `to.xPoint`, `to.at`, `to.uid`, `to.name`.

### 3. Nesneye hizalı UCS

Tıklanan yer `at` nesnenin hangi kenarının, ucunun ya da yerinin seçildiğini söyler (AutoCAD'in UCS Nesne'si gibi):

- **Çizgi:** başlangıç `at`'a yakın uç, x ekseni öbür uca.
- **Çoklu çizgi, alan** (halkaları ve parçalarıyla), **tarama:** `at`'a en yakın kenar; başlangıç o kenarın `at`'a yakın ucu, x
  ekseni kenar boyunca öbür uca. Yaylı kenarda x ekseni yayın o uçtaki teğetidir, yayın içine doğru.
- **Yay:** başlangıç merkez, x ekseni `at`'a yakın uca.
- **Daire:** başlangıç merkez, x ekseni `at`'a.
- **Elips:** başlangıç merkez, x ekseni büyük eksen.
- **Yazı, blok yerleştirmesi:** başlangıç yerleştirme noktası, x ekseni yazının ya da yerleştirmenin dönüşü.
- **Nokta:** başlangıç nokta, açı geçerli UCS'inki.
- Başka türler (ölçü, kılavuz, eğri, NURBS, tablo, resim, raster, nokta bulutu) `object_without_axis`.

### 4. Girişte ve gösterimde UCS (CAD-DATA-10)

- **Yazılan nokta:** mutlak `x,y` UCS'tedir (CBS'de `Y,X` sırası UCS'in eksenleriyle); göreli `@dx,dy` UCS'in eksenleri boyunca;
  kutupsal açı UCS'in x ekseninden (CAD) ya da UCS'in y ekseninden saat yönünde (CBS'nin semti) okunur. `#ad` ve nesne izleme dünyanın
  noktasıdır. Çekirdeğin `point_from_text_framed`'i (`tools::ucs`).
- **Orto, kutupsal izleme, Dik açı, `<açı` kilidi:** UCS'in eksenlerine göredir; Sapma, Paralel ve Dik nesneye göredir (dünyada).
  Çekirdeğin `constrain_cursor_in` ve `constrain_locked_in`'i girdiyi UCS'e çevirip kuralı uygular, sonucu dünyaya döndürür.
- **Izgara ve karelaj keneti:** ızgara çizgileri UCS'in eksenlerine paraleldir, UCS'in başlangıcından aralık katlarıyla; karelaj
  keneti aynı kafestir.
- **Durum çubuğu ve Koordinat oku:** UCS dünya değilken koordinatlar UCS'tedir ve hücre “UCS” yazar; Koordinat oku dünyayı da verir.
- **Koordinat ekseni simgesi** (ADR 0165 §5): CAD'de UCS'in eksenleriyle döner, sol altta durur (sahibin 12 Ekim isteği); dünyada
  başlangıç karesi vardır, UCS'te yoktur. CBS'de UCS dünya değilken sol altta Y ve X adlı dönmüş simge görünür.
- **Masaüstü:** Görünüm sekmesinde Koordinatlar paneli (CAD ve CBS): Dünya UCS, Önceki UCS, Nesneye hizala, İki noktayla UCS, UCS
  orijini, UCS'i döndür, Adlı UCS (`ucs.world`, `ucs.previous`, `tool.ucsObject`, `tool.ucsTwoPoints`, `tool.ucsOrigin`,
  `tool.ucsRotate`, `tool.ucsNamed`; kayıtları `catalog/v1/additions.json`'da).
- **Ölçü ve tarama:** Doğrusal ölçünün yatay ve düşeyi UCS'in eksenleridir (döndürülmüş doğrusal, açısı UCS'inki); yeni taramanın
  desen açısı UCS'in açısına eklenir. Var olan ölçü ve taramalar değişmez.
- Etiketler, yazıların yeni dönüşü ve blok yerleştirmenin dönüşü bugünkü gibi dünyadadır (AutoCAD'de de yazının dönüşü UCS'e göre
  yazılır; bu bir sonraki adımdır, §7).

### 5. Python ve ajan (CAD-DATA-11)

- **Tel her zaman dünyadır** (ADR 0256 §4): komutların noktaları, nesnelerin koordinatları ve ölçüler dünyada. UCS'te çalışmak isteyen
  çağıran noktaları açıkça çevirir; komut örtük UCS okumaz (CMD-07).
- Özetin `conventions`'ına `ucs` (`origin`, `angle`, `name`) eklenir.
- Python: `Document.ucs` (geçerli UCS, `UcsFrame`; yoksa `WORLD_UCS`), `Document.named_ucs`, `Document.to_world(points, ucs=None)` ve
  `Document.to_ucs(points, ucs=None)` (verilmezse geçerli UCS; `UcsFrame` ya da adlı UCS'in adı), `Conventions.ucs`;
  `kentos.cad.ucs.set` katalogdan (`TwoPointsUcsTo`, `SaveUcsTo` …).
- MCP: `cad.ucs.set` aracı katalogdan (adlı UCS'i silebildiği için yıkıcı listede); `drawing.toWorld` ve `drawing.toUcs` noktaları
  açık bir UCS'le (`ucs`), adlı bir UCS'le (`name`) ya da geçerli UCS'le çevirir, masaüstü tutamacında da; özet `conventions.ucs`'i taşır.
- Ölçünün `frame`'i `world` kalır.

### 6. Dosyalar ve sunucu

- **KCAD v2 şema 49:** ayarların `ucs`'i ve `namedUcs`'i. Yazıcı 49'u yalnız geçerli UCS dünya değilken ya da adlı UCS varken yazar.
- **DXF:** yazıcı başlığa `$UCSORG`, `$UCSXDIR` yazar ve UCS tablosuna adlı UCS'leri; okuyucu bunları alır (sonraya, §7).
- **Sunucu:** proje ayarları JSON olarak gider ve gelir; ayrı sütun yoktur.

### 7. Sonraya kalanlar

- Yazının, ölçünün ve blok yerleştirmenin dönüşünün yeni nesnede UCS'e göre yazılması.
- DXF'in `$UCSORG`, `$UCSXDIR` ve UCS tablosu.
- Görünüm başına UCS (AutoCAD'in UCSVP'si) ve görünümü UCS'e döndürmek (PLAN); kameranın dönüşü yoktur.
- Koordinat ölçüsünün UCS'in eksenlerinde ölçmesi; ölçünün `frame: "ucs"`'i.
- Nokta editörü, koordinat listesi ve Hesap pencerelerinin UCS'te göstermesi (bugün dünyada: kadastro ve ölçme dünyanındır).
- 3B UCS (z ekseni, üç nokta).

## Sonuçlar

- Kullanıcı döndürülmüş bir eksende yazarak, ortoyla ve ızgarayla çizer; çizimin koordinatları dünyada kalır, eski çizimler değişmez.
- Python ve ajan UCS'i görür, noktaları açıkça çevirir; tel dünyadır, örtük bağlam yoktur.

## Doğrulama

- Çekirdek: `tools::ucs`'in testleri; `fixtures/point-input/v1`'in dokuz UCS durumu (elle hesaplanmış, dosyanın notunda).
- Karelaj: `store::snap`'in `grid_frame`'i; ızgara `build_grid_framed`.
- Komut: `fixtures/commands/v1/cad.ucs.set.json` (bağımsız başvuru `scripts/fixtures/ucs_command_cases.py`), yetenek testi.
- KCAD: `ucs.kcad` ve bozuk dosyalar, `kcad_v2_reference.py --check`.
- Masaüstü resimleri: `KENTOS_SHOTS_ONLY=ucs-dunya,ucs-nesne,ucs-iki-nokta,ucs-yazilan,ucs-serit cargo test -p kentos-desktop
  tools_screens -- --ignored --nocapture` (`.run/shots/arac-ucs-*`).
- Python ve MCP: `python/tests/test_ucs.py`, `kentos-mcp`'nin `an_agent_sets_a_ucs_and_turns_its_points_explicitly`'si.
