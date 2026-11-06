# ADR 1019: `cad.standards.install` verilen blok kitaplıklarını da okur

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` (ADR 1017 §3'ün sınırı: “Python ve
  MCP'nin `cad.standards.install`'ı hâlâ yalnız yerleşik kitaplıkları okur”).
- **Bağlam belgesi:** ADR 1100 §2 (`Libraries`, yayın başvurusu), ADR 1017 (kurulum penceresinde proje kitaplığı).

## 1. Karar

- `StandardsInstall` girdisi isteğe bağlı `libraries` kazanır: projenin ya da ofisin `kentos.blocks` dosyalarının
  mutlak, `.kblok` uzantılı yolları (en çok 16 tane, her biri en çok 16 MiB). Verilen sırayla, KentOS'un
  kitaplıklarından **önce** aranır; satırın `library`'si dosyanın adıdır (uzantısız). Kitaplık her çağrıda okunur,
  çizime yazılmaz. Komut sürümü 1 kalır: alan yoksa davranış öncekiyle aynıdır.
- Retler, öbürlerinden önce ve ilk sorunlu yolda (`libraries[i]`): `invalid_file_path` (göreli ya da `.kblok`
  değil), `file_not_found`, `block_library_unreadable` (yeni; `kentos.blocks` değil, blokları okunmuyor ya da
  16 MiB'tan büyük).
- Masaüstü kurulum penceresi eklenen kitaplıkların yollarını aynı girdiye koyar (son eklenen önce) ve düz
  `plan`/`execute`'ü çağırır: Komut günlüğündeki çağrı kitaplığıyla birlikte yinelenir. Python ve MCP aynı alanı
  kullanır (`python/kentos/cad/standards.py` yeniden üretildi).

## 2. Kanıt

`scripts/fixtures/standards_install_library_cases.py` (KentOS kodu yok) `fixtures/commands/v1/standards-libraries/`
altına üç dosya yazar: `proje` (BILINMEYEN'in yayını ve kentos-ofis'inkinden bir çizgi fazla KAPI 2. yayını),
`ikinci` (başka bir BILINMEYEN), `bozuk` (`kentos.document`). On yeni vaka: proje kitaplığıyla eksik BILINMEYEN'in
eklenmesi; KAPI'nın KentOS yerine projeden bulunması; iki kitaplığın iki sırası; projedeki KAPI'yla yerinde
değiştirme; doğrulamanın yazmaması; dört ret. İlk koşu gerçek bir şey yakaladı: başvurunun yazdığı kartta `version`
yoktu ve okuyucu kitaplığı reddetti (`missing field version`); ret doğruydu, başvuru düzeltildi.
Masaüstü: `standard_install::tests::a_project_library_added_in_the_window_is_searched_for_the_blocks`.

## 3. Sınırlar

Bulut kitaplığı doğrudan okunmaz (indirilen `.kblok` verilebilir); pafta şablonu kitaplığı girdiden verilemez.
16 MiB sınırı vaka ile değil kodla tutulur.
