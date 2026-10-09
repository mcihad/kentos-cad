# Madde tarifi: bir TODOS maddesi baştan sona

Bu belge bir TODOS.md maddesinin hangi sırayla ve nasıl yapıldığını anlatır. Örnekler GIS-10 (ADR 0208, harita servisleri) ve
öncekilerden alınmıştır. Amaç, aynı işi başka bir ajanın da aynı sırayla, aynı denetimlerle ve aynı kaliteyle yapabilmesidir.

Kuralların kaynağı CLAUDE.md'dir. Bu belge onları iş sırasına koyar ve sık atlanan ayrıntıları yineler. Çelişki olursa CLAUDE.md ve
sahibin son kararı geçerlidir.

## 0. Değişmeyen ilkeler

- **Bir madde tek parçadır:** bir ADR, bir commit (ADR içinde), bir tam takım, bir push ve bir rapor. Madde alt adımlara bölünüp ayrı
  ayrı commit'lenmez.
- **İki platform:** masaüstü (Rust/Iced, `apps/desktop`) ve web (TS/DOM, `apps/web`) birlikte yapılır. Gerekiyorsa bulut (`kentosd`)
  ve otomasyon (Python, MCP) da eklenir. Sahip aksini söylemedikçe platform sorulmaz.
- **Hesap tek yerdedir:** ortak Rust çekirdeği (`crates/shared/*`). Web onu WASM'la çağırır, masaüstü doğrudan. TS'te geometri ya da
  sayı algoritması yazılmaz.
- **Bağımsız doğrulama:** her kural ve hesap, KentOS kodu kullanmadan yazılmış bir Python başvurusuyla denetlenir (`--check`).
  - Beklenen değer KentOS'un kendi çıktısından üretilmez.
  - Tolerans büyütülmez.
  - Fixture hataya göre yenilenmez.
- **Önce performans** (sahibin önceliği):
  - Ağır iş arayüz iş parçacığında yapılmaz: masaüstünde ayrı iş parçacığı, web'de işçi (Web Worker).
  - Toplu veri tipli dizilerle taşınır, JSON'la değil.
  - Karede bellek ayrılmaz.
  - Her ADR'de bütçe yazılır, release derlemesinde ölçülür.
- **Dil:**
  - Arayüz Türkçedir; adlar AutoCAD'in Türkçe arayüzündeki adlardır.
  - Kod, tanımlayıcılar ve yorumlar İngilizcedir.
  - Belgeler (ADR, TODOS) Türkçedir.
- **İkonlar:** her menü satırının ve her aracın kendi ikonu olur; aynı ikon tekrarlanmaz. İkonlar sahibe sorulmadan seçilir, resimlerde
  gösterilir ve raporda adları söylenir.
- **Fare önce:** her seçenek tıklanabilir, gizli açılır menü olmaz. Nokta ya da nesne isteyen her alanda “Sahneden seç” bulunur.
- **Sahibe ne sorulur:** yalnız gerçekten onun kararı olanlar (yeni bağımlılık, ürün yönünde bir çatallanma). Gerisine ajan karar
  verir, kararı ADR'ye yazar ve raporda söyler.
- **Sıranın dışında kalanlar:** mevzuatla düzenlenen maddeler (GIS-06, GIS-07 …) yol haritasının sonuna kalır. Sahip “ertelendi”
  dediği maddeleri (HYB-24 …) atla.

## 1. Başlamadan

1. **Oku:**
   - TODOS.md'deki maddeyi ve bağlı olduğu maddeleri,
   - maddenin andığı ADR'leri, CLAUDE.md'nin ilgili bölümlerini ve DESIGN.md'yi,
   - benzer bir önceki maddenin ADR'sini ve commit'ini. `git show --stat <commit>` hangi dosyalara dokunulduğunu gösterir; bu, en
     iyi kontrol listesidir.
2. **Adları ve numaraları ayır:**
   - ADR numarası (`ls docs/adr/`).
   - Gerekiyorsa `.kcad` şeması ve `FORMATS_VERSION`.
   - Yeni fixture, iz, modül ve dosya adları. Her adın boş olduğunu denetle; ADR 0181'in `navigation.json`'u bir kez başka bir izin
     üstüne yazılmıştı.
   - Paralel çalışmada §14'teki kurallara uy.
3. **Kapsamı kararlaştır:**
   - Profesyonel karşılıklarına bak (ArcGIS, QGIS, Netcad, AutoCAD).
   - Sık kullanılan işleri al; kapsam dışını da yaz.
   - Maddeyi ADR adımlarına böl, ama tek parçada bitir.

## 2. ADR'yi önce yaz (`docs/adr/NNNN-ad.md`)

Kod yazılmadan önce kararlar yazılır. Kurallar, bağımsız başvuru yalnız ADR'den yazılabilecek kadar kesin olmalıdır: sıralar, eşitlikte
hangisinin seçildiği, birimler, sınırlar, ret kodları ve iletileri.

```markdown
# ADR NNNN: Başlık

- **Durum:** kabul edildi (YYYY-AA-GG). Kapsam: sahibin sözleri ya da kararları (alıntıyla); ilke “Performance First”; madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `ID`, ilgili ADR'ler.

## Bağlam
Kullanıcının sorunu, profesyonel araçların karşılığı, KentOS'ta bugün ne olmadığı.

## Karar
### 1. Kapsam              (ne var, ne yok; kapsam dışı ayrıca)
### 2. Veri modeli         (sözleşme türleri, kurallar, `.kcad` şeması)
### 3. … kurallar ve hesaplar (kesin tanımlar)
### n. Arayüz              (komut kimlikleri, araçlar, pencereler, şerit, kısayol harfleri)
### n. Komutlar ve otomasyon (ürün komutu, Python, MCP)
### n. Performans          (nerede çalışır, bütçeler)

## Uygulama
(madde bitince doldurulur)

## Doğrulama
(madde bitince doldurulur)
```

## 3. Sözleşme (`crates/shared/contracts`)

- **Türler:** `serde` (`rename_all = "camelCase"`), `ts-rs` (`#[cfg_attr(feature = "ts", ts(export))]`) ve `schemars` ile tanımlanır.
- **Kurallar:** kuralları `problem()` ve listeler için `..._problem()` biçiminde, Türkçe iletilerle yaz. Aynı kurallar okuyucuda,
  komutta ve sunucuda kullanılır. Projede saklanan ayar için bir de `sanitized` yaz.
- **Modül kaydı:** `src/lib.rs`'e `pub mod …; pub use …::*;` ekle.
- **Proje ayarına yeni alan:** `ProjectSettings` struct'ı açıkça yazan her yere alanı ekle:
  `grep -rn --include='*.rs' "connections: Vec::new()," crates apps`.
- **TypeScript türleri:** `cargo test -p kentos-contracts` çalıştır; ts-rs çıktıyı `apps/web/src/contracts/generated/`'a yazar. Bu
  klasör elle düzenlenmez.
- **Komut kataloğu:** `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts catalog`. Yeni komut için `catalog.rs`'e bir
  `CommandDescriptor` ve örnekler ekle; katalog testindeki girdi ve çıktı tür eşlemesine de ekle.
- **Ayar şeması** (kullanıcı ayarı eklendiyse): `KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings`.

## 4. Dosya biçimi (`.kcad`), gerekiyorsa

Projede ya da nesnede kalıcı yeni bir alan varsa yeni bir şema numarası gerekir.

- **Kodek** (`crates/shared/kcad`):
  - `lib.rs`: `SCHEMA_WITH_*` sabitini ve `SCHEMAS` listesini ekle.
  - `encode.rs`:
    - ayarın alan sayısını güncelle;
    - anahtarı RFC 8949'un sırasına koy (önce uzunluk, sonra baytlar);
    - `schema_of`'a ekle. Kural: yalnız yeni alan varken yeni şema yazılır, öbür çizimler bayt bayt eskisi gibi kalır.
  - `decode.rs`, `decode/objects.rs`: `Features` bayrağını ekle.
  - Konu için ayrı modüller aç: `encode/<konu>.rs`, `decode/<konu>.rs`. Yazıcı da okuyucu da sözleşmenin kurallarıyla bütünü denetler.
- **Sürüm:** `FORMATS_VERSION`'ı iki yerde artır: `crates/shared/contracts/src/formats.rs` ve `apps/web/src/io/version.ts`.
- **Spesifikasyon:** `docs/specs/kcad-v2.md`'ye yeni bölümü yaz.
- **Bağımsız Python:**
  - Okuyucu `tools/kcad/kcad.py`: sabitler, alanların tablosu ve kuralların bağımsız kopyası.
  - Yazıcı `scripts/fixtures/kcad_v2_reference.py`: örnek dosya (`fixtures/kcad/v2/<konu>.kcad` ve `.json`), bozuk dosyalar
    (`fixtures/kcad/v2/broken/`), `expected.json`.
  - Gelecek sürümü temsil eden bozuk dosyayı bir ileri taşı: `broken/schema-version-N.kcad` → `N+1`.
  - Denetim: `python3 scripts/fixtures/kcad_v2_reference.py --check` ve `python3 tools/kcad/kcad.py validate DOSYA`.
- **Web:** `apps/web/src/io/kcad.ts`, `model/snapshot.ts`; nesne türüyse tipli sütunlar `io/columns.ts` ↔ `crates/shared/kcad/src/columns.rs`.
- **Başka biçimler:** DXF, GeoJSON ve Shapefile alanı taşıyamıyorsa bunu söylemeleri gerekir (kayıp raporu).

## 5. Çekirdek hesap ve bağımsız başvuru

- **Yer:** `crates/shared/geometry-core/src/ops/<konu>.rs` (çoğu zaman), büyükse kendi crate'i (`crates/shared/<konu>`).
- **Saflık:** DOM, Iced, ağ ve SQL bilmez. `jsmath` ve `libm` kurallarına uyulur (yasak `f64::max` yerine `js_max` gibi). Kullanıcı
  girdisinde `unwrap` ve `expect` ile panik üretilmez.
- **Girdi ve çıktı:** web ile masaüstü için JSON çağrıları (`ops::<konu>::calls`), sıcak yolda tipli tamponlar. Geometri deposunda
  (`store/`) olan şekiller oradan okunur.
- **Bağımsız başvuru:** `scripts/fixtures/<konu>_cases.py`. KentOS kodu kullanmadan ADR'den yazılır. mpmath (50 basamak), kesirler,
  pyproj/PROJ, GDAL, OWSLib, shapely ya da laspy gibi kütüphaneler kullanılabilir.
  - Çıktısı `fixtures/<konu>/v1/cases.json` dosyasıdır.
  - `--check` bayrağı dosyanın betiğin yazdığıyla aynı olduğunu denetler.
  - Kural: betik beklentiyi KentOS'tan almaz. Fixture değişirse farkı oku.
- **Testler:** çekirdeğin testleri aynı dosyayı okur (`crates/shared/<crate>/tests/all/<konu>.rs`). Yeni test dosyası `tests/all/`'a
  modül olarak eklenir. Web için WASM testi `*.wasm.test.ts`.
- **WASM:** bağlayıcı `crates/wasm/<paket>/src/`; derleme `pnpm wasm`. `pkg/` elle düzenlenmez. Ağır ve seyrek kullanılan modül
  tembel yüklenir (`io/<konu>/module.ts` gibi).
- **TS sarmalayıcıları:** `apps/web/src/model/ops/<konu>.ts` (`op<…>('ad')`). Hesap yeniden yazılmaz.

## 6. Ürün komutu, gerekiyorsa

Belgeyi değiştiren her yeni iş bir ürün komutundan geçer. Python ve AI ayrı bir mutasyon yolu açmaz.

- **Sözleşme:** `crates/shared/contracts/src/cad_<konu>.rs`. Girdi, çıktı, plan, ret kodları ve sıraları doc yorumunda yazılır.
- **İşleyiciler:**
  - web `apps/web/src/product/<komut>.ts`, `product/registry.ts`, `product/checks.ts`;
  - masaüstü `crates/native/application/src/<komut>.rs`, `codes.rs`, `checks.rs`, `lib.rs`.
- **Ortak durumlar:** `fixtures/commands/v1/<komut>.json`. Bunları sözleşmenin kurallarından yazan bağımsız betik
  `scripts/fixtures/<konu>_command_cases.py --check`'tir.
- **Oynatıcılar** (yeni bir beklenti türü varsa üçüne de eklenir):
  - web `apps/web/src/product/fixtures.test.ts`,
  - masaüstü `crates/native/application/tests/all/fixtures.rs`,
  - Python `python/tests/test_command_cases.py`.
- **Başsız sunucu:** `crates/native/headless/src/dispatch.rs`.
- **Python SDK:** `python3 scripts/python/sdk.py` (katalog değişince; `--check` farkı arar).
- **MCP:** `crates/native/mcp/src/tools.rs` (silen ya da yıkıcı işaretiyle) ve testi `crates/native/mcp/tests/mcp.rs`.

## 7. Masaüstü ve web arayüzü

İki platform aynı davranışı ve aynı adları taşır. Davranış ortak fixture'larla eşit tutulur.

| İş | Web | Masaüstü |
|---|---|---|
| Araç (çizimde tıklanan) | `apps/web/src/tools/<ad>.ts`, `tools/catalog.ts` (kimlik, grup, yöntem, `steps`) | `crates/native/interaction/src/<ad>.rs`, oturumun araç listesi |
| Komut kaydı | `apps/web/src/app/commands.ts` ya da `app/<konu>Commands.ts`; `isEnabled` için `watch` | `apps/desktop/src/app.rs` (yönlendirme ve `enabled`), `apps/desktop/src/catalog.rs` |
| Menü ve şerit | `apps/web/src/app/menus.ts`, `app/ribbon.ts` (CAD ve CBS şeritleri ayrı) | şerit envanterden gelir |
| İkon | `apps/web/src/ui/icons.ts` ya da `ui/<konu>Icons.ts` | ikonlar envanterden gelir |
| Pencere ve panel | `apps/web/src/ui/<konu>/` (DOM `h()`, `confirmDialog`), CSS jetonları `styles/` | `apps/desktop/src/<konu>/` (KentOS UI bileşenleri) |
| Öznitelikler | `ui/properties/` | `apps/desktop/src/properties/` |
| Alt panelin sekmesi | `ui/bottom/`, `app/layoutPlan.ts`, `ui/bottom/logPlan.ts` | `bottom.rs`, `layout_plan.rs`, `log_plan.rs`; ortak `fixtures/shell/v1/layout.json`, `log.json` (`scripts/fixtures/layout_cases.py`, `log_cases.py`) |
| İşlemler aracı | `apps/web/src/processing/builtin/` | `crates/native/processing/src/builtin/` |

- **Masaüstünde araç:** araç kamerayı kendisi değiştirmez; `ViewChange` ister, kabuk uygular. Uzun iş için kabuktan sonuç bekler
  (soru ve cevap biçimi), kendi iş parçacığını açmaz.
- **Görünüş:**
  - Sabit renk ve yazı boyutu eklenmez; jetonlar ve palet kullanılır.
  - Her pencere 1100×650'de taşmamalı, düğme kesilmemeli.
  - Açık ve koyu temada, büyük yazıyla da denetlenir.
- **Masaüstünün kaydı:** masaüstünün taşıdığı komutlar ve pencereler kayda geçer.
  - `KENTOS_WRITE_PORTED=1 cargo test -p kentos-desktop ported` `apps/desktop/ported.json`'u yeniden yazar.
  - Web penceresinin masaüstü karşılığı `apps/desktop/equivalents.json`'a yazılır (`screens` bölümü, `{ "desktop": "implemented",
    "where": "…" }`).

## 8. Ortak etkileşim izleri

Çizimde kullanılan her araç için bir iz yazılır: `fixtures/interaction/v1/<ad>.json` (adımlar, beklenen nesneler, katmanlar, seçim).

- **Sahne:** gerekiyorsa `fixtures/interaction/v1/<ad>.kcad`. Mümkünse bağımsız bir Python betiğiyle üretilir
  (`scripts/fixtures/<ad>_scene.py --check`).
- **Oynatıcılar:**
  - web `pnpm e2e:interaction` (tarayıcıda, üç varyant),
  - masaüstü `cargo test -p kentos-desktop traces` (pencere açmadan).
  - Yeni bir eylem ya da beklenti türü iki oynatıcıya birlikte eklenir.
- **Kullanım senaryosu** (isteğe bağlı): `./target/debug/kentos-cad kullan <ad>` ve `pnpm -C apps/web e2e:use <ad>`.

## 9. Bulut, Python ve MCP, gerekiyorsa

- **Sunucu:** `crates/server/application`, `apps/api`. Projenin ayarlarına ya da katmanlarına giren yeni alanın kuralları
  `project.changes`'te de denetlenir (`changes.rs`, `projects.rs`).
- **Veritabanı:**
  - Testler yalnız geçici veritabanında koşar (`KENTOS_E2E_DB=scratch pnpm e2e:cloud`).
  - `kentos` veritabanına ve 5432'deki `database-postgis-1` konteynerine dokunulmaz.
  - `kentos_cad`'e migration uygulanacaksa önce yedek alınır.
- **Python:** `python/kentos/<konu>.py`; yerel bağlar `crates/native/python/src/<konu>.rs` (`lib.rs`'e `pymodule_export`), tür
  imzaları `python/kentos/_native.pyi`, testler `python/tests/test_<konu>.py`. Takım: `pnpm py:test`.
- **MCP:** katalog komutu MCP'de kendiliğinden görünmez; `tools.rs`'e eklenir.

## 10. Performans ölçümü

- **Nerede:**
  - çekirdek `cargo test --release -p <crate> --test all <konu>::timing -- --ignored --nocapture`,
  - masaüstü `cargo test --release -p kentos-desktop perf::<konu> -- --ignored --nocapture --test-threads=1`.
- **Ne:**
  - büyük ve gerçekçi veri,
  - p50 ve p99,
  - bütçeyle karşılaştırma,
  - makine bilgisi (işlemci, GPU, tarih).
- **Sonuç ADR'ye yazılır:** Doğrulama'ya tablo olarak. Bütçeyi aşan süre gizlenmez; TODOS.md'ye ayrı bir madde olarak yazılır
  (örnek `REN-17`).
- **Ağır işin yolu:** önce profil çıkar (hangi pay ne kadar), sonra hızlı yol yaz (örnek: nokta başına dönüşüm yerine karo ağıyla
  taşıma, 13,6 ms → 4,6 ms).

## 11. Resimler ve görsel inceleme

- **Web:** `(cd apps/web && node scripts/e2e/shots.mjs <grup>)`; sahneler `shots.mjs`'te `SCENES.<grup>` altında tanımlanır.
  Resimler `apps/web/scripts/e2e/out/shots/<grup>/`'a düşer.
- **Masaüstü:** `KENTOS_SHOTS_ONLY=a,b cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` (ya da modülün kendi
  `screens` testi). Resimler `.run/shots/`'a düşer. GPU gerektiren sahnelerde `KENTOS_SNAPSHOT_BACKEND=wgpu` verilir.
- **Ölçüler:** her sahne 1440×900 ve 1100×650'de, açık ve koyu temada çekilir.
- **İnceleme:** her resme bütün olarak bakılır: kesik düğme, taşma, üst üste binen yazı, yanlış ikon, boş alan. Sorun düzeltilir ve
  yeniden çekilir.
- **Gönderme:** sahibe madde sürerken birkaç resim gönderilir. İkonlar resimlerde görünmelidir.
- **Dikkat:** `/tmp` bellek diskidir. Chrome profilleri ve büyük çıktılar `.run/` ya da geçici bir klasöre yazılır, iş bitince silinir.

## 12. Envanter ve belgeler

- **Envanter:** `pnpm inventory` `docs/inventory/web.json` ve `web.md`'yi yeniden yazar.
  - Farkı oku: yeni komutlar ve pencereler “masaüstünde” görünmeli.
  - Görünmüyorsa `equivalents.json` ya da `ported.json` eksiktir.
- **ADR'nin Uygulama'sı:** dosyalar ve modüller, katman katman (sözleşme, çekirdek, masaüstü, web, sunucu, otomasyon). Seçilen
  ikonlar adlarıyla, ayrılan ya da sonraya kalanlar nedenleriyle.
- **ADR'nin Doğrulama'sı:**
  - bağımsız başvurular ve kapsamları,
  - `.kcad` denetimleri,
  - testler ve sayıları,
  - gerçek servis ya da veriyle denenenler,
  - görsel incelemede bulunup düzeltilenler,
  - süreler tablosu,
  - ölçülmeyenler (açıkça).
- **TODOS.md:** satır `- [x] \`ID\` … — **G Ay, [ADR NNNN](docs/adr/NNNN-ad.md)** (kapsamın kaynağı): ne yapıldı, bağımsız
  başvurular, kapsam dışı.
- **CLAUDE.md:**
  - §0'a kısa bir kayıt;
  - §2'ye yeni denetim komutları (bağımsız başvuru, resimler, süreler);
  - §10.1 Devir notunda “Sıradaki …”.
  - Ayrıntı ADR'dedir, CLAUDE.md'de tekrarlanmaz.

## 13. Biçim, takım, commit, push, rapor

1. **Biçim:** yalnız bu maddede yazılan ya da değiştirilen Rust dosyaları biçimlenir.
   - `rustfmt --edition 2024 --check DOSYA` ile denetle.
   - `main.rs`, `lib.rs` ve `mod.rs` dosyalarına rustfmt'yi dosya yoluyla verme; alt modülleri de biçimler. Bunları standart girdiden
     geçir: `rustfmt --edition 2024 --emit stdout < DOSYA`.
   - Dosyanın HEAD'deki hâli biçimliyse bütün dosya biçimlenebilir; değilse yalnız kendi parçaların.
   - Ardından diff'te silinen `use` ve `mod` satırı olmadığını denetle:
     `git diff -U0 | grep -E "^-\s*(pub(\([a-z]+\))? )?(use|mod) "`.
2. **Ağır işler sırayla:** aynı anda tek ağır iş (derleme, test, ölçüm, e2e). Bellek sınırı makineyi donmaktan korur:

   ```sh
   nice -n 10 flock -w 3000 /tmp/kentos-heavy.lock \
     systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 timeout 1800 <komut>
   ```

   `make` ve `scripts/dev/svc.sh heavy <komut>` aynı kilidi kullanır. Uzun çıktılar `.run/logs/`'a yazılır.
3. **Testlerin sırası:**
   - Madde sürerken yalnız dokunulan testler koşar.
   - Push'tan önce tam takım bir kez koşar:

   ```sh
   pnpm typecheck
   pnpm test                    # Vitest
   pnpm rust:test               # cargo test + clippy -D warnings + bağımlılık yönü
   pnpm rust:test:desktop       # KentOS UI, çizim hattı, masaüstü: test + clippy
   pnpm py:test                 # Python SDK ve ortak komut durumları
   pnpm e2e:interaction         # ortak izler web'de (masaüstündekiler rust:test:desktop'ta)
   pnpm build
   pnpm inventory
   python3 scripts/fixtures/<dokunulan>_cases.py --check   # dokunulan her bağımsız başvuru
   python3 scripts/fixtures/kcad_v2_reference.py --check   # .kcad değiştiyse
   ```

   - Denetim komutu `&&` zincirinde `| tail`'e verilmez (çıkış kodu kaybolur): önce çıkış koduna bakılır, sonra commit'lenir.
   - Düşen test düzeltilir; atlanan ya da koşmayan denetim raporda söylenir.
4. **Commit:**
   - `git add` dosyaları tek tek yolla verir (her yolun var olduğunu denetle); `git add -A` ve `git stash` kullanılmaz.
   - Başlık: `feat(<alan>): <Türkçe başlık> — ADR NNNN (<ID> done)`.
   - Gövde İngilizce, kısa paragraflar.
   - Son satırlar:

   ```text
   Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
   Claude-Session: <oturumun adresi>
   ```

5. **Push:** `main`'de çalışan ana ajan
   `GIT_TERMINAL_PROMPT=0 git -c credential.helper= -c 'credential.helper=!gh auth git-credential' push -q origin main` ile gönderir.
   Paralel ajan kendi dalını gönderir ve PR açar (§14).
6. **Masaüstü ikilisi:** `cargo build -p kentos-desktop` (`target/debug/kentos-cad`). Sahip ikiliyi doğrudan çalıştırır; testler
   onu yeniden derlemez.
7. **Rapor:** sahibe Türkçe ve kısa.
   - Commit, ne yapıldı (kullanıcının gözüyle), seçilen ikonlar.
   - Test sayıları, bütçeyi aşanlar.
   - Denenemeyenler, yolda bulunan eski hatalar.
   - Sıradaki madde.
   - Rapordan sonra aynı turda sıradaki maddeye geçilir.

## 14. Paralel ajanlar

Birden çok ajan aynı makinede çalışırken:

- **Ayrı çalışma alanı:**
  - Her ajan kendi dalında ve kendi git worktree'sinde çalışır: `git worktree add ../kentos-cad-<id> -b <id>`.
  - `CARGO_TARGET_DIR` paylaşılmaz. Yol bağımlılıkları çakışır, derleme eski çıktıyı kullanır. Her worktree kendi `target/`'ını
    kullanır.
  - Disk dolmasın: `df -h /` denetlenir, biten worktree silinir.
- **Ağır işler makine boyunca sırayla:** bütün ajanlar `/tmp/kentos-heavy.lock` kilidini kullanır. Aynı anda iki derleme, iki
  tarayıcı e2e ya da iki ölçüm koşmaz.
- **Numaralar:**
  - **ADR numarası** işe başlarken ayrılır (sahip ya da ana ajan verir). Atlanan numara sorun değildir, çakışan numara sorundur.
  - **`.kcad` şeması ve `FORMATS_VERSION`** birleşme sırasıyla verilir. Okuyucu “şema ≥ N” diye karar verdiği için `main`'de 34
    varken 33 olamaz. Dalda geçici numarayla çalışılır; `main`'e ilk giren sıradaki numarayı alır, sonra gelen yeniden numaralar. O
    zaman şunlar güncellenir: `SCHEMA_WITH_*`, `SCHEMAS`, `broken/schema-version-*.kcad`, `expected.json`, Python okuyucusu ve yazıcısı,
    spesifikasyon, iki `FORMATS_VERSION`.
  - **Stilli çizim hattının sözleşme sürümü** (WGSL) de birleşme sırasıyla verilir.
- **Çakışan dosyalar:**
  - Elle birleştirilir:
    - `CLAUDE.md` (§0'ın uzun satırı, §2 ve §10.1),
    - `TODOS.md`,
    - `apps/desktop/equivalents.json`,
    - `app/ribbon.ts`, `app/menus.ts`, `ui/icons.ts`,
    - `tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`.
  - Birleştirildikten sonra yeniden üretilir, elle birleştirilmez:
    - `docs/inventory/web.{json,md}` (`pnpm inventory`),
    - `apps/web/src/contracts/generated/commandCatalog.json` ve TS türleri (`cargo test -p kentos-contracts`, `KENTOS_WRITE_CATALOG=1`),
    - `apps/desktop/ported.json` (`KENTOS_WRITE_PORTED=1 … ported`),
    - `fixtures/kcad/v2/expected.json` ve örnek dosyalar (`kcad_v2_reference.py`),
    - Python SDK'nın üretilen dosyaları (`scripts/python/sdk.py`),
    - `Cargo.lock`.
- **Birleştirme:**
  - Paralel ajan `main`'e push etmez; dalını gönderir ve PR açar:
    `git push -u origin <dal>` ve `gh pr create` (gövde sonunda “🤖 Generated with Claude Code”).
  - Birleştiren (sahip ya da ana ajan) önce şunlara bakar:
    - dalın güncel `main` üstünde olduğuna;
    - diff'te büyük ve ikili dosyalara, kılavuz ve yol haritası değişikliklerine.
  - Sonra dalın son commit'inde hızlı denetimleri koşar:
    - WASM'dan sonra `pnpm typecheck`,
    - `pnpm inventory`,
    - bağımsız başvurular,
    - masaüstü derlemesi.
  - Ardından `gh pr merge N --merge` ile birleştirir.
- **Ortak kaynaklar:**
  - Veritabanı yalnız geçici olandır.
  - Geliştirme sunucularının kapıları çakışmasın; e2e betikleri kendi sunucularını açar.
  - Sahibin kullanıcı verisi (`~/.config/kentos-cad`, `~/.local/share/kentos-cad`) silinmez; testler kendi geçici klasörlerini kullanır.

## 15. Kontrol listesi

- [ ] TODOS maddesi, ilgili ADR'ler ve önceki benzer commit okundu; numaralar ve adlar ayrıldı
- [ ] ADR: Bağlam, Karar (kapsam, veri modeli, kesin kurallar, arayüz, otomasyon, performans)
- [ ] Sözleşme ve kuralları; TS türleri ve katalog üretildi
- [ ] `.kcad` (gerekiyorsa): kodek, spesifikasyon, Python okuyucusu ve yazıcısı, örnek ve bozuk dosyalar, `FORMATS_VERSION`
- [ ] Çekirdek hesap ve testleri; bağımsız başvuru (`--check`) ve `fixtures/<konu>/v1`
- [ ] WASM bağlayıcısı ve TS sarmalayıcıları
- [ ] Ürün komutu (gerekiyorsa): iki işleyici, ortak durumlar, üç oynatıcı, SDK, MCP
- [ ] Masaüstü ve web arayüzü: araçlar, pencereler, şerit, menü, ikonlar, Öznitelikler, alt panel, İşlemler
- [ ] Ortak etkileşim izleri, iki oynatıcıda
- [ ] Bulut, Python, MCP (gerekiyorsa)
- [ ] Release süreleri, ADR'de bütçeyle
- [ ] Resimler: iki platform, 1440×900 ve 1100×650, açık ve koyu tema; incelendi, sahibe gönderildi
- [ ] Envanter, `equivalents.json`, `ported.json`
- [ ] ADR'nin Uygulama'sı ve Doğrulama'sı, TODOS.md `[x]`, CLAUDE.md §0, §2, §10.1
- [ ] Yalnız kendi dosyalarına rustfmt; düşen `use` ve `mod` satırı yok
- [ ] Tam takım geçti (çıkış kodları okundu)
- [ ] Commit (dosyalar tek tek), push ya da PR, masaüstü ikilisi, rapor
