# 2080 — Harita işi: dosyası olmayan öğenin içerik revizyonu ve onay damgası

Tarih: 2026-11-08. plan/map-cad.md MAP-01, üçüncü tur. Durum: taslak. Plan satırları:

- “Kaynağı, kaynaktan türetilmiş nesneyi ve kullanıcının bağımsız çizimini özelliklerde ayır. Nesne kimliği, kaynak
  revizyonu, üretim işlemi ve güncellik bilgisi birlikte erişilsin.”
- “İşin durumu ‘taslak / denetlenecek / onaylı teslim revizyonu’ gibi açık kapsamla tutulabilsin.”

İlgili:

- [ADR 1200](1200-map-work-structure-and-provenance.md): Harita işi. Bu ADR onun “Sonra”sındaki dosyasız öğe revizyonunu
  ve onaylı teslim revizyonunun içerik damgasını ele alır;
- [ADR 1201](1201-map-work-relative-files-relink-and-contextual-tools.md): göreli yollar, Seçimi türet;
- [ADR 1320](1320-sheet-sources-and-update-publication.md) §2: nesne özeti `kentos.object-digest/1`.

## Dayanak

- **W3C PROV-DM** (ADR 1200 §Dayanak): revizyon (wasRevisionOf) bir varlığın yeni hâlidir; türetme (wasDerivedFrom) bir
  varlığın ötekinden üretilmesidir. Revizyonu olmayan bir varlıktan türetilen şeyin güncelliği söylenemez.
- **Git** ([Pro Git, Git Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects), 8 Kasım 2026):
  - içeriğin adı, türünü ve boyunu taşıyan başlıkla içeriğin özetidir;
  - ağaç nesnesinin her girdisi bir içeriğin özetini, türünü ve adını taşır;
  - aynı içerik aynı adı alır.

  KentOS'un içerik revizyonu bir ağaç gibidir: öğenin nesnelerinin kalıcı kimlikleriyle özetlerinin sıralı listesinin
  özeti.
- **Civil 3D veri kısayolları** ([To Synchronize a Reference Object](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-B079A088-6594-4095-A50B-7A8FC5770C5C.htm),
  8 Kasım 2026):
  - kaynak nesne değişince başvurusu güncel değildir;
  - Prospector ağacında ve durum çubuğunda işaretlenir;
  - Synchronize değişikliği başvuruya çeker.

  KentOS'ta türetme kaydı güncel değil görünür; yeniden üretmek kullanıcının işidir (ADR 1200 §Sonra).
- **ISO 19650'nin ortak veri ortamı** (Hong Kong DEVB, *Appendix I – ISO 19650 Terminologies*, 2021, s. 12. madde,
  okunan PDF):
  - WIP, SHARED ve PUBLISHED durumları BS 1192'deki gibidir;
  - ARCHIVED durumu durumlar arasındaki bütün geçişleri (kapıları) ve bütün durum (uygunluk) ya da revizyon
    değişikliklerini kaydeder.

  KentOS'ta onay bir kapıdır: onaylanan revizyon damgalanır, sonraki değişiklik görünür. ISO 19650-1'in kendisi
  okunmadı (ücretli).
- **KentOS bugün** (ADR 1200, 1201):
  - dosyası olan öğenin revizyonu dosyanın SHA-256'sıdır; dosyası olmayan öğenin (tasarım, teslim, çalışma) revizyonu
    yoktu;
  - bu yüzden tasarımdan türetme reddediliyordu (`work_item_no_file`);
  - onaylı tasarım sonradan değişse de “Onaylı” görünüyordu.

  Pafta yayını sayfalarının kaynak nesnelerinin özetini zaten yazıyor (`kentos.object-digest/1`, ADR 1320).

## Karar

1. **İçerik revizyonu (`kentos.work-content/1`).** Dosyası olmayan öğenin revizyonu şöyle hesaplanır:
   - öğenin tuttuğu nesneler alınır: katmanlarındakiler ve gruplarının altındaki katmanlardakiler (katman profili);
   - `[[kalıcı kimlik, nesne özeti], …]` listesi kimliğe göre sıralanır;
   - listenin boşluksuz JSON'unun SHA-256'sı revizyondur.

   Nesne özeti ADR 1320'ninkidir: yuva kimliği olmadan, anahtarları sıralı nesne JSON'unun SHA-256'sı. Nesnesi olmayan
   öğenin revizyonu `[]`'nin özetidir.

   Katmanın görünüşü (renk, stil) içerik değildir; nesnenin kendi alanları (öznitelik, kendi rengi) içeriktir.

   Revizyon dosyada saklanmaz, okunurken nesnelerden hesaplanır:
   - hızlı okuma onu yalnız yargılandığı yerde hesaplar: bir türetmenin kaynağı ya da damgalı onaylı öğe;
   - derin okuma her dosyasız öğe için hesaplar.

   Gerekçe: büyük çizimde her düzenlemeden sonra bütün nesneler özetlenmesin. Şeridin her karede sorduğu “seçimin rolü”
   hiçbir şey hesaplamaz (`map_work::role_of`).
2. **Dosyası olmayan öğeden türetme.**
   - Kaydın revizyonu öğenin o anki içerik revizyonudur; içerik sonra değişince kayıt güncel değildir.
   - Kaynağın kendi katmanındaki nesne o öğeden türetilemez (`own_content`): tasarımın kendi nesneleri onun içeriğidir.
   - Yeniden bağla ve Yenile yalnız dosyası olan öğenindir (`work_item_no_file`).
3. **Onay damgası (`approved`).**
   - Onaylı duruma geçen öğe o anki revizyonunu damgalar: dosyası olan dosya revizyonunu, olmayan içerik revizyonunu.
   - Damga bugünkü revizyondan farklıysa öğe `revised` olur. Hapı “Onay eskidi”dir: kısa, ki satırın adı dar panelde
     okunsun. Nedeni ipucundadır: “Onaydan sonra içeriği (dosyası) değişti (onaylanan …, şimdi …): değişikliği
     denetleyip yeniden onaylayın.”
   - Onaylı öğeyi yeniden onaylamak bugünkü revizyonu damgalar; değişmemiş öğede “Değişecek bir şey yok”.
   - Onaydan çıkan öğenin damgası kalkar.
   - Damga ofisin kaydıdır, kurum onayı değildir.
   - Şema 206'dan önce onaylanmış öğenin damgası yoktur; yargılanmaz, yeniden onaylanınca damgalanır.
4. **Sözleşme.**
   - Yeni alanlar: `WorkItem.approved`, `WorkRow.content` ve `.approved`, `MapWorkEdited.content` ve `.approved`.
   - Yeni durum `WorkState::Revised`, yeni ret `own_content`.
   - `map.work.query` ve `map.work.edit` v1 kalır: alanlar isteğe bağlıdır, dosyasız kaynak önceden reddedilen bir
     girdiydi.
5. **`.kcad` şema 206.**
   - `approved` alanı ve dosyası olmayan öğeden türetme şema 206'dandır.
   - Yazıcı 206'yı yalnız bunlardan biri varken seçer.
   - 205 ve öncesi damgayı `unknown_field`, dosyasız kaynağı `bad_value` sayar.
   - Örnek `map-work-stamps`; ayrıntı docs/specs/kcad-v2.md.
6. **Masaüstü (yalnız KentOS Harita).** Harita işi panelinde:
   - satırın ipucunda “İçerik revizyonu …” ve “Onaylanan revizyon …” yazar;
   - `revised` satırın hapı “Onay eskidi”, düğmesi “Yeniden onayla”dır;
   - Seçimi türet dosyası olmayan öğeleri de sunar; seçimin nesnelerini tutan dosyasız öğe sunulmaz.

   Şeritte yeni komut yok: mevcut Harita işi paneli ve bağlamsal sekmelerin Harita işi paneli.
7. **Kapsam işareti.** Harita işi `cadWork: map` projesinin ayarı olduğu için bu dilim **yalnız Harita (CAD)**'dır.
   Nesne özeti (`kentos.object-digest/1`) Harita ve CBS'nin ortak biçimidir.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_work_cases.py` ADR'den elle yazılır (hashlib ve json; KentOS kodu yok).
  `map.work.query` 30 (27'den), `map.work.edit` 54 (48'den). Yeni durumlar:
  - tasarımdan türetilen çizgi: öznitelik değişince güncel değil, geri alınca güncel;
  - onaylı tasarımın damgası: değişince `revised`; ortofotonun başka dosya revizyonu damgası;
  - onayın damgası: dosyalıda dosya revizyonu, dosyasızda içerik revizyonu; yeniden onay, değişmemiş öğe, onaydan çıkış;
  - `own_content`, Yenile'nin dosyasız öğede reddi.

  Rust ve Python geçer; yetenek listesi denetimi geçer.
- **`.kcad`:** `map-work-stamps` örneği (içerik revizyonları Python'da hesaplanmış) ve bozuklar:
  - `map-work-stamps-in-schema-205`;
  - `map-work-stamp-not-approved`, `map-work-stamp-not-sha`;
  - artık şema 205'te yazılan `map-work-no-file-source`.

  1689 dosya bağımsız okuyucuyla aynı sonucu verir; Rust kcad testleri 242.
- **MCP** (`crates/native/mcp/tests/map_work_stamps.rs`): şema 206 örneği açılınca iki onay ve kayıt güncel okunur.
  Python'un yazdığı içerik revizyonu Rust'ınkiyle aynıdır. Tasarım denetime dönünce damga kalkar, yeniden onayda aynı
  damga gelir.
- **Masaüstü** (`map_work_stamps_tests.rs`):
  - panelden onay damgalar; çizgi değişince “Onay eskidi”; yeniden onay ve geri alma;
  - Seçimi türet tasarımı sunar, tasarımın kendi çizgisine sunmaz; tasarım değişince kayıt güncel değil;
  - kaydet ve yeniden aç: aynı damga ve kayıt güncel.

  Mevcut map_work testleri ve çapraz filtreler 139 geçer. Resimler `.run/gui-evidence/harita-isi-damga-*`.

## Sonra

- Nesne türüne göre bağlamsal araçlar (yüzey, plan alanı, altyapı) ve tipli katman profili (MAP-01'in kalanı).
- Onaydan sonra hangi nesnelerin değiştiği: bugün yalnız revizyon farkı söylenir, nesne başına damga yok.
- İçe aktarmanın türetme kaydını kendiliğinden yazması (sahibin kararı bekliyor).
- Güncel olmayan türevin yeniden üretimi ya da güncel kabul edilmesi (Civil 3D'nin Synchronize'ı).
- Bağlı GeoPackage'ın geri alınabilir yeniden bağlanması.
