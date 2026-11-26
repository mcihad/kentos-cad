# ADR 9314: Noktanın adı yazıdır ve noktasını izler

- **Durum:** kabul edildi (2026-11-26). Sahibin sözleri (26 Kasım 2026):
  - “CAD içerisinde sahneye nokta koyarken yazılan yazılar da aynı şekilde yazı olacak etiket değil”;
  - “Ayrıca bu yazılar noktaya bağlı olabilir. Taşınınca yazının da taşınması iyi olur.”
  - “Nokta listesinden ve object inspector üzerinden nokta adı değişince metni de değişebilmelidir.”
- **İlgili:**
  - [ADR 0175](0175-labels-to-text.md) §4 (nesneye bağlı yazı);
  - [ADR 0152](0152-survey-points.md) §2 (Nokta aracı);
  - [ADR 0802](0802-strict-module-separation.md) (Mimari'de nokta adsızdır);
  - [ADR 9311](9311-arial-times-point-names-labels-off.md) §6 (köşe adları yazıdır, etiketler kapalı başlar);
  - [ADR 9313](9313-fonts-folder-listing-upright-texts.md) (yazılar dik).
- **`.kcad`:** belge şeması 257 (§5).

## Bağlam

- **Nokta aracı:** adlı noktanın adını noktanın etiketi olarak yazıyordu (`EntityBase.label`). ADR 9311'den beri
  katmanın etiketi kapalı başladığı için ad görünmüyordu. Kişi katmanın etiketini açınca da ad etiket motorunun
  yerleşimiyle çiziliyordu: elle yeri değişmez, DXF'e gitmez.
- **Köşe noktalarını numarala:** ADR 9311'den beri adları Metin nesnesi olarak yazar, ama yazı noktayı bilmez.
  Nokta taşınınca yazısı yerinde kalır.
- **Bağlı yazı (ADR 0175 §4):** yazı nesnesinin kalıcı kimliğini (`labelOf`) ve ölçeğini (`labelScale`) taşır. Nesne
  değişince yazı etiket kuralıyla yeniden yazılır: yerini katmanın etiket stili verir. Yazı elle taşınınca bağ
  kopar.

### Araştırma (26 Kasım 2026'da okundu)

- **Civil 3D:**
  - COGO noktasının etiketi noktaya bağlıdır; nokta taşınınca etiketi de gider.
  - Etiket kendi tutamacıyla sürüklenebilir (“dragged state”); “Reset Label” ilk yerine döndürür
    ([Changing the Dragged State of a Label](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Tutorials/files/GUID-057893BF-B66F-4F60-8ABC-A0C935E986F9.htm),
    [IsLabelDragged](https://help.autodesk.com/cloudhelp/2022/ENU/Civil3D-API/files/html/3ab188b1-fd04-49dc-880f-24e2cd8e20aa.htm)).
- **BricsCAD Civil noktası:**
  - iki tutamacı vardır: noktanınki ve etiketinki;
  - etiketi sürüklemek kılavuz çizgi açar, tutamaca yeniden basmak sıfırlar
    ([Civil point leader and components in dragged state](https://help.bricsys.com/en-us/document/bricscad/civil-tools/civil-points/understanding-civil-point-leader-and-components-in-dragged-state)).
- **Netcad:** nokta yazısının noktayla ilişkisini anlatan resmî bir kaynak bulunamadı. Karar yukarıdakilere dayanır.
- **KentOS kararı:**
  - nokta ile adı ayrı nesnelerdir: yazı gerçek bir Metin nesnesidir, DXF'e gider, elle düzenlenir;
  - yazı noktasına bağlıdır, nokta taşınınca birlikte gider;
  - elle kaydırılan yazı bağını korur (sürüklenmiş etiket gibi), elle metni değişen yazı bağını koparır.

## Karar

### 1. Nokta aracı adı yazı olarak yazar

- **Yazı:** adlı noktayı yazan araç adını Metin nesnesi olarak da yazar, aynı adımda.
  - Yeri noktanın sağ üstüdür: köşe adının kuralı (`corner_text_at`), dışa doğru yön sağ üstün açıortayı.
  - Yüksekliği projenin ölçü yazısı yüksekliğidir (`annotation_mm(Measure)`), çizim ölçeğinde.
  - Yazı tipi projeninkidir, yazı diktir.
  - Katmanı noktanın katmanıdır: katman gizlenince ikisi birlikte gizlenir.
- **Etiket motoru kullanılmaz.** Noktanın etiketi (`label`) adı tutmaya devam eder. Bağlı yazısı olan noktanın etiketi
  çizimde ve paftada çizilmez (ADR 0175 §4): yazı onun yerini alır.
- **Adsız nokta** yazı yazmaz.

### 2. Bağ: noktanın adı (`labelOf`, ölçeksiz)

Ölçeksiz `labelOf` noktanın adının bağıdır; ölçekli bağ ADR 0175'in etiket kuralıdır. Belge bağı her adım kaydedilmeden
önce izler (`follow_links`):

| Olay (hangi komut, araç, Python ya da MCP yaptıysa) | Yazı |
|---|---|
| Nokta taşındı | Aynı kadar kayar, aynı adımda. Yazı aynı adımda kendisi de taşındıysa (birlikte seçilip taşındı) ikinci kez kaydırılmaz. |
| Noktanın adı değişti | Metni yeni ad olur |
| Noktanın adı silindi | Yazı silinir |
| Nokta silindi | Yazı silinir |
| Yazı elle taşındı, döndürüldü, boyu ya da hizası değişti | Bağ kalır; yazının noktaya göre yeri değişir |
| Yazının metni elle değişti | Bağ kopar; yazı kendi başına kalır |
| Yazı kopyalandı ya da bloğa girdi | Kopya bağ taşımaz (ADR 0175 §4) |

- **Adın değiştiği yerler:** Noktalar (nokta listesi), Öznitelikler'in “Nokta adı” satırı, Python ve MCP adı
  `cad.entities.set` ile noktanın adına (`label`) yazar; yazı hepsinde yeni adı alır.
  - Öznitelikler'de adı yansıtan “Nokta” ya da “Ad” özniteliği değişince ad da aynı adımda değişir, yazı da onunla.
    Parsel ve Ada'nın var olan kuralı bu iki özniteliğe de genişledi.
- **Komut:** `cad.entities.create`'in yeni nesnesi `labelOf`'u `labelScale`'siz yalnız bir noktaya bağlayabilir.
  Başka bir nesneye bağlanan yazı `invalid_link` ile reddedilir.
- **Bağı kopar:** `cad.entities.set`'in `unlink`'i bu bağı da koparır.

### 3. Köşe noktalarını numarala

“Nokta ve yazı” çıktısında her köşenin yazısı aynı çalışmada yazılan noktasına bağlanır. İşlem aracının değişiklik
kümesi bunu `name_links` ile söyler: yeni nesneler arasında hangi yazının hangi noktanın adı olduğu. Uygulayıcı önce
noktaları yazar, kalıcı kimliklerini yazılara verir, sonra yazıları yazar; hepsi tek adımdır.

### 4. Komutlar ve kanallar

- **`cad.point.create`:**
  - `nameText` (açık/kapalı, öntanımı kapalı): ad (`label`) varsa yazısını da yazar;
  - plan yazıyı da gösterir (`text`);
  - çıktı yazının kimliğini verir (`textUid`, `textId`).
- **Nokta aracı:** adı soran programlarda (`ToolOption::PointNames`: Harita, Yol, hepsi bir arada) `nameText` açık
  yazar.
- **Python:** `cad.point.create(..., name_text=True)`. **MCP** ve **yapay zekâ ajanı** aynı komutu çağırır.
  **Eklenti** de aynı komutu çağırır.

### 5. Dosya

- **Şema 257:** `.kcad` belge şeması 257'de bir yazının `labelOf`'u `labelScale`'siz yazılabilir. Yazıcı 257'yi yalnız
  böyle bir yazı varken yazar; başka her çizim baytı baytına aynı kalır.
- **Okuyucu:** daha eski şemalı dosyada ölçeksiz `labelOf` bozuk dosyadır (`bad_value`), eskisi gibi.
- **Sütunlar:** yazıların tipli sütunları ölçeksiz bağı ölçeğin yerine NaN yazarak taşır.
- **Bağımsız Python:** yazıcı ve okuyucu (`scripts/fixtures/kcad_v2_reference.py`, `tools/kcad/kcad.py`) aynı kuralı
  uygular.
- **DXF:** bağı yazmaz; yazı düz yazıdır (ADR 0175 §4).

### 6. Modüller

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Harita | var | Nokta aracı adı sorar; köşe numaralandırma Harita'nındır |
| Yol | var | Nokta aracı adı sorar (kamulaştırma, aplikasyon noktaları); köşe numaralandırma Yol'da da var |
| Mimari | yok | Nokta aracı ad ve kod sormaz (ADR 0802); bağ çekirdekte olduğu için Mimari'de yazılan bağlı yazı da izlenir |
| CBS | yok | Nokta veri tablosunun satırıdır, adı öznitelik; adı etiket motoru gösterir |

### 7. Kanallar

| Kanal | Var / yok | Nasıl |
|---|---|---|
| Python | var | `cad.point.create`'in `name_text`'i; bağ belgenindir, Python'un taşıma ve düzenleme komutları da izler |
| MCP | var | Aynı komut ve alan |
| Yapay zekâ ajanı | var | MCP ile aynı |
| Eklenti | var | Aynı komut; köşe numaralandırma eklentinin çalıştırdığı araçla da bağlı yazar |

Tam uyum: bağı her istemcide belgenin aynı kuralı izler (`kentos_domain::Document::follow_links`). Kanal yazıyı
kendisi taşımaz.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/core/entity.rs` (`labelOf` ve `labelScale`'in açıklaması),
  `crates/shared/commands/src/core/cad_primitives.rs` (`nameText`, `textUid`, `textId`, plan `text`), `cad_create.rs`.
- **Belge:** `crates/native/domain/src/linked.rs` (ölçeksiz bağın kuralı).
- **Komutlar:** `crates/native/application/src/core/point.rs` (adın yazısı), `create.rs` (ölçeksiz bağın denetimi).
- **İşlem aracı:** `crates/native/processing/src/types.rs` (`ChangeSet::name_links`), `runner.rs` (`add_named`:
  yeni nesneler kalıcı kimlikleriyle tek adımda), `builtin/vertex_numbering.rs`.
- **Araç ve masaüstü:** `crates/native/interaction/src/core/point.rs`, `apps/desktop/src/gis/points/edit.rs` (Satır
  ekle), `apps/desktop/src/cad/properties/commit.rs` (Nokta ve Ad öznitelikleri).
- **Dosya:**
  - `crates/shared/kcad/src/{schemas,lib}.rs`: şema sabitleri `schemas.rs`'e taşındı, `lib.rs` 2 000 satırın altına
    indi;
  - `encode/{schema,objects}.rs`, `decode{,/objects,/objects/object,/objects/build,/objects/features}.rs`,
    `columns/unpack.rs`;
  - `docs/specs/kcad-v2.md`, `tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`.
- **Örnekler:** `fixtures/kcad/v2/name-links.{json,kcad}`, `broken/name-links-in-schema-256.kcad`. “Çok yeni şema”
  örneği `schema-version-258.kcad` oldu.

## Doğrulama (26 Kasım 2026)

- **Belge:** `fixtures/document-ops/v1/linked-texts.json`'a üç senaryo eklendi; `every_document_fixture_matches_the_native_document`
  geçti. Senaryolar şunları sınar:
  - taşıma (yazı aynı kadar kayar);
  - ad değişimi (metin yeni ad olur), ad silme (yazı silinir), geri alma;
  - elle kaydırma (bağ kalır, sonra yine izler);
  - birlikte taşıma (bir kez kayar);
  - metni elle değişen yazı (bağ kopar);
  - nokta silme (yazı silinir).
- **Dosya:** `cargo test -p kentos-kcad` 287 sınama geçti. Yeni `name_links`: şema 257, gidiş-dönüş, sütunlar, şema
  256'nın reddi. `kcad_v2_reference.py --check`: 1 896 dosya tutarlı.
- **Komutlar:** `cad.point.create` (iki yeni durum: bağlı yazı, adsız noktada yazı yok), `cad.entities.create` (ölçeksiz
  bağın reddi ve kabulü), `cad.entities.set`, `cad.entities.transform` ve `cad.processing.run` ortak durumları ile
  `capabilities` geçti. `point::tests`: yazının yeri, yüksekliği, bağı ve tek adım.
- **İşlem aracı:** `kentos-processing --test cases` 38 sınama geçti. Köşe numaralandırmanın ve parsel ölçü yazıları
  modelinin yazıları noktalarına bağlı (`$added:N`); pencere ile komut aynı.
- **Etkileşim:** `kentos-interaction` 1 005 sınama geçti.
- **Masaüstü:** noktalar, Öznitelikler, etiketler ve ortak süzgeçlerle 275 sınama geçti (bağımlılık tabanı bu işten
  önce de kırmızıydı). Harita paketi 195, Yol paketi 129 sınama geçti.
- **Python:** `test_point_names` (yazının yeri ve bağı; taşıma ve ad değişimi; dosyada bağ) ile `cad.point.create`,
  `cad.entities.create` ve `cad.processing.run`'ın ortak durumları geçti.
- **Resimler** (`.run/gui-evidence/kullanim-9314/`, `point_names_screens`):
  - `nokta-adlari-yazi.png`: Noktalar'dan yazılan üç adlı nokta, adları sağ üstte yazı;
  - `nokta-tasindi-adi-degisti.png`: N2 6 m kuzeye taşındı, adı da; N3 Noktalar'da N30 oldu, yazısı da.
- **Koşulmayanlar:**
  - gerçek pencerede fare ve klavyeyle kabul;
  - donmuş web'in belge durumları;
  - `scripts/dev/source_size.py`'nin tabanı: `kcad/src/lib.rs` sınırın altına indi, `encode/objects.rs` küçüldü. Tabanın
    bu iki satırı sahibin `--write`'ıyla düşürülmeli.

## Açık kalanlar

- **Yazıyı sıfırla:** elle kaydırılan yazıyı ilk yerine döndüren komut yok; kişi yazıyı taşır ya da yeniden yazar.
- **İçe almada adlar:** koordinat listesinden ve GNSS'ten alınan noktaların adları hâlâ etikettir (ADR 9311 §Açık
  kalanlar).
