# ADR 0732: Mimari denetimin kalanı: kapsam, kalıcı istisna, yeni denetimler, önizlemeli toplu düzeltme

- **Durum:** kabul edildi (29 Ekim 2026); ARCH-DOC-03'ün kalan alt maddeleri.
- **Bağlam:** `plan/architecture.md` ARCH-DOC-03; ADR 0703 (`cad.arch.audit`, Denetim sekmesi), ADR 0589 §3 (topoloji
  istisnası ve özetle bağlanması), ADR 0717 (Kat ver), ADR 0723/0724 (profiller), ADR 0727 (mutfak dizisi), ADR 0343
  (merdiven ve rampanın kendi uyarıları), ADR 0679 (mahal ayırıcı), ADR 0253 (`cad.changeset.apply`).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0703'ün ilk dilimi bütün çizimi denetliyor, her bulguyu her denetimde yeniden gösteriyor ve tek tek düzeltiyordu.
Açık kalanlar: bulguyu bilerek bırakma (istisna), denetimi bir kata ya da seçili nesnelere daraltma, merdiven ve
rampanın kendi uyarılarının denetimde görünmemesi, çok kısa artık duvar parçası ve üst üste çizilmiş duvarlar, düzeltmenin
yazmadan önce görülmesi ve birden çok düzeltmenin tek adımda yapılması.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit LT 2025 — Warnings for Selected Elements](https://help.autodesk.com/view/RVTLT/2025/ENU/?guid=GUID-42294460-2388-44D4-B68E-74943FC5179B):
  seçili nesnenin uyarıları ağaç olarak listelenir (Show Related Warnings); seçimle daraltılmış uyarı listesi.
- [Revit — Troubleshooting: Rooms](https://help.autodesk.com/cloudhelp/2019/ENU/Revit-Troubleshooting/files/GUID-E11AD695-DB31-4C1C-9702-E0EF611D838C.htm):
  “Highlighted Room Separation Lines Overlap” üst üste binen ayırıcıyı, “Room Is Not Enclosed” kapanmayan odayı söyler.
- [Navisworks — Results Tab](https://help.autodesk.com/cloudhelp/2022/ENU/Navisworks/files/GUID-FCC9E5E1-2717-48D2-8DBE-2055CF2DC61E.htm):
  çakışmanın durumu Yeni/Etkin/İncelendi/Onaylandı/Çözüldü; onaylanan çakışma test yinelenince aynı çakışma bulunursa
  onaylı kalır, onaylayanın adı ve zamanı tutulur; bulunmayan çakışma Çözüldü olur.

KentOS kararı: Revit'in seçimle daraltması ve Navisworks'ün onayı alınır; ama onay (istisna) nesnelerin o anki hâline
bağlanır (ADR 0589 §3'ün kuralı): nesneler değişince istisna düşer, bulgu yeniden açılır ve söylenir. Bulgu silinmez,
saklanmaz; istisna proje dosyasında açıkça durur, notuyla.

## Karar

### 1. Bulgunun kalıcı anahtarı

Her bulgu bir `key` taşır: `kod:kimlikler` (kimlikler bulgunun sırasıyla, virgülle), uç boşluğunda ucun adı
(`:start`, `:end`); aynı denetimde aynı anahtar ikinci kez çıkarsa sırasıyla `#2`, `#3` eklenir. Anahtar ilgisiz
düzenlemelerde değişmez (kimlikler kalıcıdır).

### 2. Kapsam

`cad.arch.audit`'in `within`'i: `levels` (kat kimlikleri; `null` katsız nesneler) ve `uids` (nesneler). Bulgu, nesnelerinden
biri verilen katlardan birindeyse (nesnenin katı ADR 0715'in kuralıyla: açıklığın duvarının, etiketin nesnesinin, ölçünün
bağlarının) ve nesnelerinden biri verilen nesnelerdense kapsamdadır (Revit'in “Show Related Warnings”i). İkisi birlikte:
ikisi de tutmalı. Kapsam dışındaki bulgular cevabın `outside`'ında sayılır. Retler: `level_not_found` (`within.levels[i]`),
`entity_not_found` (`within.uids[i]`). Masaüstünde **Kapsam ▾**: Bütün çizim / Etkin kat / Seçili nesneler.

### 3. Kalıcı istisna

Proje ayarının `arch.auditExceptions`'ı (KCAD şema 130, opt-in): `{key, code, uids, digest, note}`. `digest` bulgunun
kendisinin özetidir: `{at, code, message, uids}`'in kanonik JSON'unun (ADR 0253'ün sayı yazımıyla) SHA-256'sının ilk sekiz
baytı, onaltılık. Nesnelerin kayıtlı hâline bağlanmaz: bulguyla ilgisi olmayan bir değişiklik (rengi, katmanı) istisnayı
düşürmez; bulgunun ölçüsü, iletisi ya da yeri değişince (boşluk büyür, mahal küçülür, duvar kayar) düşer. Denetim:

- İstisnası olan ve özeti tutan bulgu `exception: true`; `exceptions: hide` (varsayılan) onu listelemez ve `excepted`'te
  sayar, `show` listeler, `only` yalnız onları listeler.
- Özeti tutmayan (bulgusu değişmiş) bulgu açık listelenir, `lapsed: true`, uyarı `exception_lapsed`.
- Bulgusu artık bulunmayan istisnaların anahtarları `stale`'de.

Yazan komut `cad.arch.exceptions` v1: `mark` (anahtarlar, isteğe bağlı `note`, isteğe bağlı `audit`: bulguları bulan
denetim girdisi), `unmark` (istisnaların anahtarları), `prune` (bulgusu artık olmayanları siler). Tek geri alma adımı
(“İstisna”, “İstisnayı kaldır”, “İstisnaları temizle”); topoloji istisnasından (ADR 0589, geri alma adımı değil) farklı olarak
yanlışlıkla verilen istisna Ctrl+Z ile döner. Retler: `invalid_input` (anahtar yok ya da yineleniyor), `finding_not_found`
(`keys[i]`), denetim girdisinin retleri `audit.` yoluyla.

### 4. Yeni denetimler

| Grup | Kod | Ne zaman | Düzeltme |
|---|---|---|---|
| Duvarlar | `arch_wall_short` | Açık duvarın yolu `minLength`'ten (varsayılan 0,05 m) kısa; artık parça | açıklığı yoksa sil (`cad.entities.delete`) |
| | `arch_wall_overlap` | Aynı kattaki iki düz duvarın yolları paralel (1°), yanlamasına aralık yarı kalınlıklar toplamından az ve boyuna örtüşme ikisinin kalınlığından uzun | biri öbürünün aynısıysa (yol, kalınlık, hiza, öteleme) sonrakini sil |
| | `arch_separator_overlap` | Aynı kattaki iki mahal ayırıcının düz kenarları aynı doğruda 1 mm'den uzun örtüşüyor (Revit'in ayırıcı uyarısı) | yok |
| Etiketler | `tag_empty` | Etiketin yazısı boş: nesnesinin şablondaki değerleri yok (ADR 0344 eki) | yok |
| Merdivenler | `stair_short`, `stair_long`, `stair_rise_level` | Merdivenin kendi uyarıları (ADR 0343, 0713): yol rıhtlara yetmiyor, yol uzun, rıhtlar katın yüksekliğini tutmuyor | `stair_rise_level`: yüksekliği kattan al (`cad.arch.edit` `heightFromLevel`) |
| Rampalar (yeni) | `ramp_no_run`, `ramp_slope_mismatch` | Rampanın kendi uyarıları (ADR 0343) | `ramp_slope_mismatch`: eğimi hesaplanana eşitle (`cad.arch.edit` `slope`) |
| Mutfak dizileri (yeni) | `arch_casework_broken` | Dizinin duvarı yok ya da dizi kurulamıyor (ADR 0727 `broken`) | diziyi bırak (`cad.casework.edit` `release`) |

Blondel oranı (`stair_step_rule`) bilgi olduğundan bulgu değildir; 2a+b profil seçilince denetlenir (ADR 0724).
Yaylı duvar ve yaylı ayırıcı örtüşmesi bu dilimde yok.

### 5. Önizleme ve toplu düzeltme

- Düzeltmesi olan bulgu seçilince masaüstü düzeltmeyi çizimin okuma kopyasında çalıştırır ve değişecek nesnelerin yeni
  çevresini kesikli vurgu renginde, silinecekleri tehlike renginde çizer; çizim değişmez. Python ve MCP aynı önizlemeyi
  `cad.changeset.apply`'ın planıyla alır.
- Satırlar Ctrl/⌘ ile tek tek, Shift ile aralık olarak seçilir. **Düzelt (n)** seçili bulguların düzeltmelerini tek
  `cad.changeset.apply` ile, tek geri alma adımı “Denetim düzeltmeleri” olarak yazar; biri yazılamazsa hiçbiri yazılmaz ve
  hangisi olduğu söylenir.
- Satırın sağ tık menüsü: Seç ve göster, Düzelt (satırın ya da seçililerin), İstisna yap… (not sorulur), İstisnayı
  kaldır; istisnalı satır soluk ve “İstisna ·” önekli (notu sonda), düşen istisnanın satırı uyarı renginde “İstisna
  düştü ·” önekli.
- Ctrl/⌘ ve Shift ile seçilen bulguların bütün nesneleri seçilir ve görünüm hepsine sığdırılır (Revit'in birden çok
  uyarıyı göstermesi gibi).
- Çubuk tek satırdır, sığmazsa sayı ve düğmeler ikinci satıra iner: Denetle, Grup ▾, **Kapsam ▾**, **İstisnalar (n) ▾**
  (Gizle/Göster/Yalnız istisnalar ve Eski istisnaları temizle; bulgusu kalmamış istisna varken uyarı işareti),
  **Eşikler ▾** (uç boşluğu, kısa duvar, küçük ve dar mahal, rampa ve merdiven profili; her biri alt menü, seçili değeri
  yanında, hepsi ipucunda); sonra sayı (“n bulgu · m düzeltilebilir · k kapsam dışında”), Kata ver ▾, İstisna yap… ya da
  İstisnayı kaldır, Düzelt. ADR 0703'ün çubukta tek tek duran eşikleri 1100 px'te taşıyordu; değerleri artık menüde ve
  ipucunda görünür.
- Proje ayarları › Mimari kaydı istisnaları korur (formun değil, Denetim'in); `cad.settings.set`'in `arch`'ı ise bütün
  mimari ayarı, istisnalar dahil, yerine koyar (evre ve etkin kat gibi).

### 6. Kapsam dışı

Görünüşe göre istisna, kullanıcı adı ve zaman damgası (KentOS'ta oturum kimliği yok; not serbest metindir), yaylı
örtüşmeler, 3B çakışma.

## Doğrulama

- **Ortak durumlar** bağımsız başvurudan (`python3 scripts/fixtures/arch_audit_cases.py --check`, KentOS kodu olmadan):
  `cad.arch.audit` 80 (kapsamın 10, istisna görünümlerinin 5, yeni denetimlerin 15 durumu dahil), `cad.arch.exceptions` 25
  (yap, iki bulgu, değişmeyen, not, düşen istisnanın yeniden verilmesi, kaldır, temizle, denetimin girdisiyle, plan, retler:
  anahtarsız, yinelenen, uzun ve denetim karakterli not, olmayan bulgu ve istisna, `audit.` yoluyla denetimin retleri,
  sürüm çakışması), `cad.arch.audit.accessibility` 24 ve `cad.arch.audit.stairs` 33 yeni anahtarlarla yeniden yazıldı.
  Rust tarafı `KENTOS_CASES_ONLY=cad.arch cargo test -p kentos-native-application --test all every_command_case`; yetenek
  testi iki komutun her ret ve uyarı kodunu bir durumda görür (`invalid_revision` ve `not_finite` için `expectedRevision`
  ve `audit.maxGap` durumları eklendi).
- **KCAD şema 130** (opt-in): örnek `fixtures/kcad/v2/audit-exceptions.kcad` bağımsız Python yazıcısıyla; altı bozuk örnek
  (kod, özet, şema 129'da, nesnesiz, yinelenen, kimlik) ve n+1 sürüm `broken/schema-version-131.kcad`;
  `python3 scripts/fixtures/kcad_v2_reference.py --check` (1332 dosya), `cargo test -p kentos-kcad --test all`.
- **Masaüstü:** `rows_chosen_together_are_previewed_and_fixed_in_one_step` (üç satır, önizlemede iki kesikli çevre ve bir
  silinen, tek adım, Geri al hepsini döndürür), `an_exception_hides_its_finding_until_the_finding_changes` (not sorusu,
  gizli ve gösterilen, duvar 2 cm kayınca düşen istisna uyarı renginde), `kapsam_keeps_to_the_level_and_the_selection` ve
  ADR 0717'nin `kata_ver_gives_every_object_without_a_level_one`'ı.
- **MCP:** `cargo test -p kentos-mcp --test arch_audit_exceptions`: gerçek sunucuda istisna, gizlenme ve sayılma, kayıt ve
  yeniden açma, kaldırma. **Python:** `pnpm py:test` (375 test; ortak durumlar tipli katmandan). İki şey bulundu ve
  düzeldi: Python koşucusu metnin içindeki `$uidOf:N`'yi (bulgunun anahtarı) çözmüyordu, Rust'taki gibi çözer; yeni
  denetimlerin kurulumundaki boş etiketin yazısı açıkça `""` yazılmıştı, KCAD yazıcısının biçimi onu yazmaz (Python
  kurulumu `.kcad` olarak açar, Rust JSON'dan açtığı için görmüyordu).
- Masaüstünün Denetim modülü 1453 satıra çıkmıştı; durumu ve olayları `arch_audit.rs`'te, yazmaları `arch_audit/write.rs`,
  önizlemesi `arch_audit/preview.rs`, çizimi `arch_audit/view.rs`'te. Clippy sekiz değişen crate'te ve kentos-python'da
  temiz, ortak crate'ler wasm32'ye derlenir.
- **Gerçek arayüz** (`KENTOS_SHOTS_ONLY=denetim-ekler,denetim-toplu-onizleme,denetim-toplu-duzeltildi,denetim-istisna-sor,denetim-istisna-gizli,denetim-istisna-goster,denetim-istisna-dustu,denetim-kapsam,denetim-satir-menusu`
  ve ADR 0703/0717'nin dokuz sahnesi; 1440×900 ve 1100×650, koyu ve açık): çubuk 1100 px'te tek satıra sığar, 1440'ta da;
  toplu önizlemede silinecek duvar tehlike renginde, değişecek duvar kesikli; istisna satırı soluk, notu sonda; düşen
  istisnanın satırı uyarı renginde; kapsamda “1 bulgu · 1 düzeltilebilir · 27 kapsam dışında”. Ham resimler
  `.run/gui-evidence/2026-10-29-arch-audit-exceptions/`'da, depoya girmez.
