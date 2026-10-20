# ADR 0538: Bağlı katmanın kaynakla uzlaştırılması: taban ve açık çatışma çözümü

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-37'nin ilk dilimi: tek revizyon üzerinde iyimser düzenleme ve
  açık uzlaştırma. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0503 (bağlı katman), 0505 (PostGIS), 0510 (alan sınırlı çalışma kopyası), 0531 (komşular),
  0532 (birlikte yazma), 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - aynı nesnede alan alan birleştirme: bugün iki yanın değiştirdiği nesne, değişiklikleri ayrı alanlarda olsa da çatışmadır;
    tabanda alan başına özet gerekir;
  - adlı çalışma sürümü ve review → publish akışı (ihtiyaç kanıtlanırsa);
  - ilişkili değişikliklerin birlikte denetlenmesi: ortak kenarı, N:M bağı ya da üst ve alt kaydı değiştiren iki düzenleme;
    bugün her nesne kendi başına çatışır;
  - bulut projesinin `project.changes` çatışmasının aynı farkla gösterilmesi, sunucu işinin commit'inde sürüm denetimi;
  - `with` ile birden çok katmanın birlikte uzlaştırılması.

## Bağlam

Bağlı katman (ADR 0503), tablonun çalışma kopyasıdır. Bağ dosyanın son eşleşmedeki SHA-256'sını ve katmanın özetini tutar.
Böylece iki yanın değiştiği ayrılır:
- Dosya değiştiyse yazma reddedilir (`source_changed`).
- Katman değiştiyse okuma reddedilir (`pending_changes`), `discard` ile katmandaki iş gider.

Ama iki yan birden değiştiğinde (QGIS kullanıcısı tabloyu, KentOS kullanıcısı katmanı düzenledi) yapılacak bir şey kalmıyordu:
- yazmak başkasının işini ezer;
- okumak kendi işini götürür.

Bağ kopya tutmadığı için kimin neyi değiştirdiği nesne düzeyinde bilinmiyordu. Plan son yazanın kazanmasını yasaklar; anlamlı
farkı ve açık uzlaştırmayı ister.

## Karar

1. **Taban.** Projenin `sourceBases`'i (`.kcad` şema 91) her bağ için son eşleşmede her nesnenin iki özetini tutar.
   - Katmanın özeti: nesnenin türünün, geometrisinin ve özniteliklerinin kanonik JSON'unun SHA-256'sının ilk 8 baytı
     (`layer_digest`'in özetlediği öz).
   - Tablonun özeti: satırın okunduğu nesneninki. Yazılan satır nesneye bit bit geri okunmadığında (sistem değişimi) farklı olur.
     Tabloda olmayan nesneninki 0'dır.
   - Satırlar kalıcı kimliklerin bayt sırasıyla, 32'şer bayt, base64.
   - Taban, alındığı `revision` ve `digest`'e aittir. Bağ başka sürümdeyse taban bayattır ve kullanılmaz.
   - Bağlama, okuma ve yazma tabanı yeniden alır. Yazmadan sonra tablo yeniden okunur: taban, tablonun tuttuğu biçimdir.
   - Aynı dosyanın başka bir tablosunun yazılması, dokunulmayan bağın tabanını yeni sürüme taşır.
   - 200 000'den çok nesneli katmanın tabanı yoktur (`MAX_BASE_ROWS`).
   - Taban bağın JSON'unda değildir: komutların cevapları ve bağı söyleyen yüzeyler değişmez.
2. **`cad.sources.reconcile`.** Her nesne için tabana göre iki yanın ne yaptığını bulur: hiçbir şey, ekleme, değiştirme ya
   da silme.
   - Yalnız katman değiştirdiyse nesne tabloya yazılır; yalnız tablo değiştirdiyse katmana alınır.
   - İkisi de sildiyse ya da aynı tür, geometri ve özniteliklere getirdiyse uyuşur (`agreed`).
   - Öbür hâllerde çatışmadır: `changed`, `removedInLayer`, `removedInTable`. Farklı öznitelikler adlarıyla ve iki yanın
     değerleriyle, geometrinin farklı olup olmadığı söylenir.
   - Çatışmanın yanı `take` (nesne nesne) ve `rest` (kalanlar) ile seçilir: `layer` ya da `table`.
   - Plan hepsini söyler, hiçbir şey yazmaz. Çözülmemiş çatışma orada tablonun hâlinde sayılır.
   - Yürütme bütün çatışmalar çözülmeden yapılmaz (`unresolved_conflicts`, conflict); geçerleme de aynı reddi verir.
   - Yürütmede önce tablonun yanı katmana alınır. Sonra katman, `cad.sources.sync` write'ın yoluyla, dosyanın şimdiki hâline
     yazılır. İkisi tek geri alma adımı (“Uzlaştır: tablo”) ve tek işlemdir: dosyanın reddi katmandakini de geri alır.
   - Retler: `no_source_base`, `not_a_conflict` (yalnız çatışmalar için yan seçilir), `invalid_input` (aynı nesneye iki
     kez yan). Ayrıca bağın retleri: `source_not_found`, `layer_not_found`, `layer_locked`, `source_gone`,
     `import_unreadable`, `table_not_found`, `crs_mismatch`, `source_readonly`, `derived_dataset`, `geometry_mismatch`.
   - Uyarı: yazmanınki (`export_losses`).
3. **Masaüstü.** Kaynak ▸ Uzlaştır… penceresi açılır:
   - iki yönün sayıları ve uyuşanlar;
   - çatışmaların listesi: nesne, türü, farklı alanlar iki yanın değerleriyle, her satırda Benimki / Tablodaki;
   - Hepsi benimki, Hepsi tablodaki ve Uzlaştır;
   - satıra tıklamak nesneyi çizimde gösterir.

   Yazma `source_changed` ile reddedilince bağlı katmanın kartı Uzlaştır…'ı önerir.
4. **Python ve MCP** komutu katalogdan alır; uzlaştırma var olanı değiştirdiği için MCP'de yıkıcı araçtır.
5. **Ortak durumların koşucusu**na `copyFile` adımı eklenir: bir fixture'ın baytlarını bir `$tmp:` dosyasının üstüne yazar
   (iki adım arasında QGIS'te değişen tablo).

## Doğrulama

- `python3 scripts/fixtures/reconcile_cases.py --check`: `cad.sources.reconcile`'ın 31 durumu, KentOS kodu olmadan.
  - Dosyaları GDAL'ın OGR'ı yazar (`--files`): `uzlas.gpkg` on parsel; `uzlas-qgis.gpkg` aynı dosyanın bir QGIS kullanıcısının
    düzenlediği hâli (öznitelik, taşınan geometri, silinen ve eklenen satırlar).
  - Satırlar `gpkg_import_cases.py`'nin okuyucusuyla okunur; kural bu ADR'den yazılmıştır.
  - Kapsadıkları: dört tür çatışma, uyuşan değişiklik, yalnız tablo ya da katman, ikisi de silen, `take` ve `rest`, iki yönde
    yürütme ve ardından sıfır fark (taban yeniden alınır), geri alma, on dört ret ve uyarı.
- `KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all reconcile` geçici kopyada uzlaştırır ve dosyayı
  `kentos-uzlasti.gpkg` olarak yazar. `reconcile_cases.py --written` onu OGR'la satır satır denetler.
- `.kcad` şema 91: `fixtures/kcad/v2/source-bases.kcad` bağımsız Python yazıcısıyla, üç bozuk örnek;
  `kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Resimler: `cargo test -p kentos-desktop exchange::gpkg_tests::reconcile_screens -- --ignored
  --nocapture` (`.run/shots/gpkg-uzlastir-*`).
