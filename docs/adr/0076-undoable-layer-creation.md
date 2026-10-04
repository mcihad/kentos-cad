# ADR 0076: Katman ve grup eklemek geri alınabilir tek adım

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.8; ADR 0003 (işlem ve geri alma), 0020 (ortak belge durumları), 0048 (masaüstünde içe aktarma), 0072 (katman silme)
- **Kaynak:** web ajanının `bdaed77`'si. Ortak durumlar ve web tarafı aynı commit'tedir.
  - Durumlar: `fixtures/document-ops/v1/layer-add.json` (11 senaryo), `layers.json`'un değişen ekleme senaryosu.
  - Web kodu: `model/document.ts` (`addLayer`), `model/layers.ts` (`make`, `containerFor`), `io/apply.ts`, `processing/runner.ts`.
  - Tanım ve kararlar web ajanıyla birlikte yazıldı; kararlar sahibin genel yönüyle alındı (web ve masaüstü birebir).

## Bağlam

Katman oluşturmak iki platformda da geçmişin dışındaydı:

- Yeni katman ve Yeni grup Ctrl+Z ile geri alınmıyordu; geri alma onları atlayıp önceki adımı alıyordu.
- Bir içe aktarmanın geri alınması nesneleri alıyor, onlar için açılan katmanları boş bırakıyordu.

Task 12 (ADR 0072) belgeye geri alınabilir katman işlemleri getirmişti (`layerRemove` ve tersi). Eksik olan eklemeydi.

## Karar

### Belge

- **`Document::add_layer(new, parent, activate) -> Result<String, Refusal>`** (web'in `CadDocument.addLayer`'ı) tek adımdır: katman için “Katman ekle”, grup için “Grup ekle”. Açık bir işlemin ya da grubun içindeyse ona katılır, adı işlemin olur.
- **Yeri** eskisi gibidir:
  - `parent` bir grupsa onun sonuna;
  - bir katmansa onun grubunun sonuna;
  - değilse ağacın en üstüne, sona (bilinmeyen `parent` de).
  - Girdiği grup açılır. Açık olmak görünüm durumudur; geri alma grubu kapatmaz.
- **Kayıt:**
  - Adım `LayerAdd` işlemidir: düğüm, üst grubu ve yeri. Tersi task 12'nin `LayerRemove`'u.
  - `activate` ile yeni katman aynı adımda etkin olur ve adım `LayerActive { before, after }` işlemini de kaydeder.
  - `LayerActive` yalnız etkin katman hâlâ `before` iken uygulanır. Arada elle etkin yapılan katmana geri alma da yineleme de dokunmaz.
- **Geri alınan düğümle giden etkin katman:** yerine ağacın ilk katmanı etkin olur. Bu, yeni bir ağacın kuralıdır (`LayerTree::detach`).
- **Ret:** ağaçta olan kimlik, hiçbir şey değişmeden reddedilir: “‘a’ kimlikli katman zaten var; katman eklenmedi.” Önceden sessizce yeni bir kimlik veriliyordu.
- **Korumalar:**
  - Hâlâ nesne taşıyan düğüm geri almada yerinde kalır; aksi hâlde nesneler katmansız kalırdı. Yerelde olmaz: adımın kendi nesneleri önce gider.
  - Ağaçta olan kimlik iki kez eklenmez.
- **Dışarıdan gelen değişiklik** (`apply_external`): başka birinin nesnesi, bir adımın eklediği ya da sildiği katmana gelirse o adım düşer. Katman adımın kaydettiği düğümde de, düğümün bugünkü alt ağacında da aranır (sonradan eklenen gruba konan katman). O adımı geri almak katmanı başkasının nesnesinin altından çekerdi.

### Masaüstünün çağıranları

- Yeni katman: etkin yapılır.
- Yeni grup.
- Satır menüsünün “Yanına yeni katman” / “İçine yeni katman”ı: etkin yapılır ve artık Yeni katman gibi “‘X’ katmanı eklendi ve etkin yapıldı.” der.
- İçe aktarmalar (DXF, koordinat listesi, GeoJSON, Shapefile) yeni katmanlarını ve grubunu kendi işlemlerinin içinde açar. Tek Ctrl+Z nesneleri ve onlar için açılan katmanları birlikte alır, yineleme ikisini geri getirir.

### Bulut

Veritabanı projesinde ekleme bugünkü yoldan gider: ağaç `project.changes` ile. Geri alması bir katman silmedir: ağaç katmansız gider, task 12'nin sırası ve sunucu korumasıyla. Protokol değişmedi.

## Doğrulama

- `fixtures/document-ops/v1`, iki belgede: 77 durum, yeni 11 dahil.
- `crates/native/domain/tests/all/external.rs`: başka birinin nesnesi eklenen katmanın, sonradan gruba konan katmanın ve ilgisiz adımın durumu (web'in `layerAdd.test.ts`'i).
- `crates/native/domain/tests/document.rs`: sayaç ve ret.
- Masaüstü:
  - `layering::tests`: Yeni katman ve Yeni grup'un adımları, geri almada önceki etkin katman;
  - `exchange::tests` ve `exchange::apply::tests`: içe aktarmanın tek adımı katmanlarıyla birlikte.
- Geçenler: `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm e2e:interaction`, `pnpm inventory:check`, `KENTOS_E2E_DB=scratch pnpm e2e:cloud`.
