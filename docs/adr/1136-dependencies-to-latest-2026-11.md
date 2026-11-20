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
  Geri sarma çalışır, yalnız aranması yavaştır. Bu yüzden yalnız bu not, masaüstü crate'inin (lib ve bin) ve
  Python modülünün kökünde `#![allow(linker_messages)]` ile susturuldu.

Aynı işte ilgisiz iki bozukluk da giderildi: `ui-showcase`'in model görüntüleyicisi yeni `quality`/`measure`
alanlarını almamıştı; eklenti SDK'sının simge listesi (`sdk/icons.txt`) son BIM simgelerini içermiyordu.

## Sonuçlar

- Şifreleme crate'leri (rsa, p256/p384, ed25519-dalek, sha1/sha2) RustCrypto'nun yeni kuşağındadır; imza
  doğrulaması aynı kalır, testler bunu gösterir.
- Gerçek imza kartıyla (PKCS#11) uçtan uca deneme bu işte yapılmadı.
