# ADR 8061: Eklenti SDK'sının kararlılık ilkesi — neyin güvencede olduğu, nasıl eskidiği, nasıl göç edildiği

- **Durum:** kabul edildi ve ilk dilimi uygulandı (16 Kasım 2026): ilke yazılı; eskiyen ve deneysel ad kaydı
  (`kentos_extensions::sdk_surface`, bugün boş), paketlemede ve kurulumda uyarıları, `kentos kek upgrade`.
- **Bağlam:** plan/eklenti.md D7 ve ADIM 21'in dersi ("yazılı SDK kararlılık politikası … ADIM 19'un başında"; "sürüm
  göçü: `verify`'da kalıcı kimlikli göç kuralları ve `kentos kek upgrade`"); araştırma notu
  `docs/research/2026-11-13-qgis-eklenti-sistemi.md` §5; ADR 8057 §1 (uyumluluk, `compat.rs`), §15 (çalışma kökeni),
  §71 (bileşen davranış tablosu), §72 (Python kuralları); ADR 8059 (katalog ve yayın kapıları).
- **Neden şimdi:** eklentiyi yazanların çoğu yapay zekâ ajanları olacak ve ekosistem büyüdükçe bir kırılmanın bedeli
  paket sayısıyla çarpılır. Örnek ekosistemde ana sürüm geçişinden 8 ay sonra paketlerin yalnız %42'si yeni sürümde
  (indirme ağırlıklı %68,4) [H2]; eksik üst sınırı "aynı ana sürümün sonu" sayan yükleyici ile `x.99` dolduran depo
  ayrı yorum yapınca uyumlu eklentiler kurulamaz oldu [L7]. Yazılı ilke, eklentiyi yazan insanın ve ajanın neye
  güvenebileceğini söyler; KentOS'un geliştiricisine de neyi bozamayacağını.

## §1. Güvencedeki yüzey

Kararlı olan, bir eklentinin dokunduğu her şeydir:

| Yüzey | Ne güvencede |
|---|---|
| `kentos.ext` (Python) | `__all__`'daki adlar; sınıfların genel yöntem ve alanları; **argüman adları, sırası, varsayılanları**, dönüş türü ve anlamı (`sdk/kentos_ext.pyi` bunun yazılı hâli, `python/kentos/ext.py`'den üretilir, sapma test kırmızısı) |
| `kentos.ui_panel` (Python) | bileşen işlevleri ve argümanları (panelin şemasından üretilir) |
| JSON biçimleri | `kentos.extension` (manifest), `kentos.processingTool`, `kentos.ui.panel`, `kentos.layerSchema`, denetim olguları, kuralın örnekleri (`kentos.auditExamples`, ADR 8057 §75): her biri `schemaVersion` 1; JSON Schema'ları SDK'nın yanında |
| Davranış | panel bileşenlerinin davranış tablosu (ADR 8057 §71), komutun `ctx.params` değerlerinin biçimi, doğrulama ve ret **kodları** (`python_runtime_install`, `manifest_utility_tab` …) |
| Ajan yüzeyi | `extension.*` araçlarının adları, argümanları ve sonuç şemaları; `kentos://sdk/ai-kek-pack` ve bölümlerinin adları |
| Komut satırı | `kentos kek` alt komutları, bayrakları ve çıkış kodları (0 tamam, 1 ret, 2 yanlış komut ya da okunamayan dosya) |

## §2. Kapsam dışı

`_` ile başlayan her ad; `kentos._native`, `kentos.ext_host`, `kentos.host` (KentOS'un kendi süreç kodu); deneysel
işaretli adlar (§5); ileti **metinleri** (kodları güvencede, cümleleri iyileşebilir); KentOS'un Rust crate'leri ve iç
iletileri (aracının JSON satırları SDK'nın kendi işidir).

## §3. Kırılma yalnız ana sürümde

- Güvencedeki yüzeyde ad kaldırmak, yeniden adlandırmak, argüman adını ya da sırasını değiştirmek ve anlamı değiştirmek
  yalnız **SDK protokolü artınca** olur (`compat::SDK_PROTOCOL`, bugün 1). Protokol artışı bir ADR ister.
- JSON biçimlerinde kırılma `schemaVersion` artışıdır. KentOS eski sürümü okumaya devam eder; bilmediği daha yeni
  sürümü "daha yeni bir KentOS gerekir" diye reddeder (manifest §3.2'nin kuralı, bütün biçimlerde aynı).
- Eklemek kırılma değildir: yeni ad, isteğe bağlı yeni alan, yeni bileşen türü, yeni ret kodu. Bilinmeyen alanın
  reddi (manifestin `deny_unknown_fields`'ı) bu yüzden yeni alanı eski KentOS'ta reddettirir; paket bunu
  `compatibility.kentos.min` ile söyler.
- **Uyumluluk aralığı:** üst sınır yazılmamışsa üst sınır yoktur (`compat.rs`); katalog bilinen bir kırılmada aralığı
  paketin dışından daraltır, kendiliğinden doldurmaz.

## §4. Eskime

Kaldırılacak bir ad önce eskir, en az **bir KentOS sürümü** eskimiş olarak durur ve ancak protokol artışında kalkar
(örnek ekosistemin ilkesi de böyle: istisnai kırılma bile en az bir ara sürüm önce uyarılır [Q2]). Bir eskime tek
commit'te dört yerde yazılır:

1. `kentos_extensions::sdk_surface::DEPRECATED`'e satır: ad, kimin olduğu, yerine geçen (düz yeniden adlandırmaysa),
   hangi KentOS sürümüyle eskidiği, ne yapılacağı;
2. `python/kentos/ext.py`'de adın `DeprecationWarning`'i (eklentinin günlüğüne düşer);
3. bu ADR'nin §10'una satır;
4. KentOS'un kendi kodu (Vitrin, örnek iskelet, MEBİS fikstürü, SDK örneği) adı kullanmaz: onların "uyarısız"
   testleri (`verify::tests::the_demo_extension_passes_every_gate`, SDK örneğinin doğrulaması) eskiyen adda kırmızıdır.
   Örnek ekosistemin "eskime uyarısı CI'da hatadır" kuralının karşılığı [Q2].

Paketin Python'u eskiyen adı kullanırsa `sdk_deprecated` uyarısı, satırıyla ve yerine ne yazılacağıyla; Kapı 1'dedir
(paketleme, kurulum, `verify`), yani kurulum incelemesinde kullanıcı da görür.

## §5. Deneysel yüzey

Bir ad deneysel çıkabilir (`sdk_surface::EXPERIMENTAL`): yeri ve biçimi oturmamış bir yetenek. Atölyede ve yerel
kurulumda çalışır; `sdk_experimental` uyarısı verir; **genel ve kiracı kataloğu onu kullanan paketi yayımlamaz**: paket
kapısının (ADR 8059, `GateKind::Package`) tarayıcısı bu kodu engelleyen bulgu sayar (tarayıcı işçisi ADIM 22A'nın
kalanıdır). Deneysel ad ya kararlılığa geçer (kayıttan çıkar) ya da bir protokol artışında kalkar.

## §6. Davranışın kararlılığı

`@ext.expr` ve `@ext.audit` sonuçları belgeye işler (etiket, süzgeç, denetim raporu). Eklentinin yazarı bu davranışı
değiştirince eklentinin sürümünü artırır; KentOS'un çalışma kökeni kaydı hangi sürüm ve özetle üretildiğini tutar (ADR
8057 §15). KentOS'un tarafında ifade motorunun, olguların ve panel davranış tablosunun anlamı §3'ün kuralına bağlıdır.

## §7. Göç

- **`kentos kek upgrade <klasör>`:** eskiyen adlardan yerine geçeni düz yeniden adlandırma olanları klasörün Python'unda
  yazar; belge dizgisine ve yoruma dokunmaz; değiştirdiği her satırı söyler. `--check` yazmaz, değişecek bir şey varsa
  1 ile çıkar (yazarın kendi denetimi için). Mekanik karşılığı olmayan eskime `verify` uyarısı olarak kalır ve ne
  yapılacağını söyler. Göç kuralının kalıcı kimliği eskiyen adın kendisidir.
- **Ajana geçiş notu:** `extension.sdk part=verifyCodes` kodların anlamını, rehberin Kurallar'ı ilkenin özetini verir.
- **Protokol artışında kurulu eklentiler:** listede kalır, uyumlu sürüm katalogdan önerilir, uyumsuz olan nedenli pasif
  kalır (örnek ekosistem kırık kodu taşımamak için eklenti klasörlerini hiç taşımıyor, kullanıcı neyin eksik olduğunu
  kendi buluyor [K21]; KentOS kırık kodu çalıştırmaz ama kullanıcıya neyi kaybettiğini gösterir).

## §8. Ölçüm

Her protokol artışında katalog "yeni protokole hazır eklenti oranı"nı iki biçimde izler: paket sayısıyla ve kurulum
ağırlıklı. İkisi de gerekli: örnekte %42 ile %68,4 arasındaki fark, en çok kullanılanların geçtiğini ama uzun kuyruğun
geride kaldığını söylüyor [H2]. (Ölçümün yeri ADIM 22A'nın katalog yüzeyi.)

## §9. Uygulama (16 Kasım 2026)

- `crates/shared/extensions/src/sdk_surface.rs`: `Deprecated`, `Experimental`, `DEPRECATED`, `EXPERIMENTAL` (ikisi de
  boş), Python'un kod satırlarında (belge dizgisi ve yorum hariç, `python_rules::code_lines`) noktadan sonra ya da
  `from kentos… import` ile kullanılan adı bulur; `upgrade`/`upgrade_with`.
- Kapı 1: `archive.rs` `check_tree` her `.py` dosyası için `sdk_surface::scan` (uyarılar).
- `apps/cli/src/upgrade.rs`: `kentos kek upgrade <klasör> [--check] [--json]`. Vitrin'de: "Yükseltilecek bir şey yok",
  çıkış 0.
- SDK: rehberin Kurallar'ında ilkenin iki satırı; `verifyCodes` bölümünde `sdk_deprecated`, `sdk_experimental`. Paket
  47.468 / 48.000 bayt.
- **Sınamalar:** `sdk_surface` 2 (sınamanın kendi kaydıyla: eskiyen ad yerine geçenle ya da notuyla satırında,
  deneysel ad yayımlanamaz diye; belge dizgisi, yorum ve `offset_rows` gibi içinde geçen ad kullanım değil;
  `upgrade` kodu yazar, belge dizgisini, yorumu ve yerine geçeni olmayanı olduğu gibi bırakır; SDK'nın boş kaydında
  uyarı ve yükseltme yok).
- **Kalan:** paket kapısı tarayıcısının `sdk_experimental`'ı engelleyen bulgu sayması (ADIM 22A), göç oranı ölçümü,
  JSON biçimlerinde `schemaVersion` 2 göçleri (`kek upgrade`'in biçim kuralları, ilk ikinci sürümle).

## §10. Eskiyen ve deneysel adlar

| Ad | Kimin | Durum | Yerine | KentOS | Not |
|---|---|---|---|---|---|
| — | — | — | — | — | Bugün eskiyen ya da deneysel ad yok. |

## Kaynaklar

- [Q2] QEP 425 — Stable API policy (4–16 Haziran 2026):
  https://github.com/qgis/QGIS-Enhancement-Proposals/blob/master/qep-425-stable-api.md
- [D9] Backwards Incompatible Changes (API break): https://api.qgis.org/api/api_break.html
- [L7] Plugins' Dual QGIS 3 and QGIS 4 Compatibility (24 Şubat 2026):
  https://lists.osgeo.org/pipermail/qgis-developer/2026-February/068089.html
- [K21] `src/app/qgsversionmigration.cpp`: https://github.com/qgis/QGIS/blob/master/src/app/qgsversionmigration.cpp
- [H2] 3.44 ve 4.0 eklenti dizinlerinin karşılaştırması (13 Kasım 2026): araştırma notunun §5'i.
