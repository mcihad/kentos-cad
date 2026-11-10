# A2 — Çizimdeki mimari öğelerin tam denetimi

**Kapsam:** veri modeli, plan temsili, ölçeğe göre ayrıntı, düzenleme, açıklama (etiket/ölçü), ilişkiler, çizelge/metraj,
KCAD, DXF/DWG/PDF çıktısı, Python/MCP. Şerit/araç kullanımı ve plan maddelerinin durumu bu raporun konusu değildir.

**Depo:** `/home/cihad/Projects/kentos-cad`, dal `arch-adv-ileri`, HEAD `e40e6dae0` (temiz çalışma ağacı). Tarih: 7 Kasım 2026.

## 0. Yöntem, kanıt sınıfları ve kısaltmalar

- Yalnız okuma yapıldı: `rg`/`grep`/`sed`, `git log`. **Derleme, test, GUI çalıştırılmadı.** Bu yüzden hiçbir davranış bu
  oturumda çalıştırılarak doğrulanmadı; “doğrulandı” denen her şey ADR'lerin kendi kaydıdır.
- Kanıt etiketleri:
  - **[K]** kodda okundu (dosya:satır).
  - **[A]** ADR/plan kaydı; bu oturumda yeniden doğrulanmadı.
  - **[T]** tahmin/çıkarım; kodda doğrudan karşılığı yok ya da davranış çalıştırılmadı.
  - **[P]** meslek pratiği (Türk mimari proje uygulaması); mevzuat madde numaraları bu araştırmada **doğrulanmadı**
    (yalnız kataloğun kendi kaynakladığı İmar Yönetmeliği md. 31 hariç).
- Yol kısaltmaları:
  - `C/` = `crates/shared/contracts/src/`
  - `G/` = `crates/shared/geometry-core/src/`
  - `APP/` = `crates/native/application/src/`
  - `DOM/` = `crates/native/domain/src/`
  - `F/` = `crates/shared/formats/src/`
  - `CMD/` = `crates/shared/commands/src/`
  - `STY/` = `crates/native/style/src/`
  - `CAT/` = `catalog/v1/`
  - `ADR nnnn` = `docs/adr/nnnn-*.md`; `PLAN` = `plan/architecture.md`; `PLAN-ADV` = `plan/architecture-advanced.md`

## 1. Kısa sonuç

1. **Plan çizimi için nesne yelpazesi geniş ve derin:** duvar (düz, yaylı, ≤16 bileşenli, birleşim/düğüm kararlı), kapı ve
   pencere (6 işleyiş, tip/örnek, net geçiş), boşluk/niş/baca, mahal (net/eksen/brüt, boşluk/şaft), döşeme/balkon/teras,
   aks/kolon, merdiven (düz/L/U/döner), rampa, mutfak dizisi, etiket, görünüm işaretleri, çizelgeler ve metraj var. Hepsi
   KCAD'de anlamıyla saklanıyor, Python/MCP katalogdan üretiliyor.
2. **Düşey (kesit/görünüş) model yok.** Duvarda yalnız `height` var; döşemenin katmanı, çatı, kiriş, tavan nesnesi yok.
   Bina kesiti ve cephe görünüşü türetilemiyor; yalnız oda iç görünüşü (ADR 0799) var ve o da kanat çizmiyor. ADR 0347 §3
   otomatik kesiti 19 Ekim 2026'da tasarlamış, gereken verilerin çoğunu “yok” diye listelemiş; liste bugün de büyük ölçüde geçerli.
3. **Ölçeğe göre ayrıntı yalnız duvarda var** (Şematik/Uygulama/Ayrıntılı). Kapı, pencere, kolon, merdiven, döşeme her
   ölçekte aynı çizilir; yalnız yazı boyları kâğıda göre ölçeklenir.
4. **Kalem/çizgi denetimi nesne türü düzeyinde.** Bir kapının kasası, kanadı, yayı ve eşiği; bir duvarın kesit çizgisi ile
   tarama sınırı aynı kalemi alır (bileşen kenarı hariç). “Kesilen kalın / görünen ince” ayrımı ve plan kesme yüksekliği
   ayarı yok (1,10 m sabit, yalnız açıklıklarda kullanılıyor).
5. **Türk proje takımında karşılığı olmayan büyük parçalar:** vaziyet planı ve emsal/TAKS/KAKS hesabı, çatı planı, kiriş,
   kolon–duvar birleşimi, kesit/görünüş türetimi, grafik kapı-pencere listesi, tadilatta açıklığın kendi evresi, otopark,
   asansör nesnesi, peyzaj, giydirme cephe, çatı penceresi, balansmanlı merdiven.

## 2. Ortak altyapı (bütün öğeler için)

### 2.1 Veri modeli

- Mimari nesne türleri `Entity` içinde birinci sınıf: `Wall`, `Opening`, `Space`, `Slab`, `Grid`, `Column`, `Stair`,
  `Ramp`, `Tag`, `ViewMark` [K] `C/entity.rs:1851-1870`. Mobilya ve donatı `Insert` (blok yerleştirme) olarak çizilir ve
  `arch` bilgisi taşır [K] `C/entity.rs:2016`.
- Her mimari nesnenin ortak bilgisi `ArchInfo`: kat, evre, yapım durumu, disiplin [K] `C/arch.rs:565-579`. Evre
  `Existing/Demolish/New/Temporary` [K] `C/arch.rs:386-391`; durum ve disiplin enum'ları [K] `C/arch.rs:493-532`.
- **Açıklığın `ArchInfo`'su yok:** katı ve evresi duvarından okunur [K] `C/entity.rs:2001-2010` (evre),
  `C/entity.rs:2029-2035` (kat); `OpeningFields` alanları [K] `C/opening.rs:281-378`.
- Proje ayarı `ArchSettings`: katlar, duvar ayrıntısı, ölçme yöntemi, evre görünümü, mahal sözlükleri, etkin kat, denetim
  istisnaları, açıklık tipleri, bina programı, modüller, malzemeler, tasarım seçenekleri [K] `C/arch.rs:1253-1325`.
- Kat: yalnız `id, name, elevation, height` [K] `C/arch.rs:1177-1188`. Katın kendi kesme yüksekliği, kaba/ince döşeme kotu
  ve proje sıfırının mutlak kotu yok [K] (alan yok; `rg 'project zero|mutlak kot'` sonuçsuz).

### 2.2 Plan temsili ve kalem

- Nesneler depoda parçalara (`Piece`) açılır: duvar `geom::wall::pieces` (görünüşün ayrıntısı ve ölçeğiyle), aks/kolon/
  merdiven/rampa/etiket/işaret `geom::arch::pieces(shape, view.scale)`, açıklık `geom::opening::pieces(place, opening)`
  [K] `G/store/mod.rs:736-782`.
- **Açıklığın parçaları ölçek almaz, kalem ve renk taşımaz** (yalnız `dashed` bayrağı) [K] `G/store/mod.rs:763-778`.
- Duvarın iki yüzü, uç kapakları ve girintileri kendi kalemini taşımaz (nesnenin/katmanın kalemi); yalnız bileşenler arası
  çizgi bileşenin (ya da bağlı malzemenin) kenar kalemini alır [K] `G/geom/wall.rs:598-612`, `G/geom/wall.rs:740-764`.
- Görünüş tarifinin kalemleri **14 nesne türü kategorisine** göredir (duvar, açıklık, kolon, merdiven, döşeme, mahal, ölçü,
  etiket, yazı, tarama, blok, aks, işaret, diğer) [K] `C/cad_views.rs:260-279`; ADR'nin kendi sınırı: “Kalem nesne türüne
  göredir; katman ya da öznitelik koşuluyla geçersiz kılma … yok” [A] ADR 0783:85.
- Bileşen kenarının çizgi tipi yalnız 4 temel tiptir (continuous, dashed, dashdot, dotted) [K] `C/arch.rs:145`,
  `C/layer.rs:12-17`; projenin karmaşık çizgi tipleri (ADR 0313) bileşen kenarına bağlanamaz [K].
- Ofis standardı 10 katman: A-DUVAR (0,50; duvar ve kolon), A-KAPI, A-PENCERE (0,25), A-MOBİLYA (0,13), A-EKSEN, A-ÖLÇÜ,
  A-YAZI, A-TARAMA, A-REVİZYON, A-YARDIMCI [K] `CAT/standards/mimari-ofis.json:5-14`. Kolon, döşeme, merdiven, mahal, kot,
  evre (yıkılacak/yeni) için ayrı katman önerisi yok.

### 2.3 Ölçeğe göre ayrıntı

- Duvar: `Schematic` (>1:75, dış hat + stil dolgusu), `Standard` (≤1:75, uygulama bileşenleri ve taramaları), `Detailed`
  (≤1:30, bütün bileşenler) [K] `C/arch.rs:25-26`, `C/arch.rs:94-125`; proje ayarı ve görünüş tarifi ölçeği geçersiz kılar
  [K] `APP/walls.rs:286-312`, `C/cad_views.rs:185-189`; model alanında yakınlaştırmaya göre [A] ADR 0696.
- Bileşen taramaları kâğıt boyunda (ölçekle büyür) [K] `G/geom/wall.rs:1-14` (başlık yorumu).
- Öbür nesnelerde ölçek yalnız yazı/ok/balon boyunu belirler: `LABEL_MM 2.5`, `NUMBER_MM 2.0`, `HEAD_MM 3.0`, balon 8 mm
  [K] `G/geom/arch.rs:23-31`; mahal etiketi `LABEL_MM × ölçek` [K] `G/geom/space.rs:480-483`. Açıklık parçasında ölçek
  parametresi yok [K] `G/geom/opening.rs:731`, `G/store/mod.rs:763`.

### 2.4 Plan kesme düzlemi

- `PLAN_CUT_HEIGHT = 1.1` m sabit [K] `C/opening.rs:21`, `G/geom/opening.rs:26`. Yalnız boşluk/niş/baca kesilip
  kesilmediğinde ve merdiven oluştururken (`cut_height` girdisi) kullanılır [K] `G/geom/opening.rs:375-380`,
  `CMD/cad_arch.rs:596`; başka kullanım yok [K] (`rg PLAN_CUT`).
- Sonuç: kesme düzleminin altında kalan alçak duvar (parapet, yarım duvar) yine kesit gibi çizilir; katın/görünüşün kendi
  kesme yüksekliği yok [K]. Kesme üstü kesikli çizim yalnız merdivende (rıhtlar, korkuluk) [K] `G/geom/arch.rs:1390-1418`,
  kesmeyen boşluk/niş/bacada [K] `G/geom/opening.rs:731-788` ve kütüphanenin “Üst dolap” bloğunda [A] ADR 0702:43 var.

### 2.5 İlişkiler (belge izleyicileri)

- Açıklık → duvar (konak): duvar taşınınca/kalınlaşınca açıklık izler; konak yok, duvar dışı, çakışma, derinlik aşımı
  `broken` [K] `DOM/opening_follow.rs:1-13`.
- Mahal → duvarlar (tohum noktasıyla yeniden bulunur) [K] `DOM/space_follow.rs:1-8`; döşeme kenarı → duvarlar [K]
  `DOM/slab_follow.rs:1-5`; kolon → iki aks [K] `DOM/arch_follow.rs:1-9`; etiket → nesne [K] `DOM/tag_follow.rs:1-8`;
  görünüm işareti → adlı görünüm/pafta numarası [K] `DOM/viewmark_follow.rs:1-15`.
- Mimari bağlar (mesafe, paralel, eşit aralık, sabit; duvar çizgisi ve pervaz) [A] ADR 0770, 0780, 0790, 0797.
- **Olmayan ilişkiler:** duvar ↔ kolon birleşimi [A] ADR 0771:56 (“duvar kolona birleşmez; kolon kendi çizilir”);
  duvar tepesi ↔ üst kat/döşeme altı [K] (`WallFields` yalnız `height`, `C/arch.rs:646-650`); merdiven ↔ döşeme boşluğu;
  şaft ↔ katlar (her katta ayrı boşluk) [A] ADR 0708:26.

### 2.6 Düzenleme

- Tutamaklar: duvar yolunun köşeleri ve kenar ortaları; açıklığın tek orta tutamağı (duvar boyunca kayar); mahalin etiket
  noktası (bütün mahali taşır); aks/merdiven/rampa yol noktaları; kolonun merkezi [K] `G/ops/grips.rs:106-118`,
  `G/ops/grips.rs:406-447`. **En/kalınlık, el/yön çevirme, rıht sayısı, kolon en-boy-dönüş tutamağı yok** [K].
- Geçici ölçüler: açıklık [A] ADR 0701, duvar [A] ADR 0710, kolon [A] ADR 0712, aks ve boy başvuruları [A] ADR 0728.
- Öznitelikler paneli alanları ürün komutlarıyla yazar (ör. `WallChange`: en, sabit tutulacak çizgi, hiza, öteleme, stil,
  uçlar, dolgu, yükseklik, kat, evre, durum, disiplin) [K] `CMD/cad_walls.rs:471-556`.

### 2.7 KCAD

- Her mimari özellik kendi şema numarasıyla “opt-in” yazılır (ör. 115 korkuluk, 116 boşluk, 117 döner merdiven, 119 niş/
  baca, 123 döşeme, 125 merdiven katı) [A] `docs/specs/kcad-v2.md:492-602`. Belirlenimli CBOR [A]
  `docs/specs/kcad-v2.md:125-160`. Mimari nesnelerin anlamı dosyada kayıpsız [A].

### 2.8 DXF/DWG/IFC

- “KentOS verisi” açıkken her mimari nesne bir adsız blok (`*U…`) ve onu taşıyan INSERT olarak yazılır; INSERT nesnenin
  JSON'unu XDATA'da taşır, KentOS geri okur [K] `F/dxf/writer/entities.rs:1883-1960`, `F/dxf/writer/mod.rs:676-760`.
- Kapalıyken parçalar düz çizgi/yay/tarama olarak yazılır ve rapor her türün kaybını söyler (duvar stili/bileşen/kat/
  yükseklik, açıklık türü/numarası/yüksekliği, mahal no/ad/yöntem, döşeme türü/kalınlığı …) [K]
  `F/dxf/writer/mod.rs:786-838`.
- **Parçalar nesnenin kendi katmanında** (taban kopyalanır): taramalar, kapı yayı, cam çizgisi ayrı katmana gitmez [K]
  `F/walls.rs:265-270`. Dış programda “taramayı dondur”, “açılış yaylarını gizle” yapılamaz [T].
- **Mahal DXF'e yalnız halkalarıyla gider**: KentOS verisi açık da olsa kapalı da olsa mahalin parçaları kapalı çoklu
  çizgilerdir; kendi etiketi (numara, ad, alan) ve boşluğun (şaftın) çaprazı yazılmaz [K] `F/dxf/writer/mod.rs:752-774`.
  ADR 0708 §2 çaprazın “DXF'in mimari bloklarında” da olduğunu söyler [A]; okunan kod bunu göstermiyor (çelişki,
  çalıştırılarak doğrulanmadı). Mahal adı DXF'e ancak ayrı etiket nesnesi (`Tag`) varsa yazı olarak gider [T].
- DWG, DXF köprüsüyle (OpenCADCodec) [K] `crates/native/dwg/src/lib.rs:1-5`.
- **IFC yalnız yol aksı için var** [K] `APP/road_ifc.rs`, `F/ifc_alignment.rs`; mimari IFC açık [A] `PLAN-ADV:141`,
  `TODOS.md` BIM-07 (açık).

### 2.9 PDF/pafta

- Pafta haritası görünüş tarifiyle çizer: kat, evre, katman durumu, gizlenen kategoriler (yalnız ölçü/etiket/yazı/tarama/
  blok/mahal/aks/işaret), ölçek, duvar ayrıntısı, kırpma, kalemler, siyah beyaz, şablon, tasarım seçenekleri [K]
  `C/cad_views.rs:76-93`, `C/cad_views.rs:158-215`. Duvar/kapı/kolon/döşeme/merdiven bir görünüşte gizlenemez (kategori
  yok); disipline göre süzme yok [K] (`rg discipline` çizim katmanlarında sonuçsuz).
- PDF metni aranabilir, yazı tipleri gömülü [A] ADR 0721 (Doğrulama), ADR 0792.

### 2.10 Python ve MCP

- Python SDK katalogdan üretilir (`kentos.cad.walls` … ) [K] `python/kentos/cad/walls.py:1-4`; MCP araçları katalogdaki her
  komuttan [K] `crates/native/mcp/src/tools.rs:59-60`, `crates/native/mcp/src/tools.rs:582`.
- Üretilmiş katalogda 62 mimari komut [K] (`apps/web/src/contracts/generated/commandCatalog.json`): `cad.walls.{create,
  rectangle, partition, convert, edit, joins, distances}`, `cad.openings.{create, edit, query, distances}`,
  `cad.openingtypes.edit`, `cad.spaces.{create, discover, edit, finish, number, quantities, separators}`,
  `cad.slabs.{create, edit}`, `cad.grids.create`, `cad.columns.{create, distances}`, `cad.stairs.create`,
  `cad.ramps.create`, `cad.arch.{edit, audit, exceptions}`, `cad.tags.*`, `cad.viewmarks.*`, `cad.schedules.*`,
  `cad.casework.*`, `cad.elevations.*`, `cad.materials.edit`, `cad.legends.materials`, `cad.north.create`,
  `cad.revisions.mark`, `cad.modules.*`, `cad.options.*`, `cad.program.*`, `cad.levels.assign`, `cad.joins.*`,
  `cad.dimensionsets.*`, `cad.views.edit`, `cad.sheets.levels`, `cad.navigator.read`.
- Merdiven/rampa/aks/kolonun düzenlenmesi tek `cad.arch.edit` üzerinden [K] `APP/arch_edit.rs:1-5`.
- Sonuç: **nesne var olduğu sürece GUI–Python–MCP eşitliği yapısal olarak sağlanıyor**; eksikler nesne modelinin
  eksikleridir, API'nin değil [T].

## 3. Öğe öğe denetim

### 3.1 Özet matris

Simgeler: ✓ var · ~ kısmi · ✗ yok. “Ölçek” = 1:20/1:50/1:100/1:200'de farklı çizim. “DXF anlam” = KentOS verisi kapalıyken
başka programda anlam taşıyor mu.

| # | Öğe | Var | Ölçek | Etiket/ölçü | Çizelge/metraj | DXF anlam | Py/MCP |
|---|---|---|---|---|---|---|---|
| 1 | Duvar: düz / yaylı / çok katmanlı | ✓ / ✓ / ✓ | ✓ (3 düzey) | ✓ | ✓ | ✗ (çizgi+tarama) | ✓ |
| 1b | Giydirme cephe | ✗ | — | — | — | — | — |
| 2 | Kapı: tek/çift, sürme, cep, katlanır, döner | ✓ | ✗ | ✓ | ✓ | ✗ | ✓ |
| 2b | Kapı: çift yönlü, pivot, garaj/seksiyonel, kepenk | ✗ | — | — | — | — | — |
| 3 | Pencere: kayıt, denizlik, sürme, sabit | ✓ (yatay kayıt planda ✗) | ✗ | ✓ | ✓ | ✗ | ✓ |
| 3b | Pencere: açılım türü, köşe/cumba, çatı penceresi | ✗ | — | — | — | — | — |
| 4 | Kanatsız boşluk, tesisat boşluğu | ✓ | ✗ | ✓ | ✓ | ✗ | ✓ |
| 5 | Kolon (dikdörtgen/daire/L) | ✓ | ✗ | ~ (derinlik yok) | ✓ | ✗ | ✓ |
| 6 | Kiriş, lento, hatıl | ✗ | — | — | — | — | — |
| 7 | Aks (düz/yay, ağ, kesik) | ✓ | ~ (balon kâğıtta) | ✓ | — | ✗ | ✓ |
| 8 | Döşeme (tek katman) | ✓ | ✗ | ✗ (etiket yok) | ✓ | ✗ | ✓ |
| 9 | Balkon / teras | ✓ (döşeme rolü) | ✗ | ✗ | ✓ | ✗ | ✓ |
| 10 | Şaft (mahal boşluğu) | ✓ | ✗ | ✓ | ✓ | ✗ | ✓ |
| 11 | Niş | ✓ | ✗ | ✓ | ✓ | ✗ | ✓ |
| 12 | Baca (duvar içi) | ✓ | ✗ | ✓ | ✓ | ✗ | ✓ |
| 13 | Merdiven: düz/L/U/sahanlık/döner+direk | ✓ | ✗ | ~ | ✓ | ✗ | ✓ |
| 13b | Merdiven: balansmanlı, yay+düz karışık | ✗ | — | — | — | — | — |
| 14 | Rampa | ✓ (döner ✗) | ✗ | ✓ (eğim, kot) | ✓ | ✗ | ✓ |
| 15 | Korkuluk | ~ (merdiven/rampa yanı, döşeme kenarı) | ✗ | ✗ | ~ (uzunluk) | ✗ | ✓ |
| 16 | Çatı planı / çatı nesnesi | ✗ | — | — | — | — | — |
| 17 | Asma tavan | ~ (tavan ızgarası taraması) | ✗ | ~ | ~ | ✗ | ✓ |
| 18 | Çatı penceresi | ✗ | — | — | — | — | — |
| 19 | Asansör | ~ (yalnız boşluk) | — | — | — | — | — |
| 20 | Mobilya, vitrifiye | ✓ (38 blok) | ✗ | ~ | ✓ | ✓ (blok) | ✓ |
| 21 | Mutfak dolabı | ✓ (dizi) | ✗ | ~ | ✓ | ✓ (blok) | ✓ |
| 22 | Otopark yeri | ✗ | — | — | — | — | — |
| 23 | Peyzaj | ✗ | — | — | — | — | — |
| 24 | Kot işareti | ✓ (kata bağlı) | ✓ (kâğıtta) | — | — | ~ | ✓ |
| 25 | Kuzey oku | ✓ | ✓ | — | — | ✓ (blok) | ✓ |
| 26 | Kesit/görünüş/detay işareti | ✓ | ✓ | — | — | ✗ | ✓ |
| 26b | Türetilmiş bina kesiti / cephe görünüşü | ✗ (oda iç görünüşü ~) | — | — | — | — | ~ |
| 27 | Mahal etiketi | ✓ | ✓ (kâğıtta) | ✓ | ✓ | ✗ (DXF'e hiç yazılmaz) | ✓ |
| 28 | Alan etiketi | ✓ (şablon) | ✓ | ✓ | ✓ | ✗ | ✓ |
| 29 | Kapı/pencere etiketi ve listesi | ✓ (tablo) / ✗ (grafik liste) | ✓ | ✓ | ✓ | ~ | ✓ |
| 30 | Malzeme taramaları | ~ (duvar bileşeni) | ~ (kâğıt boyu) | ✓ (lejant) | ✓ | ~ | ✓ |
| 31 | Yalıtım gösterimi | ~ (çapraz tarama) | ~ | — | ✓ | ~ | ✓ |
| 32 | Taşıyıcı / taşıyıcı olmayan | ~ (malzeme kategorisi) | — | — | ~ | ✗ | ✓ |
| 33 | Tadilat (mevcut/yıkılacak/yeni) | ~ | — | ✓ (evre) | ✓ (süzgeç) | ✗ | ✓ |
| 34 | Vaziyet planı (parsel, çekme, emsal/TAKS) | ✗ (mimaride) | — | — | — | — | — |

Matrisin “DXF anlam” sütunu §2.8'deki koda dayanır; kot işareti (ölçü olarak yazılır, kat bağı kaybolur) ve blok satırları ayrıca çalıştırılarak denetlenmedi [T]. “Py/MCP ✓” komutun katalogda olması demektir (§2.10).

### 3.2 Duvar

- **Parametreler [K]** `C/arch.rs:611-663`: yol (`pts`, yay için `bulges`, `closed`), `width`, `justify` (sol/orta/sağ),
  `offset`, `style`, `components` (≤16, `C/arch.rs:19`), `caps` (açık/kapalı), şematik `fill`, `height` (yoksa katın),
  `arch`, `joins` (uç birleşimi + düğüm kararı `start_pass/end_pass`), `cleanup`. Bileşen: ad, metin malzeme ya da
  `material_id`, kalınlık (biri kalanı alır), kenar kalemi, tarama, en az ayrıntı, öncelik 1–9, `wrap` (uçta/sövede
  döner) [K] `C/arch.rs:173-210`. Sabit tutulacak çizgi (yol/eksen/bitmiş yüz/çekirdek yüzü) [K] `C/arch.rs:304-345`.
- **Hazır stiller [K]** `C/arch.rs:1605-1678`: Dış duvar 25 (sıva/tuğla/ısı yalıtımı/sıva), İç duvar 15, Gazbeton 20,
  Betonarme perde 25, Bölme duvar 10 (alçı levha/yalıtım/alçı levha). Şematik dolgu hepsinde `#5A5A5A` (perdede `#3C3C3C`).
- **Plan temsili [K]** `G/geom/wall.rs:691-767`: iki yüz, uç kapakları, şematikte stil dolgusu; Uygulama/Ayrıntılı'da
  bileşen taramaları ve aralarındaki çizgiler. Birleşimler: köşe (gönye), T, kesişme, alın/geçen, bileşen öncelikleri
  [K] `G/geom/wall_join.rs:1-23`. Yaylı duvar ofsetle (merkezli yay) [K] `G/geom/wall.rs:1-7`.
- **Ölçek:** ✓ üç düzey (2.3). Kesilen/görünen ayrımı yok; alçak duvar da kesit gibi (2.4) [K].
- **Düzenleme:** yol tutamakları; Böl, Birleştir, Uzat, Buda, Kaydır, Boy, Çevir, Stili değiştir, Düğümler [K]
  `CMD/cad_walls.rs:575-701` (işlemler); geçici ölçü, bağlar [A] ADR 0710, 0770.
- **Açıklama:** duvar ölçü dizisi (açıklıklar, toplam, duvar kalınlıkları, akslar; cephe ve mahal tarifleri) [K]
  `C/dimension_set.rs:27-41`, [A] ADR 0695, 0700, 0729, 0730; iç ölçü zinciri [A] ADR 0711; etiket değerleri tür, kalınlık,
  boy, yükseklik, alan, hacim, kat, evre [K] `DOM/arch_values.rs:355-370`.
- **Çizelge/metraj:** duvar çizelgesi [K] `C/schedule.rs:26-36`; brüt/net yüz, açıklık düşümü, cep düşümü [K]
  `APP/spaces_quantities.rs:1-20`; malzeme metrajı bileşen bileşen [K] `APP/material_quantities.rs:1-8`.
- **Çıktı:** KCAD ✓; DXF kapalıyken çizgi+tarama, stil/bileşen/kat/yükseklik kaybı raporlanır [K]
  `F/dxf/writer/mod.rs:827-838`.
- **Sınırlar:**
  - Yükseklik modeli: taban ötelemesi, üst kata/döşeme altına bağlı tepe, eğik/kalkan tepe yok [K] `C/arch.rs:646-650`.
  - Değişken kalınlık, duvar profili (söve, denizlik, dış kaplama, saçak) yok [K].
  - **Giydirme cephe yok** [K] (`rg 'curtain|giydirme'` sonuçsuz).
  - Kolonla birleşim yok [A] ADR 0771:56.
  - Taşıyıcı bayrağı yok; taşıyıcılık yalnız bileşen malzemesinin kategorisinden çıkarılabilir [K] `C/materials.rs:29-41`.

### 3.3 Kapı

- **Parametreler [K]** `C/opening.rs:281-378`: konak duvar, konum (`at`, `anchor`), kaba boşluk `width`, kanat sayısı
  (1–2; döner 3–4; katlanır 2–6), işleyiş (`Swing, Slide, Fixed, Revolving, Pocket, Folding`; `C/opening.rs:94-109`),
  menteşe ucu (`hand`), açıldığı yan (`side`), açılış açısı, yay göster, kasa eni, denizlik/lento kotu, tip ve tip sürümü,
  örnek istisnaları, yapı (`build`), pervaz noktaları (`place`), kopukluk.
- **Yapı (`OpeningBuild`) [K]** `C/opening_types.rs:85-178`: anma ölçüsü referansı (kaba/kasa dış/kanat) `:45-52`, montaj
  boşluğu, kanat kalınlığı, eşit olmayan kanat payı, kasa derinliği ve yüzden uzaklığı, eşik, pervaz/söve, denizlik,
  dikey/yatay kayıt, kullanım alanı, tambur, bindirme, ray, iki yana toplanma, cep payı, ray sayısı.
- **Hazır tipler [K]** `C/opening.rs:817-903`: Kapı 90/80, İki kanatlı kapı 140, Sürme kapı 160, Pencere 120, Sabit
  pencere 100, Boşluk 100, Tesisat boşluğu 20, Niş 60, Kalorifer nişi 100, Baca 20. Proje tipleri ve örnek kontrolü [A]
  ADR 0772.
- **Plan temsili [K]** `G/geom/opening.rs:920-1070`: kasa halkaları, kanat (çizgi ya da kalınlıklı bant), açılış yayı,
  sürme kanatları (1–4, XOX, eşit olmayan), döner tambur, cep, katlanır paneller (30° açık çizilir, `C/opening.rs:25`),
  eşik, iki yüzde pervaz; kullanım alanı kesikli `G/geom/opening.rs:735-741`.
- **Ölçek:** ✗ her ölçekte aynı parçalar; 1:200'de pervaz/eşik/kasa halkaları da çizilir [K] `G/store/mod.rs:763`.
- **Kalem:** kasa, kanat, yay, eşik tek kalem (açıklığın katmanı) [K] `G/store/mod.rs:766-772`.
- **Düzenleme:** orta tutamakla kaydırma; Taşı, Çevir, Dağıt/Çoğalt, Numarala araçları [K] `CAT/ribbon.json` Mimari
  sekmesi (`tool.openingMove`, `tool.openingFlip`, `tool.openingArray`, `tool.openingNumber`, satır 1930-2230); geçici ölçü
  [A] ADR 0701. El/yön çevirme tutamağı ve en tutamağı yok [K] `G/ops/grips.rs:110-111`.
- **Açıklama:** etiket değerleri numara, tip, en (kaba/kasa/kanat/net), yükseklik, denizlik, lento, alan, duvar, işleyiş,
  kanat, tambur, cep, ray, panel [K] `DOM/arch_values.rs:211-262`; yerleştirmede numara [A] ADR 0714. **Ölçü dizisinde
  “90/210” gibi en/yükseklik yazısı yok** [K] (`C/dimension_set.rs`, ADR 0694/0695/0700/0730'da yükseklik/lento geçmiyor);
  etiket şablonuyla `{width}/{height}` yazılabilir [T].
- **Çizelge:** kapı-pencere çizelgesi; çizelge alanları bütün türler için 48 (açıklıkta numara, tip, dört en, yükseklik,
  denizlik, lento, işleyiş, kanat, tambur, cep, ray, panel …), gruplama ve ara toplam [K] `C/schedule.rs:100-167`, [A] ADR 0733.
- **Sınırlar:**
  - Çift yönlü (salon/çarpma), pivot, garaj (seksiyonel/katlanır), kepenk, otomatik kapı ayrımı, yangın kapısı (YK)
    niteliği yok [K] `C/opening.rs:94-109`.
  - Açıklık bir kenarda durur; köşeden dönemez [K] `G/geom/opening.rs:3-4`. Yaylı duvarda simge kiriş boyunca düz
    çizilir [K] `G/geom/opening.rs:6-7`.
  - Kendi evresi yok (duvarınkini alır) [K] `C/entity.rs:2003`.
  - Görünüşte kanat/açılış çizimi yok [A] ADR 0799:114.

### 3.4 Pencere

- **Parametreler:** kapıyla aynı alanlar; denizlik (`sill_board`), dikey kayıt (`mullions`), yatay kayıt (`transoms`,
  “plan does not show them”) [K] `C/opening_types.rs:126-141`.
- **Plan temsili [K]** `G/geom/opening.rs:949-990`: kasa bandının iki yüz çizgisi, ortada tek cam çizgisi (sürmede iki
  bindirmeli kanat, isteğe bağlı ray), dikey kayıtlar, dış yüzde denizlik halkası. **İşleyiş “Swing” olsa da pencerede
  açılış çizilmez** (pencere dalında `swing` okunmuyor) [K].
- **Ölçek:** ✗.
- **Sınırlar:**
  - Açılım türü (vasistas, çift açılım, pivot, sabit+açılır karışık bölmeler) yok; görünüşte açılım üçgeni de yok [K].
  - Köşe pencere, cumba, kemerli/yuvarlak pencere yok [K] (dikdörtgen denizlik–lento; tek kenar).
  - Kepenk/panjur kutusu, lento yok [K].
  - Yüksekteki pencere (denizlik > 1,10) kesmeyen biçimde çizilmez: kapı/pencere her zaman “kesilmiş” [K]
    `C/opening.rs:83-86`.
  - Çatı penceresi yok [K] (`rg 'skylight|çatı penceresi'` sonuçsuz).

### 3.5 Kanatsız boşluk ve tesisat boşluğu

- `OpeningForm::Opening` [K] `C/opening.rs:39-40`. Kesme yüksekliğini kesiyorsa yalnız duvardaki boşluk, kesmiyorsa
  kesikli çerçeve ve çapraz [K] `G/geom/opening.rs:751-758`. Tesisat boşluğu bir tip [K] `C/opening.rs:896-897`.
- Sınır: üstü kapalı geçişin lento izi (kesikli) çizilmez [K] (`Form::Opening if cut => Vec::new()`).

### 3.6 Kolon

- **Parametreler [K]** `C/arch_elements.rs:312-411`: referans noktası, biçim (dikdörtgen/daire/L, `:163-170`), en, derinlik,
  L kol kalınlıkları, dönüş, dokuz noktalı hiza (`:199-210`), tür adı (serbest metin), etiket, `fill` (dolu/boş), iki aks,
  konum yazısı (“B-3”), kopukluk, `arch`.
- **Plan temsili [K]** `G/geom/arch.rs:719-755`: dış hat, isteğe bağlı **düz dolgu**, etiket ve konum yazısı.
- **Ölçek:** ✗ (dolgu açık/kapalı sabit; 1:50'de tarama, 1:100'de siyah dolgu gibi bir kural yok) [K].
- **İlişki:** akslar kesişince izler [K] `DOM/arch_follow.rs:1-9`; **duvarla birleşmez** [A] ADR 0771:56.
- **Açıklama:** etiket değerleri etiket, konum, tür, en, kat, evre; **derinlik yok** (“25/50” yazılamaz) [K]
  `DOM/arch_values.rs:343-353`, `C/arch_tags.rs:15-55`. Kolon çizelgesi ✓ [K] `C/schedule.rs:30`.
- **Sınırlar:** malzeme bağı yok (betonarme taraması malzemeden gelmez) [K] (`ColumnFields`'ta `material_id` yok); T/U/
  çokgen/çelik profil yok; yükseklik ve üst bağ yok.

### 3.7 Kiriş, lento, hatıl

- **Yok.** `rg 'beam|kiriş'` mimari kodda yalnız geometrik “kiriş” (chord) sözcüğünü bulur [K] (`C/codes.rs:1261-1263`
  vb.). Yol haritasında BIM-03 açık [A] `TODOS.md` (`BIM-03`: “Duvar, döşeme, kolon, kiriş … çatı ve merdiven”).
- Etki: planda kesikli kiriş izi, kesitte kiriş, açıklıkta lento gösterilemez [T].

### 3.8 Aks

- **Parametreler [K]** `C/arch_elements.rs:111-140`: iki nokta (yaylı olabilir), etiket, balon uçları (`:66-72`), balon
  boyu (kâğıt mm), kesik aks uç parçaları. Aks ağı (dik/döndürülmüş, radyal), eşit bölme, kesişime bağlı kolonlar [A]
  ADR 0707, 0725. Otomatik etiketleme (I ve O atlanır) [K] `C/arch_elements.rs:881-905`.
- **Plan temsili [K]** `G/geom/arch.rs:1-8`: kendi çizgi tipi (kesik-noktalı), uç balonları ve etiketi.
- **Sınırlar:** çok parçalı (kırık) aks, balonun kılavuzla yana alınması, aksın kata/görünüşe göre boyu yok [K]
  (yalnız iki nokta).

### 3.9 Döşeme, balkon, teras

- **Parametreler [K]** `C/slab.rs:295-333`: halkalar (delikli), rol (döşeme/balkon/teras, `:35-43`), kalınlık
  (varsayılan 0,15), üst kotun kattan farkı, `arch`, malzeme metni ya da `material_id`, kenar (korkuluk/parapet ve eni,
  `:75-81`), metrajdan çıkarma, kaynağa bağlı sınır (duvarlar ya da mahal, yüz kuralı, öteleme) [A] ADR 0722, 0771.
- **Plan temsili [K]** `G/geom/slab.rs:325-336`: halkalar ve serbest kenar boyunca korkuluk (iki çizgi) ya da parapet
  (bant). Rol çizimi değiştirmez (balkon = döşeme) [K]. Kat döşemesinin halkaları da her zaman çizilir; duvar yüzleriyle
  üst üste binebilir [T].
- **Metraj [K]** `APP/slab_quantities.rs:1-10`: alan (boşluklar düşülür), çevre, hacim, üst kot, kenar uzunluğu.
- **Sınırlar:**
  - Katman yapısı (kaplama, şap, yalıtım, betonarme) yok: tek kalınlık [K]. Kesit üretiminin ön koşulu [A] ADR 0347:51-54.
  - Eğim/su yönü (teras, balkon) yok [K].
  - Döşeme etiketlenemez (etiket değerinde `Slab` dalı yok) [K] `DOM/arch_values.rs:209-438`; döşeme kotu planda yazılmaz.
  - Üst kattaki çıkma/balkonun alttaki planda kesikli izdüşümü yok [K].

### 3.10 Şaft (mahal boşluğu)

- `SpaceFields.void`: döşemesiz boşluk, çapraz çizilir, alanı mahal toplamına katılmaz [K] `C/space.rs:232-236`,
  `G/geom/space.rs:382-452`; metrajda ayrı [A] ADR 0708 §3.
- **Sınırlar [A]** ADR 0708:26: katlar arası şaft yok (her katta ayrı), döşemeyle bağ yok, asansör kabini/kapısı yok.
- DXF'te çapraz yazılmaz, yalnız halkalar [K] `F/dxf/writer/mod.rs:752-774` (bkz. 2.8).

### 3.11 Niş ve 3.12 Baca

- `OpeningForm::Niche` (bir yüzden `depth`), `Flue` (duvar içinde boşluk, `inset`) [K] `C/opening.rs:41-44`,
  `C/opening.rs:336-346`. Kesmiyorsa kesikli; bacanın çaprazı [K] `G/geom/opening.rs:758-785`. Duvar incelince `tooDeep`
  [K] `DOM/opening_follow.rs:10-12`. Metrajda duvardan düşülmez [A] PLAN:1019.
- **Sınırlar:** bağımsız (duvar dışı) baca/şönt baca bloğu, çatıdan çıkış, baca kesiti yok [K].

### 3.13 Merdiven

- **Parametreler [K]** `C/arch_elements.rs:434-483`: yürüme çizgisi 2–4 nokta (düz/L/U; `MAX_FLIGHT_POINTS = 4`,
  `:13`), döner merdivende iki noktalı yay (`bulges`), en, rıht sayısı/yüksekliği, basamak, sahanlık boyu, kesme rıhtı,
  numaralama, korkuluk yanları, kat yüksekliğinden tasarım, merkez direği (helezon).
- **Plan temsili [K]** `G/geom/arch.rs:1373-1440`: bant, sahanlık kenarları, rıhtlar (kesmenin üstü kesikli), kesme
  çizgisi (tek düz çapraz; kırık çizgi değil, `:1399-1408`), korkuluklar (bant kenarından 5 cm içeride tek çizgi,
  `:289`), çıkış oku, basamak numaraları, direk. Üst katta yukarıdan görünen kısım [K] `G/geom/arch.rs:1223-1307`.
- **Denetim:** İmar Yönetmeliği md. 31 profilleri (rıht, basamak, 2a+b, en, sahanlık, dar uç, korkuluk) [K]
  `CAT/arch-profiles/imar-merdiven-konut-1.json`, [A] ADR 0724.
- **Metraj:** uzunluk, sahanlık sayısı, plan alanı, korkuluk uzunluğu, iç basamak [K] `G/geom/arch.rs:1310-1369`.
- **Açıklama:** etiket değerleri yalnız rıht sayısı, en, kat, evre [K] `DOM/arch_values.rs:378-384`; “18×17,5/27” gibi rıht/
  basamak yazısı otomatik değil.
- **Sınırlar:**
  - **Balansmanlı (dengelenmiş) basamak yok**; L/U dönüşü yalnız sahanlıkla [K] (dönüşte sahanlık `landing`).
  - Yay+düz karışık kol, 4'ten çok noktalı kol, çok katlı merdiven, rıht sayısının yeniden hesaplanması açık [A]
    PLAN:1081, PLAN:1096-1097.
  - Limon, basamak burnu, kalınlık, merdiven boşluğu/döşeme kesimi, kesitte merdiven yok [K].
  - Ölçek: ✗.

### 3.14 Rampa

- **Parametreler [K]** `C/arch_elements.rs:506-543`: merkez çizgisi 2–4 nokta, en, kot farkı, eğim, başlangıç kotu ya da
  kata bağlı fark, sahanlık, korkuluk. En uzun kol ve ara sahanlık, erişilebilirlik profili [A] ADR 0723.
- **Plan temsili [K]** `G/geom/arch.rs:1604-1661`: bant, sahanlıklar, korkuluklar, ok, eğim yazısı (`%5.00`), iki uçta kot.
- **Sınırlar:** döner rampa, otopark rampası geçiş eğimleri, enine eğim yok [A] PLAN:1127, [K].

### 3.15 Korkuluk

- Merdiven/rampa yan çizgisi [K] `C/arch_elements.rs:24-28`, `G/geom/arch.rs:957-970`; döşemenin serbest kenarında
  korkuluk (iki çizgi) ya da parapet (bant) [K] `C/slab.rs:75-81`, `G/geom/slab.rs:295-322`.
- **Sınırlar:** bağımsız korkuluk nesnesi (keyfi yol, galeri kenarı dışında), yükseklik, dikme/küpeşte/cam, uç uzatması,
  görünüşte korkuluk yok [K].

### 3.16 Çatı planı, 3.18 çatı penceresi

- **Yok.** Çatı nesnesi, eğim, mahya/dere/kalkan, saçak, oluk/iniş borusu, baca çıkışı, çatı penceresi yok [K]
  (`rg -i 'roof|çatı'` mimari sözleşme ve uygulamada yalnız “çatışma” sözcüğünü bulur). ADR 0347 §3 kesit için “çatı ve
  eğimli tavan (yok)” der [A] ADR 0347:51-52.

### 3.17 Asma tavan

- Mahalin tavan ızgarası, mahale bağlı ilişkili tarama olarak (`Kaplama: Tavan`) [K] `APP/spaces_finish.rs:1-6`;
  mahal yüksekliği (döşemeden tavana) [K] `C/space.rs:224-228`; etikette `{ceilingElevation}` [K] `C/arch_tags.rs:36`.
- **Sınırlar:** tavan nesnesi (kot farkları, kutu/bulkhead, boşluklar, aydınlatma/menfez), tavan planı (RCP) görünüşü yok [K].

### 3.19 Asansör

- Yalnız mahal boşluğu (“lift shaft”) [K] `C/space.rs:232`. Kabin, kat kapısı, makine dairesi, karşı ağırlık sembolü yok;
  blok kitaplığında asansör yok [K] `CAT/blocks/kentos-temel.kblok` (38 bloğun listesi).

### 3.20 Mobilya, vitrifiye; 3.21 Mutfak dolabı

- 38 gerçek boyutlu blok: kapı/pencere blokları, masa/sandalye/kanepe/yatak/dolap/komodin/koltuk/çalışma masası/TV/kitaplık,
  lavabo/klozet/asma klozet/pisuvar/duş/köşe duş/küvet/dolaplı lavabo, beyaz eşya, evye/ocak/fırın/alt-üst dolap; işaretler
  (kot, kuzey oku, kesit), antet, revizyon [K] `CAT/blocks/kentos-temel.kblok`. Dinamik parametreler, kullanım boşluğu zarfı,
  Duvara hizala [A] ADR 0702:35-46.
- Mutfak dizisi: düz/L/U, evye, ocak, bulaşık, fırın, buzdolabı, tezgâh kesimi, üst dolaplar; duvarları izler [K]
  `APP/casework_run.rs:1-6`, `C/casework.rs:120-194`.
- Donatı çizelgesi (blok kartı kategorisi, kat, evre, mahal) [K] `DOM/arch_values.rs:412-425`, [A] ADR 0733.
- **Sınırlar:** yalnız plan görünüşü (blokların görünüş/kesit temsili yok); ofis, okul, sağlık, engelli WC seti, otopark,
  ağaç yok [K]; ölçeğe göre sadeleşme yok [K].

### 3.22 Otopark yeri, 3.23 Peyzaj

- **Yok** [K] (`rg 'parking|otopark'` yalnız `APP/sources.rs`'te ilgisiz; blok kitaplığında ağaç/araç yok). Program yalnız
  “Dış alan: bahçe, otopark” grubu tanır [K] `C/program.rs:33-41`.

### 3.24 Kot işareti

- Kot ölçüsü `DimensionStyle::Elevation` [K] `C/entity.rs:1404`; kata bağlı kot (`level { id, offset }`), kat kotu değişince
  izler [A] ADR 0721 §2. Kitaplıkta “Kot işareti” bloğu [K] `CAT/blocks/kentos-temel.kblok:1192`.
- **Sınırlar:**
  - Mahalin kendi döşeme kotu yok; `{floorElevation}` yalnız katın kotudur [K] `DOM/arch_values.rs:121-130`,
    `C/space.rs:188-245` (alan yok). Islak hacim −2 cm, balkon −5/−15 cm gibi farklar yazılamaz [T].
  - ±0.00'ın mutlak kotu (kot alma noktası) yok [K] (`rg` sonuçsuz).

### 3.25 Kuzey oku

- `cad.north.create`: plan kuzeyi, coğrafi kuzey (meridyen yakınsaması) ya da açı; kâğıtta boyut [A] ADR 0721 §3.

### 3.26 Kesit/görünüş/detay işaretleri ve türetilmiş çizimler

- İşaret biçimleri kesit/görünüş/detay [K] `C/viewmark.rs:27-35`; hedef adlı görünüm ya da paftanın harita çerçevesi;
  balonda pafta numarası izlenir [K] `C/viewmark.rs:154-196`, `DOM/viewmark_follow.rs:1-15`. İşaret çizim üretmez [A]
  `PLAN-ADV:50` (A9).
- Türetilmiş: **yalnız oda iç görünüşü**; duvarlar, mahal yüksekliği, açıklıkların denizlik–lento dikdörtgeni [A]
  ADR 0799 §2. Çizmedikleri: kanat/açılış, döşeme ve tavan kesiti, kaplama taraması, yaylı yan, kolon ve donatı görünüşü
  [A] ADR 0799:112-116.
- **Bina kesiti ve cephe görünüşü yok.** Tasarım ADR 0347 §3'te: duvar tabanı/tepesi kurala bağlı, döşeme/tavan/çatı katmanları,
  her bileşenin kesit taraması ve görünüş çizgisi, `geom::section` — hepsi “yok” diye listelenmiş [A] ADR 0347:46-61;
  `G/geom/` altında `section.rs` yok [K]. Elle 2B çizim temel hedef [A] `plan/cad.md:20`. Şablonun “A-301 Kesitler” paftası
  boş çerçevedir [K] `CAT/templates/mimari.json:18`.

### 3.27 Mahal etiketi ve 3.28 Alan etiketi

- Mahal: yöntem (net/eksen/brüt), numara, ad, kullanım, yükseklik, kaplamalar (zemin/duvar/tavan/süpürgelik **metin**),
  boşluk, bağımsız bölüm, kolon düşümü, kaynak kapsamı [K] `C/space.rs:160-245`.
- Kendi etiketi: numara, ad, “x.xx m²” satırları [K] `G/geom/space.rs:356-362`. Etiket nesnesiyle şablon: alan, net alan,
  çevre, kat, döşeme/tavan kotu, bölüm, hedef ve fark, pay, brüt … (38 yer) [K] `C/arch_tags.rs:15-55`; çerçeve biçimleri
  daire/altıgen/dikdörtgen/yuvarlatılmış/yok [K] `C/arch_tags.rs:72-79`.
- **Sınırlar:** kaplamalar malzeme kitaplığına bağlı değil (metin) [K] `C/space.rs:160-173`; hazır Türk etiket stili
  (ad/alan/kaplama kodu/kot) şablon olarak gelmiyor [T]; mahalin kendi etiketi DXF'e yazılmaz [K]
  `F/dxf/writer/mod.rs:752-774`.

### 3.29 Kapı/pencere etiketleri ve listeleri

- Etiket ✓ (3.3), toplu etiketle ve etiket denetimi [K] `CAT/ribbon.json` (`tool.tagAll`, `arch.tagsCheck`).
- Liste: tablo biçiminde kapı ve pencere çizelgesi [K] `C/schedule.rs:26-36`.
- **Grafik doğrama listesi yok:** tip başına görünüş çizimi (ölçüleri, açılış yönü, kayıtları, malzeme) üretilmiyor [K]
  (`rg 'type elevation|doğrama görünüş'` sonuçsuz; görünüşte kanat da çizilmiyor, ADR 0799:114).

### 3.30 Malzeme taramaları; 3.31 Yalıtım

- Proje malzemesi: kimlik, ad, kategori (taşıyıcı/yalıtım/kaplama/membran/diğer), metraj biçimi (alan/hacim), **kesit
  taraması ve kesit kalemi** [K] `C/materials.rs:29-41`, `C/materials.rs:101-120`. Kitaplık 21 malzeme (betonarme NET3,
  tuğla ANSI31, gazbeton DOTS, EPS ANSI37, XPS NET, sıvalar DOTS …) [K] `CAT/materials/kentos-temel.json`. Malzeme lejandı
  [A] ADR 0721 §5.
- Yalıtım: duvar bileşeni ANSI37 (çapraz) taraması; kütüphanede ANSI37 “Kurşun, çinko, magnezyum, yalıtım” [K]
  `G/geom/hatch_pattern.rs:172-173`; uçta ve sövede dönen katman [A] ADR 0771 §3.
- **Sınırlar:**
  - Yalıtımın kıvrımlı/zikzak simgesi (AutoCAD BATTING tipi) yok; bileşen kenarı karmaşık çizgi tipi alamaz [K]
    `C/arch.rs:145`.
  - Görünüş (yüzey) deseni yok, yalnız kesit taraması [K] `C/materials.rs:101-120`.
  - Kolon, merdiven, mahal kaplaması malzemeye bağlanamaz [K]; döşemenin malzemesi taranmaz (yalnız metraj) [K]
    `G/geom/slab.rs:325-336`.
  - Isıl iletkenlik, yangın sınıfı gibi özellikler yok [K].

### 3.32 Taşıyıcı / taşıyıcı olmayan

- Yalnız dolaylı: malzeme kategorisi `Structure`, bileşen önceliği, “Betonarme perde 25” stili [K] `C/materials.rs:29-31`,
  `C/arch.rs:1654-1664`. Disiplin alanı var ama görünüşte süzgeç olarak kullanılmıyor [K] `C/cad_views.rs:76-93`.
- “Taşıyıcı sistem planı” (yalnız kolon/perde/kiriş) ya da taşıyıcı duvarın farklı gösterimi kurulamaz [T].

### 3.33 Tadilat

- Evre görünümleri: Tümü, Mevcut durum, Yıkım planı, Yeni durum; görünüş biçimi göster/gizle/kesikli/%60 soluk [K]
  `C/arch.rs:417-490`, `STY/phases.rs:95-114`. Görünüş tarifinde evre [K] `C/cad_views.rs:168-171`.
- **Sınırlar:**
  - Açıklık ve etiketi evreyi duvardan alır [K] `C/entity.rs:2001-2010`: mevcut duvarda **yeni açılacak kapı**, **kapatılacak
    boşluk** gösterilemez.
  - Duvarın bir parçasını yıkmak için duvarı bölmek gerekir [T] (evre nesne başına).
  - Evreye göre renk/tarama/kalem (yıkılacak sarı kesikli, yeni kırmızı/taralı) yok; yalnız kesikli ve soluk [K].
  - DXF'te evre katmanlara ayrılmaz [K] (2.8).

### 3.34 Vaziyet planı

- Mimari modülde parsel, bina oturumu, çekme mesafesi, kot alma noktası, giriş, otopark, peyzaj, emsal/TAKS/KAKS tablosu
  **yok** [K] (`rg 'emsal|taks|kaks|çekme|vaziyet'` mimari kodda sonuçsuz).
- Harita modülünde imar planı denetimi (TAKS, KAKS, kat adedi/Hmax, plan notları) var [K]
  `python/kentos/cad/zoning.py:22-33`; plan senaryosu için KAKS × alan [K] `APP/scenarios_impact.rs:113-119`. Bunlar
  imar planının alanlarıdır, binanın kendi alan hesabı değildir.
- Bina programı ortak alan payı hesaplar; “resmî alan profili ve arsa payı” açık [A] `PLAN-ADV:213`.

## 4. Türk mimari proje takımına göre eksikler

İçerik sütunu meslek pratiğidir [P]; mevzuat maddeleri bu oturumda doğrulanmadı.

| Pafta / belge | Beklenen içerik [P] | KentOS'ta karşılığı | Eksik |
|---|---|---|---|
| Vaziyet planı (1/200–1/500) | Parsel ve komşular, yol, imar çekme hatları, bina oturumu ve ölçüleri, kot alma noktası, ±0.00 = mutlak kot, giriş, otopark, bahçe/peyzaj, kuzey, emsal/TAKS/KAKS tablosu | Harita modülünde parsel ve imar planı denetimi; mimaride kuzey oku | Bina ↔ parsel ilişkisi, çekme hattı ve denetimi, kot alma noktası, otopark yeri, peyzaj nesneleri, emsal tablosu |
| Kat planları (1/50 uygulama, 1/100 ruhsat) | Duvar/perde/kolon, kapı-pencere ve numaraları, mahal adı-alanı-kaplaması-kotu, merdiven, asansör, şaft/baca, ıslak hacimler, aks ve ölçüler, kesit izleri, kiriş izleri (kesikli), üst kat çıkmaları | Büyük ölçüde var (bkz. §3) | Kiriş izi, kolon–duvar birleşimi, mahal kotu, asansör sembolü, çıkma izdüşümleri, ölçeğe göre sadeleşme |
| Çatı planı (1/50–1/100) | Eğimler, mahya/dere/kalkan, saçak, oluk ve iniş boruları, baca/havalandırma çıkışları, çatı pencereleri, kotlar | — | Bütünü |
| Kesitler (en az 2, 1/50) | Döşeme katmanları, kotlar, merdiven kesiti, kiriş/lento, çatı, temel izi, kat yükseklikleri | Kesit işareti; elle 2B çizim | Türetim ve gereken düşey veri (ADR 0347 §3) |
| Görünüşler (4 cephe, 1/50–1/100) | Doğramalar ve açılım işaretleri, kaplama, söve/denizlik, saçak, kotlar, zemin hattı, bina yüksekliği | Görünüş işareti; oda iç görünüşü (sınırlı) | Cephe türetimi, açılım çizimi, kaplama deseni |
| Sistem kesitleri / sistem detayları (1/20) | Duvar–döşeme–çatı birleşimi, yalıtım sürekliliği, doğrama montajı | Planda duvar bileşenleri (Ayrıntılı) | Düşey kesit, döşeme/çatı katmanları, yalıtım simgesi |
| Nokta detayları (1/5–1/10) | Profil, montaj, su yalıtımı, kenar | Genel 2B CAD araçları + detay işareti | Detay bileşen kitaplığı (profil, yalıtım, tuğla dizisi) |
| Kapı-pencere (doğrama) listesi | Tip görünüşü, ölçüleri, açılım yönü, malzeme, adet, konum | Tablo çizelge (çok alanlı) | Grafik tip görünüşleri |
| Mahal listesi | No, ad, alan, döşeme/duvar/tavan/süpürgelik kaplaması, tavan yüksekliği | Kaplama çizelgesi, mahal metrajı | Kaplamaların malzeme kimliği, duvar kaplama yüksekliği |
| Alan hesapları | Kat brüt/net alanları, bağımsız bölüm alanları, ortak alan payları, emsal dışı alanlar, emsal/TAKS/KAKS, otopark ve sığınak hesapları | Mahal/bölüm çizelgesi, ortak alan payı (ADR 0784, 0795) | Emsal sınıflandırması ve tablosu, arsa payı, otopark/sığınak hesabı |

## 5. Ölçeğe göre ayrıntı matrisi

Beklenen [P] / KentOS [K]. “=” her ölçekte aynı çizim.

| Öğe | 1:200 | 1:100 | 1:50 | 1:20 | KentOS |
|---|---|---|---|---|---|
| Duvar | Dolu dış hat | Dolu ya da tek tarama | Çekirdek taraması + sıva çizgisi | Bütün katmanlar, yalıtım simgesi | ✓ 3 düzey (`C/arch.rs:94-125`); yalıtım simgesi ✗ |
| Kapı | Boşluk + kanat çizgisi | Kanat + yay | Kasa + kanat + yay + eşik | Kasa profili, söve, eşik detayı | = (`G/store/mod.rs:763`) |
| Pencere | İki çizgi | 2–3 çizgi | Kasa + cam + denizlik + kayıt | Profil kesiti | = |
| Kolon | Dolu | Dolu | Tarama ya da dolu | Tarama + kaplama | = (yalnız `fill`) |
| Merdiven | Bant + ok | Rıhtlar + ok | Rıhtlar, korkuluk, kesme, numara | Basamak burnu, limon | = |
| Döşeme/balkon | Hat | Hat + kot | Hat + kot + eğim | Katmanlar (kesitte) | = |
| Mahal etiketi | No + ad | No + ad + alan | + kaplama + kot | — | Yazı boyu kâğıtta ölçeklenir; içerik = |
| Mobilya | Gizli ya da sade | Sade | Sade | Ayrıntılı | = (görünürlük durumu elle) |
| Taramalar | Gizli/dolu | Seyrek | Normal | Gerçek ölçekli | Kâğıt boyu desen ✓; ölçeğe göre seçim ✗ |

## 6. “AutoCAD kadar ayrıntılı” çizim için nesne tarafındaki en önemli 15 eksik (öncelik sırasıyla)

Öncelik: **P0** temel (başka eksiklerin ön koşulu), **P1** proje takımını tamamlar, **P2** derinlik/uyum.

1. **P0 — Düşey model ve türetilmiş bina kesiti/cephe görünüşü.**
   - Kanıt: duvarda yalnız `height` [K] `C/arch.rs:646-650`; döşeme tek kalınlık [K] `C/slab.rs:295-333`; ADR 0347 §3'ün
     “yok” listesi [A] ADR 0347:50-61; oda iç görünüşü kanat çizmez [A] ADR 0799:112-116; `geom::section` yok [K].
   - Kapsam: duvar tabanı/tepesi kata, üst kata ya da döşeme altına bağlı; döşeme/tavan katmanları; kesit düzlemi ve bakış
     derinliği; kesilen kalın/görünen ince; açıklıkta kanat ve açılım işareti; bilinmeyeni çizmeden listeleme (ADV-06
     ilkesi). İlk dilim: dik duvarlı, düz döşemeli iki katlı konutta bir kesit ve bir cephe.
2. **P0 — Alt bileşen kalemleri ve kesilen/görünen/üstteki ayrımı (görüntü bileşenleri).**
   - Kanıt: açıklık parçaları kalemsiz [K] `G/store/mod.rs:766-772`; kalem nesne türünde [K] `C/cad_views.rs:260-279`,
     [A] ADR 0783:85; kesme yüksekliği sabit ve yalnız açıklıkta [K] `C/opening.rs:21`.
   - Kapsam: kapı (kasa, kanat, yay, eşik, pervaz), pencere (kasa, cam, denizlik, kayıt), duvar (kesit hattı, bileşen
     çizgisi, tarama), merdiven (kesilen, kesme üstü, ok), kolon (hat, tarama) için ayrı kalem/renk/çizgi tipi ve görünüş
     şablonunda geçersiz kılma; katın/görünüşün kesme yüksekliği; kesme altındaki alçak duvarın “görünen” çizilmesi;
     “üstteki” (kesikli) kategori: kiriş, çıkma, saçak, üst dolap.
3. **P0 — Bütün mimari nesnelerde ölçeğe göre temsil.**
   - Kanıt: yalnız duvarda `WallDetail` [K] `C/arch.rs:94-125`; açıklıkta ölçek yok [K] `G/geom/opening.rs:731`.
   - Kapsam: aynı üç düzeyi (ya da görünüş tarifinin ayrıntısını) kapı, pencere, kolon, merdiven, döşeme, mahal etiketi ve
     donatıya yay: 1:200'de boşluk + kanat çizgisi, 1:50'de kasa/eşik, 1:20'de profil; kolonda dolgu ↔ malzeme taraması.
4. **P0 — Taşıyıcı sistem: kolon–duvar birleşimi, kolon malzemesi, kiriş ve lento.**
   - Kanıt: [A] ADR 0771:56; kolonda malzeme alanı yok [K] `C/arch_elements.rs:312-411`; kiriş yok [K]; BIM-03 açık [A].
   - Kapsam: duvar çizgilerinin kolon yüzünde durması, perde–kolon taramasının birleşmesi (bileşen önceliği kuralıyla),
     kolonun malzemeye bağlanması, kiriş nesnesi (planda kesikli iz, kesitte kesit, alt kotu), açıklıkta lento/hatıl.
5. **P1 — Çatı nesnesi ve çatı planı.**
   - Kanıt: çatı yok [K]; [A] ADR 0347:51-52.
   - Kapsam: dış hattan kırma/beşik/tek yön çatı, eğim, mahya/dere/kalkan, saçak, oluk/iniş, baca ve çatı penceresi
     delikleri, eğim okları ve kotlar; kesite girdi.
6. **P1 — Döşeme yapısı, mahal kotları ve tavan.**
   - Kanıt: döşeme tek katman [K]; mahalin döşeme kotu yok [K] `DOM/arch_values.rs:121-130`; döşeme etiketlenemez [K];
     mutlak kot yok [K]; tavan yalnız tarama [K] `APP/spaces_finish.rs:1-6`.
   - Kapsam: döşeme katman stilleri (duvar stilinin eşi), mahal başına döşeme farkı (ıslak hacim, balkon), döşeme/mahal
     kotlarının otomatik kot işaretleri, ±0.00 = mutlak kot ayarı, asma tavan nesnesi (kot, kutu, boşluk) ve tavan planı.
7. **P1 — Tadilat temsilinin tamamlanması.**
   - Kanıt: açıklığın evresi duvardan [K] `C/entity.rs:2001-2010`; görünüş biçimi yalnız kesikli/soluk [K]
     `C/arch.rs:427-490`.
   - Kapsam: açıklığa kendi evresi (yeni kapı, kapatılacak boşluk = dolgu), duvarın parçasal yıkımı, evreye göre renk/
     tarama/kalem şablonu (yıkılacak, yeni, mevcut), DXF'te evre katmanları.
8. **P1 — Doğrama kapsamı ve grafik doğrama listesi.**
   - Kanıt: işleyişler 6 [K] `C/opening.rs:94-109`; pencerede açılım çizimi yok [K] `G/geom/opening.rs:949-990`; tek kenar
     [K] `G/geom/opening.rs:3-4`; çizelge yalnız tablo [K] `C/schedule.rs:26-36`.
   - Kapsam: çift yönlü, pivot, seksiyonel garaj, kepenk; pencerede vasistas/çift açılım/pivot ve bölme düzeni (sabit +
     açılır); köşe ve cumba; giydirme cephe (kayıt ızgarası); tipten doğrama görünüşü üreten “grafik liste”; ölçü dizisinde
     en/yükseklik yazısı (“90/210”, “120/150 P:90”).
9. **P1 — Vaziyet planı ve imar/alan hesapları.**
   - Kanıt: mimaride yok [K]; Harita'da imar planı denetimi [K] `python/kentos/cad/zoning.py:22-33`; arsa payı açık [A]
     `PLAN-ADV:213`.
   - Kapsam: parseli (Harita katmanından bağla) bina oturumuyla ilişkilendir, çekme hatları ve denetimi, kot alma noktası,
     emsal sınıfı mahal/döşeme özelliği ve emsal/TAKS/KAKS tablosu (resmî profil olarak, uygunluk iddiası olmadan),
     otopark yerleri ve sayımı.
10. **P1 — Merdiven ve korkuluk derinliği.**
    - Kanıt: 4 nokta sınırı [K] `C/arch_elements.rs:13`; sahanlıktan başka dönüş yok [K]; açık maddeler [A] PLAN:1081,
      PLAN:1096-1097; korkuluk yalnız çizgi [K] `G/geom/arch.rs:957-970`.
    - Kapsam: balansmanlı basamak (L/U dönüşünde), yay+düz kollar, limon/burun, “n × h / b” yazısı, döşeme boşluğu
      ilişkisi, bağımsız korkuluk nesnesi (yol, yükseklik, dikme, küpeşte, cam; galeri kenarı), kesitte merdiven.
11. **P2 — Yapı elemanı ve donatı kitaplığı: asansör, otopark, peyzaj ve görünüşleri.**
    - Kanıt: 38 blok [K] `CAT/blocks/kentos-temel.kblok`; asansör yalnız boşluk [K] `C/space.rs:232`.
    - Kapsam: parametrik asansör (kuyu, kabin, kat kapısı; şafta bağlı), otopark yeri (numara, sayım, rampa), ağaç/bitki/
      sert zemin, engelli WC seti; blokların görünüş temsili (iç görünüş ve kesitte).
12. **P2 — Malzeme, tarama ve yalıtım sunumu.**
    - Kanıt: bileşen kenarı 4 temel çizgi tipi [K] `C/arch.rs:145`; yalnız kesit taraması [K] `C/materials.rs:101-120`;
      kaplamalar metin [K] `C/space.rs:160-173`.
    - Kapsam: yalıtım simgesi (karmaşık çizgi tipi ya da özel desen olarak bileşen gösterimi), yüzey deseni (görünüş için),
      gerçek ölçekli kaplama desenleri, kolon/döşeme/merdiven/mahal kaplamasının malzemeye bağlanması, malzeme özellikleri
      (ısıl iletkenlik, yangın sınıfı) ayrı ve isteğe bağlı.
13. **P2 — Doğrudan düzenleme tutamakları.**
    - Kanıt: duvar yalnız yol tutamakları, açıklık tek orta tutamak, kolon yalnız merkez [K] `G/ops/grips.rs:106-118`.
    - Kapsam: duvar eni/yüz tutamağı, kapı eli ve açılış yanı için çevirme okları, açıklık pervaz (en) tutamakları, kolon
      en/boy/dönüş, merdiven eni ve son rıht, döşeme kenarı ofseti; her biri mevcut komut üzerinden tek geri alma adımı.
14. **P2 — Etiket ve açıklama değerlerinin tamamlanması.**
    - Kanıt: kolonda derinlik yok [K] `DOM/arch_values.rs:343-353`; merdivende yalnız rıht sayısı [K] `:378-384`; döşeme dalı
      yok [K]; etiket yerleri 38 [K] `C/arch_tags.rs:15-55`.
    - Kapsam: kolon “25/50”, merdiven rıht yüksekliği/basamak, döşeme kotu ve kalınlığı, mahal kaplama kodu/kot; Türk ofis
      alışkanlığına göre hazır etiket şablonları (mahal, kapı, pencere, kolon).
15. **P2 — Değişim: DXF katman ayrımı ve mimari IFC.**
    - Kanıt: parçalar nesnenin katmanında [K] `F/walls.rs:265-270`; adsız blok [K] `F/dxf/writer/entities.rs:1883-1960`;
      IFC yalnız yol [K]; IFC açık [A] `PLAN-ADV:141`.
    - Kapsam: dışa aktarmada parçaların alt katmanlara dağıtılması (ör. `A-DUVAR-TARAMA`, `A-KAPI-YAY`, `A-PENCERE-CAM`;
      ISO 13567 benzeri profil seçimi), evre/disiplin katmanları; mahal etiketinin ve şaft çaprazının DXF'e yazılması
      (bugün yalnız halkalar, `F/dxf/writer/mod.rs:752-774`); IFC4'te duvar, açıklık, mahal, döşeme, kolon, merdiven
      dışa aktarımı ve kayıp raporu.

## 7. Plana eklenebilecek maddeler (öneri)

Her biri `PLAN-ADV`'in biçimine (Kullanıcı sonucu, dilimler, İlk kabul) uygun yazılmak üzere kısa çekirdekler. CLAUDE.md'nin
“Araştırma önce” kuralı gereği her maddede üretici belgeleri (ör. AutoCAD Architecture görüntü temsilleri, Revit Detail Level
ve alt kategoriler, Archicad Model View Options / Partial Structure Display) okunup ADR'ye yazılmalıdır; bu raporda kaynak
adresi verilmemiştir.

| Öneri | İçerik | Bağımlılık | İlk kabul önerisi |
|---|---|---|---|
| ADV-08 Düşey model | Duvar taban/tepe kuralları, döşeme/tavan katmanları, kiriş/lento verisi, mahal döşeme farkı | — | İki katlı konutta duvar tepesi üst döşeme altına bağlı; kat yüksekliği değişince izler; KCAD/Python/MCP |
| ADV-09 Türetilmiş kesit ve cephe | `geom::section`; kesilen/görünen; açıklık açılımı; bilinmeyen listesi; eskime ve yeniden üretim (ADR 0799 kalıbı) | ADV-08 | 1:50 bir kesit + bir cephe PDF'te; kapı değişince “eski” |
| ADV-10 Görüntü bileşenleri ve ölçeğe göre temsil | Alt bileşen kalemleri, görünüş şablonunda geçersiz kılma, kesme yüksekliği, tüm nesnelerde LOD | — | Aynı kat 1:100 ve 1:50 paftada farklı kapı/pencere/kolon çizimi; siyah beyaz PDF |
| ADV-11 Taşıyıcı sistem | Kolon–duvar birleşimi, kolon malzemesi, kiriş, lento/hatıl, taşıyıcı görünüşü | ADV-10 | Perde+kolon+tuğla duvar düğümünde tarama sürekliliği; kiriş planda kesikli |
| ADV-12 Çatı | Çatı nesnesi, çatı planı, delikler, oluk/iniş | ADV-08 | Kırma çatı + baca + çatı penceresi; çatı planı paftası |
| ADV-13 Doğrama genişlemesi | Yeni işleyişler, açılım türleri, köşe/cumba, giydirme cephe, grafik doğrama listesi, “90/210” ölçü yazısı | ADV-09 (görünüş) | K01, P01 tiplerinin listede görünüşü; tip değişince liste izler |
| ADV-14 Tadilat temsili | Açıklık evresi, dolgu, parçasal yıkım, evre şablonu, DXF evre katmanı | ADV-10 | Mevcut duvarda yeni kapı + kapatılan pencere; üç evre paftası |
| ADV-15 Vaziyet ve imar hesabı | Parsel bağı, çekme hatları, kot alma noktası, emsal sınıfı ve tablosu, otopark | ADV-08 (kotlar) | Tek parselde bina oturumu, çekme denetimi ve TAKS/KAKS tablosu (resmî onay iddiası yok) |
| ADV-16 Merdiven/korkuluk/donatı | Balansman, karışık kol, bağımsız korkuluk, asansör, otopark yeri, peyzaj | ADV-08 (korkuluk yüksekliği) | Balansmanlı L merdiven profil denetiminden geçer; galeri korkuluğu |
| ADV-17 Değişim | DXF alt katman profili, evre/disiplin katmanı, mimari IFC4 | ADV-10, ADV-14 | Dış programda taramalar ayrı katmanda; IFC'de duvar/kapı/mahal sayısı eşit |

## 8. Bu araştırmanın sınırları

- Hiçbir test, derleme ya da GUI çalıştırılmadı; ADR'lerin “geçti/ölçüldü” kayıtları yeniden doğrulanmadı.
- Masaüstü kabuğundaki (apps/desktop) çizim katmanı ayrıntılı okunmadı; çizim parçalarının ekranda ve PDF'te aynı yoldan
  geçtiği ADR 0783 §1'in kaydıdır [A]. Masaüstünde ek bir mimari temsil kuralı olup olmadığı yalnız `rg` ile tarandı [T].
- Türk mevzuatına ait madde numaraları (emsal, otopark, sığınak, yangın) doğrulanmadı; yalnız merdiven profilinin md. 31
  kaynağı kataloğun kendisindedir [K] `CAT/arch-profiles/imar-merdiven-konut-1.json`.
- Web uygulaması (ADR 0228 ile duruyor) incelenmedi.
