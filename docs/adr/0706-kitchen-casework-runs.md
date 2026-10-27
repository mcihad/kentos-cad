# ADR 0706: Düz mutfak dizisi

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-LIB-02'nin ilk dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-LIB-02; ADR 0702 (KentOS kitaplığının mutfak blokları), ADR 0318 (dinamik
  blokların parametreleri), ADR 0315 (`cad.blocks.insert`).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0702 KentOS kitaplığına gerçek boylu Alt dolap (60 cm, Genişlik 30–120 cm, 15 cm adım), Evye (80 × 50 cm),
Ocak (60 × 52 cm) ve Üst dolap (35 cm derin, kesikli) bloklarını ekledi; bir mutfak bunları Blok ekle ile tek tek,
genişliği Öznitelikler'den vererek ve tezgâhı ayrı alan olarak çizerek kuruluyordu: yedi sekiz blok, ayrı ayrı
geri alma adımları, modül toplamı ve dolgu elle hesap.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Vectorworks 2025 — Creating cabinets](https://app-help.vectorworks.net/2025/eng/VW2025_Guide/FurnFixt/Creating_cabinets.htm):
  alt, üst ve yüksek dolap stilleri; *Duplicate Along Line* aynı enli dolapları çizilen çizgi boyunca dizer, çizgi
  dolap eninin katı değilse “the remainder of the line is not filled”; yerleştirmeden sonra bir tık yönü verir.
- [Vectorworks 2025 — Creating countertops](https://app-help.vectorworks.net/2025/eng/VW2025_Guide/FurnFixt/Creating_countertops.htm):
  tezgâh seçili dolaplardan ya da elle çizilir; altındaki dolaplara bağlı değildir, bağımsız düzenlenir; evye
  sembolü tezgâhta delik açar.
- [Autodesk Learn — Fixtures, furnishings, and equipment](https://www.autodesk.com/learn/ondemand/module/add-details-to-the-revit-model/unit/3VuyosQ4iIEEhSPyW9kFV3):
  Revit'te dolaplar Casework kitaplığının aileleridir (Base/Wall Cabinets); planda üstteki dolap ayrı çizgi
  stiliyle gösterilir.
- [Graphisoft topluluğu — Archicad 29 Kitchen Cabinets](https://community.graphisoft.com/t5/Libraries-objects/Archicad-29-Kitchen-Cabinets-vs-Archicad-28-Kitchen-Cabinets-Why/td-p/690903)
  (kullanıcı raporu, resmi belge değil): kitaplık dolapları tek tek nesnelerdir; tezgâh çoğu ofiste döşeme/alan
  olarak ayrıca çizilir.

KentOS kararı: Vectorworks'ün *çizgi boyunca dizme* yöntemi alınır, ama kalan sessiz bırakılmaz: dizinin sonunda
kesik olmayan **dolgu** çizilir ve uyarıyla söylenir. Tezgâh dizinin aynı adımında, dolaplara bağlı olmayan düz bir
alan olarak yazılır (Vectorworks'teki gibi bağımsız). Dolaplar KentOS'un var olan dinamik bloklarının
yerleştirmeleridir; yeni nesne türü ya da KCAD alanı yoktur. Alınmayanlar: L/U dizinin köşe dolabı ve kör köşe
çözümü tek komutta, bulaşık makinesi/buzdolabı modülü, tezgâhta evye deliği, süpürgelik ve tezgâh arkası, 3B,
duvara bağlı izleme.

## Karar

### 1. `cad.casework.run` v1

Girdi: katman, dolapların sırtının dayandığı çizginin başı `from` ve sonu `to`, yanı `side` (`left` öntanımlı:
`from`'dan yürürken sol; `right`), modül eni `module` (öntanımlı 0,60 m; Alt dolap'ın Genişlik kuralına uymalı:
0,30–1,20 m, 0,15 adım), tezgâh derinliği `depth` (0,30–1,20 m, öntanımlı 0,60), evye modülü `sink` ve ocak modülü
`cooktop` (1'den sayılan sıra), `upper` (üst dolap), renk ve çizgi kalınlığı.

Hesap (`kentos_geometry_core::geom::casework`): modül sayısı, evye istenmişse 0,90 m'lik bir modülle, çizgiye
sığan en çok modüldür (1e-9 payla); modüller `from`'dan dizilir; kalan > 1 mm ise dolgudur. Yazılanlar, tek geri
alma adımında (“Mutfak dizisi”), bu sırayla: her modülün Alt dolap yerleştirmesi (Genişlik varsayılandan farklıysa
parametresiyle; sağ yanda aynalı), evye modülüne 5 cm içeride Evye, ocak modülünün ortasına 4 cm içeride Ocak, istenmişse
her modülün üstüne Üst dolap, dolgu (0,60 m derin kapalı çoklu çizgi) ve bütün çizgi boyunca tezgâh (alan).

Bloklar adlarıyla bulunur: çizimde aynı adlı blok varsa o kullanılır (Genişlik parametresi olmalı), yoksa KentOS
kitaplığınınki aynı adımda getirilir (kimliği çizimde varsa yeni kimlikle). Çıktı yazılan nesnelerin kalıcı
kimlikleri, getirilen blokların adları, modüller (tür, başlangıç, en) ve dolgudur; plan aynı nesneleri kimliksiz
verir.

Retler, sırayla: `not_finite`; `invalid_casework` (başı sonuyla aynı, derinlik aralık dışında, blok yok ya da
Genişlik parametresi yok, modül veya evye eni Genişlik kuralına uymuyor, bir modüle bile yetmeyen çizgi, evye/ocak
dizinin dışında ya da aynı modülde, Üst dolap'ın eni uymuyor); `invalid_line_weight`; revizyon; katman. Uyarılar:
`layer_hidden`, `casework_filler`.

### 2. Araç: Mutfak dizisi

Mimari › Yerleştir'de, Rampa'nın yanında. Başa ve sona tıklanır (ya da boy yazılır), sonra dolapların duracağı tarafa
tıklanır; Enter gösterilen tarafla yazar. Önizleme komutun planıdır: dolaplar bloklarından, Genişlik'leriyle açılmış,
üst dolaplar kesikli; etiket modül sayısını, evye ve ocağın modülünü, tezgâhı ve dolguyu söyler; ret kırmızı yazılır
ve hiçbir şey yazılmaz. Görünür ayarlar: Modül eni (30–120 cm listesi), Tezgâh derinliği, Evye ve Ocak (Otomatik,
Yok ya da modül), Üst dolap, Yan (imleç tarafı, çizginin solu/sağı). Modül, derinlik ve üst dolap oturum belleğinde
kalır.

Evye ve Ocak Otomatik başlar ve seçtiği modül ayarlarda ve etikette görünür: evye iki ya da daha çok modüllü dizide
2. modül; ocak üç ya da daha çok modülde sondan ikinci, olmazsa son, evyeden en az bir modül ayrık. Bu KentOS'un
yerleşim önerisidir, ergonomi standardı diye sunulmaz.

### 3. Python ve MCP

Aynı komut `kentos.cad.casework.run` ve MCP aracıdır; yalnız yeni nesne ve blok yazdığı için yıkıcı listede değildir.

## Sınırlar

- L ve U mutfak iki ayrı diziyle kurulur; köşe dolabı, kör köşe ve iki dizinin tezgâhının birleşmesi açıktır.
- Bulaşık makinesi, buzdolabı ve uzun dolap modülü; tezgâhta evye deliği; süpürgelik ve tezgâh arkası; 3B ve kotlar
  açıktır.
- Dizi duvara bağlı değildir: duvar taşınınca izlemez; tek modülün sonradan düzenlenmesi blokların kendi
  Öznitelikler'iyledir (dizi yeniden hesaplanmaz).

## Doğrulama

- `python3 scripts/fixtures/casework_cases.py --check`: `fixtures/commands/v1/cad.casework.run.json`'un 36 durumu
  (sol/sağ yan, eğik çizgi, dolgu, modül ve derinlik, evye ve ocak, üst dolap, çizimin kendi blokları, kitaplık
  bloğunun kimliği alınmışken yeni kimlik, her ret) KentOS kodu olmadan; saklanan noktalar çekirdeğin kayan nokta
  adımlarıyla bit bit. `KENTOS_CASES_ONLY=cad.casework cargo test -p kentos-native-application --test all
  every_command_case`; Python koşucusu (`KENTOS_CASES_ONLY=cad.casework .run/py/bin/python -m unittest
  python.tests.test_command_cases`); `cargo test -p kentos-mcp`.
- `cargo test -p kentos-interaction --test all casework` (başı, sonu ve yanı tıklanan dizi tek adımda; yazılan boy,
  Enter ve ayarlar; kısa dizinin reddi) ve `--lib casework` (Otomatik evye/ocak).
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=mutfak-dizisi,mutfak-l-yazildi,mutfak-kisa,mutfak-serit cargo test -p
  kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`; `.run/shots/arac-mutfak-*`.
