# ADR 1136: Bağımlılıklar en son sürümlerine (Kasım 2026)

Durum: kabul edildi · Tarih: 20 Kasım 2026

## Bağlam

Çalışma alanının `[workspace.dependencies]` bölümü her crate'i `=` ile sabitler (CLAUDE.md §3: sürüm değişikliği
gerekçe ister). Kullanıcı bütün crate'lerin en son sürüme alınmasını, ardından release derlemesi istedi.
crates.io'daki en son sürümler 20 Kasım 2026'da tek tek okundu; `cargo update` kilit dosyasını yeniledi.

## Karar

### Yükseltilenler

| Crate | Eski | Yeni | Koddaki karşılığı |
|---|---|---|---|
| serde_json | 1.0.151 | 1.0.154 | — |
| laz | 0.13.0 | 0.14.0 | — |
| sha1 / sha2 | 0.10.7 / 0.10.9 | 0.11.0 / 0.11.1 | Özet artık `LowerHex` değil: `{:x}` yerine bayt bayt `{b:02x}` (`sheets_set.rs`, metin şekillendirme testi) |
| schemars | 1.2.2 | 1.2.3 | — |
| wasm-bindgen | 0.2.128 | 0.2.130 | Web duruyor (ADR 0228); wasm32 denetimi temiz, `wasm-bindgen-cli` web'e dönüşte aynı sürüme alınır |
| tokio | 1.53.1 | 1.54.1 | — |
| sqlx | 0.9.0 | 0.9.1 | — |
| uuid | 1.26.1 | 1.29.3 | — |
| libc | 0.2.189 | 0.2.191 | — |
| rusqlite | 0.39.0 | 0.43.0 | — |
| jsonwebtoken | 11.1.0 | 12.0.0 | `EncodingKey::from_secret` artık `Result` (OIDC testi) |
| bytes | 1.12.1 | 1.12.2 | — |
| glam | 0.30.10 | 0.34.3 | Kamera işlevleri `glam::camera` altında: `rh::proj::directx::perspective` (eski `perspective_rh`: Z [0, 1], Y yukarı) ve `rh::view::look_at_mat4` (yön küpü) |
| rfd | 0.17.2 | 0.18.0 | — |
| pyo3 | 0.29.2 | 0.30.1 | — |
| roxmltree | 0.20.0 | 0.21.2 | `ParsingOptions` yeni `entity_resolver` alanı: `None` (servis yetenekleri, eklenti simgesi) |
| base64 | 0.22.1 | 0.23.1 | — |
| ed25519-dalek | 2.2.0 | 3.0.0 | — |
| p256 / p384 | 0.13.x | 0.14.0 | — |
| rsa | 0.9.10 | 0.10.0 | `RsaPublicKey`/`RsaPrivateKey` → `PublicKey`/`PrivateKey`; `encoding` ve `pkcs1v15` özellikleri açıkça istenir (0.10'da varsayılanın dışında) |
| getrandom | 0.3.4 | 0.4.3 | — |
| tokio-tungstenite | 0.29.0 | 0.30.0 | — |
| pollster | 0.4.0 | 1.0.3 | — |
| objc2 (masaüstü) | 0.6.4 | 0.6.5 | — |
| parking_lot (masaüstü) | 0.12.5 | 0.13.0 | — |
| unicode-script (geometri çekirdeği) | 0.5.8 | 0.5.9 | — |

### Bilerek tutulanlar

Arayüz çatısının en son sürümü (iced 0.14) wgpu 27, resvg 0.45 ve tiny-skia 0.11'e bağlıdır.

- **wgpu 27.0.1, naga 27.0.3:** KentOS'un sahne ve çizim çizicileri çatının wgpu türlerini (aygıt, kuyruk, doku)
  doğrudan paylaşır; iki wgpu sürümü aynı süreçte birlikte kullanılamaz. wgpu 30'a geçiş çatının yeni sürümünü bekler.
- **resvg 0.45.1, tiny-skia 0.11.4:** arayüzdeki SVG simgeleri ile pafta/plot çıktısı aynı çiziciyle çizilsin diye
  çatının sürümünde kalır.

Bu sürümler çatı yükseldiğinde birlikte yükseltilir.

## Doğrulama (20 Kasım 2026, bu Mac)

- `cargo check --workspace --all-targets` temiz (yalnız önceden var olan uyarılar); dokuz wasm crate'i
  `wasm32-unknown-unknown` için temiz.
- İmza crate'leri, GeoPackage, nokta bulutu, servisler, eklentiler, geometri çekirdeği, dosya biçimleri, stil
  testleri geçti. `kentosd` birim testleri 147/147.
- Ortak komut tekrarı (22 200 durum): bilinen 41 hata, aynı dağılım (elevations.edit 18, ifc.inspect 9,
  program.check 7, underlay.edit 6, schedules.query.units 1); yükseltmeden yeni hata yok.
- Masaüstü çapraz süzgeçleri (ribbon_tests, module_tests, history::text, catalog, ifc_viewer) 100/100.
- Python SDK maturin ile pyo3 0.30'la derlendi; 867 testten 857'si geçti. Kalan 10'u pyo3'le ilgili değil,
  veri farkıdır: haritalama işinin (`mapWork`) bağlı dosyası kaydederken `relative` alanını kazanır, gidiş-dönüş
  testi bunu yalnız `sources` için düşer (7 örnek); tipli katmandan geçen ortak durumlarda BIM komutlarının
  sonuçları (cad.arch.performance, cad.elementtypes.edit, cad.delivery.edit, cad.zoning.parcel…: 335 durum);
  profil içe aktarma ve panel yazımı. Bunlar ayrı iştir.
- Release derlemesi `cargo build --workspace --release` uyarısız (ilk koşu 23 dk 34 sn, son koşu 5 dk 52 sn).

### Derleme uyarıları

Kullanıcının isteğiyle derleme uyarısız bırakıldı:

- Masaüstünde yalnız testlerin kullandığı sayaçlar (`Building3d::bodies`, `with_body`, `count`) `#[cfg(test)]`
  altına alındı; hiçbir düğmenin göndermediği `scene3d_window::Event::Rooms` kaldırıldı, testler görüntüleyicinin
  kendi `Spaces` anahtarını gönderir.
- `kentos-jobrun`: `Landlock` yalnız Linux'ta (ve testlerde) dışa açılır.
- Başsız testlerde gereksiz `mut` kaldırıldı.
- macOS bağlayıcısının notu “`__eh_frame` 16 MB'ı aşıyor”: masaüstü ikilisinde 253 108 FDE (20,9 MB) var, yaklaşık
  205 000 işlev derleyicinin DWARF kipinde yazılıyor ve sıkıştırılmış tablo 16 MB'tan öteye işaret edemez.
  `-no_keep_dwarf_unwind` artık geçersiz, `-no_compact_unwind` tabloyu bütünüyle kaldırıp durumu kötüleştirir.
  Geri sarma çalışır, yalnız aranması yavaştır. Bu yüzden yalnız bu not susturuldu: büyük programları ve test
  programları bağlanan 11 paketin `Cargo.toml`'unda `[lints.rust] linker_messages = "allow"` var (commands,
  extension-tools, interaction, native-style, plot, processing, headless, mcp, native-application, python, desktop).
  Not, bu paketlerin 165 MCP test programında, başsız ve uygulama testlerinde de çıkıyordu. Çalışma alanı düzeyindeki
  `[workspace.lints]` seçilmedi: Cargo 1.100, onu almayan her üye için `missing_lints_inheritance` uyarısı veriyor
  (83 paket). Kök özniteliği de seçilmedi, çünkü bağlanmayan kitaplıkta “kullanılmayan öznitelik” uyarısı veriyor.

Aynı işte ilgisiz iki bozukluk da giderildi: `ui-showcase`'in model görüntüleyicisi yeni `quality`/`measure`
alanlarını almamıştı; eklenti SDK'sının simge listesi (`sdk/icons.txt`) son BIM simgelerini içermiyordu.

## Sonuçlar

- Şifreleme crate'leri (rsa, p256/p384, ed25519-dalek, sha1/sha2) RustCrypto'nun yeni kuşağındadır; imza
  doğrulaması aynı kalır, testler bunu gösterir.
- Gerçek imza kartıyla (PKCS#11) uçtan uca deneme bu işte yapılmadı.

## Ek: clippy temizliği (22 Kasım 2026)

`cargo clippy --workspace --all-targets` uyarısız ve hatasız. Rust 1.100'ün yeni lint'leri ve sıkı lint'leri
denetlenmemiş kod birikmişti; ilk koşu bir crate'te durduğu için tamamı ancak katman katman göründü (yaklaşık 450
bulgu, 13'ü derlemeyi durduran hata).

- **Sabit adımlı parçalar:** `chunks_exact(N)` → `as_chunks::<N>()` (yaklaşık 150 yer). İki hash işlevi
  (`kentos-domain` `SlotHasher`, masaüstü etiketlerinin `Fx`'i) aynı baytları aynı sırayla okur; kalan baytlar
  `as_chunks`'ın ikinci parçasından gelir.
- **Platformdan bağımsız sayı yolu (CLAUDE.md §23.4):** geometri çekirdeği, stil çekirdeği ve dosya biçimleri
  `clippy.toml`'da std `f64` matematiğini yasaklar; yeni kodda bunu atlayan yaklaşık 85 çağrı `jsmath`'e
  (`sin`/`cos`/`tan`/`atan`/`exp` libm'den, `js_min`/`js_max`/`js_hypot`/`js_sign`) ve `kentos_formats::math`'e
  alındı. `js_max` NaN'ı korur; değişen yerlerde NaN'ın girmediği gösterildi (bölen `rate < −1e-15`, sıfır uzunluklu
  yayda `s = 0`). Çekirdek testleri değişmeden geçer.
- **Üretim kodunda `expect`/`unwrap` (crate'lerin `deny`'ı):** `seam_network` kapanmayan izi bırakır; `drawing_repair`,
  `road_volumes_approve`, `table_export`, `road_culverts_hydraulics`, `plan_dimensions`, `casework`, `properties`
  zaten bulunan değeri kullanır ya da adı konmuş bir ret döner; `standards`'ın kaynak karşılaştırması
  `unwrap_or_default` kullanır (düz kayıtların JSON'u nesnedir).
- **`!(x > 0.0)` biçimi:** NaN'ı da yakalayan anlam korunarak `partial_cmp` ile yazıldı.
- **Yerel `allow`:** `type_complexity` (1.100'ün yeni puanlaması `Box<dyn Fn(f64) -> String>`'i de karmaşık sayar),
  `too_many_arguments`, `large_enum_variant` (komut işlemleri çağrı başına bir kez okunur) ve `result_large_err`
  depodaki alışkanlıkla (252 `too_many_arguments`) öğenin üstüne yazıldı; `approx_constant` iki yerde: kesişim
  ışınının keyfi yönü ve Euler sabitinin kendisi (`EULER_GAMMA` sabitine bağlandı).
- Gerisi küçük biçim düzeltmeleridir (birleşen `if`'ler, `is_multiple_of`, test modülünün dosya sonuna alınması,
  üç başsız test modülünün aynı `files.rs`'i bir kez yüklemesi).

Doğrulama: değişen 40'ı aşkın crate'in testleri geçti; masaüstünde 544 test (dokunulan alanlar ve çapraz süzgeçler).
Bu işten önce de kırık olanlar: 17 MCP testi (araçların `destructiveHint`'i, kaynak listesi; ikisi temiz `main`'de
koşturuldu, aynı sonuç) ve `module_features` (VY işlerinin yeni GIS modülleri `ALWAYS` listesinde yok; bu işin
dokunmadığı modüller, temiz `main`'de ayrıca koşturulmadı). `commandCatalog.json` gelen
bir açıklama değişikliğiyle yeniden üretildi.

### Kırık testlerin onarımı (22 Kasım 2026)

- **MCP araçlarının `destructiveHint`'i:** ADIM 14'te (13 Kasım, `1e4551bb3`) MCP'nin yıkıcı komut listesi
  `kentos_headless::agent_tools::DESTRUCTIVE`'e taşınırken 199 kimliğin yalnız 33'ü geçmişti; yol, imar, alan, şekil,
  katman, dışa/içe aktarma komutları dokuz gündür yıkıcı değil, bir kısmı da salt okunur görünüyordu. Liste eski
  biçimiyle, yorumlarıyla geri kondu; sonradan eklenen dördü (`cad.layers.session`, `cad.form.answer`,
  `cad.delivery.edit`, `cad.underlay.edit`) korundu (203 kimlik). 15 MCP testi bununla geçer.
- **MCP kaynak listesi:** `kentos://sdk/llms.txt` (`8f00ba19f`) testin beklediği listeye eklendi.
- **MCP BIM paketi:** B7 (ADR 8124) mimarın soyadını tahmin etmeyi bıraktı, künyenin `mimar_soyadi` alanını ister;
  `kesit-binasi-tam.kcad`'da bu alan yok. Künyeyi yazan bir komut olmadığı için test kopyaya alanı kendisi ekler
  (başsız testteki gibi); örnek dosya değişmedi.
- **`module_features`:** VY işlerinin 17 GIS modülünden 14'ü `mod-gis` özelliğinin arkasına alındı; çekirdeğin
  eriştiği üçü `ALWAYS` listesine yazıldı: `address_define` (komut kaydı), `gpkg_fields` (`geopackage`) ve
  `project_store` (başsız oturumun projeyi veri kaynağına kaydetmesi, ADR 9107).

Ayrı bir açık: yalnız `mod-gis`, `mod-map` ya da `mod-road` ile derleme bu işten önce de kırıktı (`sheets_set`
mimarinin kesit ve görünüş modüllerini, `ifc_system` `kentos_ifc`'yi koşulsuz kullanıyor); yalnız `mod-arch` ve
`mod-gis`'siz derleme temiz.
