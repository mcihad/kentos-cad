# ADR 1061: Dosya alışverişinde kesin geometri, kotların korunması, açık düzleme indirme ve kaynağın korunması

- **Durum:** kabul edildi (5 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-17` (“Dosya alışverişinde görüntü,
  geometri ve düzenlenebilirliği ayrı güvenceye al”) ve `CAD-IO-15`'in 2B eşit olmayan dönüşüm kısmı. İlke
  “Performance First”; madde tek parçada.
- **Bağlam belgesi:** ADR 0586/0587 (DWG/DXF yolu, OpenCADCodec), 0588 (çoklu çizgi bütünlüğü), 0589 (blok birimleri),
  0625 (blokta XYZ), 0142/0224 (Z ve M), 0350/0506 (paket), `docs/cad/exchange-support.md` (kapsam matrisi).
- **Web:** ertelendi (ADR 0228); TS türleri üretildi, web arayüzü yazılmadı.

## Bağlam

Koddan doğrulanan açıklar:

- **Eşit olmayan ölçek.** DXF okuyucusu (`dxf/emit.rs`) blok yerleştirmesindeki eşit olmayan ölçek altında daireyi
  elipse çeviriyordu (doğru), ama yaylı çoklu çizgiyi noktalara bölüyor ve genişlikli çoklu çizginin genişliğini
  atıyordu; ikisi de `polyline_geometry_loss` reddine (ADR 0588) düşüyordu. CAD-IO-15 bunun için açıktı.
- **Z/M.** Açılış kotları koruyordu (ADR 0142, 0625), ama kotları bilerek kaldırmanın komutu yoktu: kullanıcı
  “düzleştirme”yi ancak nesneleri tek tek düzenleyerek, önizlemesiz yapabiliyordu.
- **Kaynağın ezilmesi.** Kaydet kaynağa yazmaz (ADR 0587). Ama `file.export.dxf`/`dwg` `overwrite: true` ile açılan
  kaynağın kendisinin üzerine yazabiliyordu; masaüstü dışa aktarması her zaman `overwrite: true` gönderiyordu
  (yerel “Değiştir?” sorusundan sonra): kaynağın KentOS dışı bir çizim olduğu söylenmiyordu.

### Araştırma (resmî belgeler)

- AutoCAD FLATTEN (Express Tools): Autodesk'in destek makalesi (“How to flatten a drawing in AutoCAD Products”,
  autodesk.com/support) çizgi, yay ve çoklu çizgilerin Z değerlerini 0 yaptığını, “Remove hidden lines” sorusuna N
  verilmesini söyler; komut Express Tools ister. Önizleme yoktur ve Z 0'a yazılır (kotsuz değil). KentOS kararı: ayrı,
  önizlemeli, geri alınabilir ürün komutu; kot kaldırılır (kotsuz, 0 değil) ya da açıkça verilen düzleme getirilir;
  gizli çizgi kaldırma yok (2B model).
- Autodesk DXF Reference (LWPOLYLINE 40/41/43 genişlik, 42 çıkıntı; INSERT 41/42 ölçek): genişlik çizginin
  düzlemindeki enine uzunluktur, çıkıntı dairesel yaydır. Afin dönüşüm altında dairesel yay eliptik yay olur; bu,
  DXF'in yaylı çoklu çizgisiyle değil, rasyonel B-spline (SPLINE, ağırlıklı) ile tam gösterilir.
- Piegl & Tiller, *The NURBS Book*, §7.3–7.4: koniklerin rasyonel ikinci derece Bézier gösterimi ve afin dönüşümde
  denetim noktalarının dönüşüp ağırlıkların korunması (afin değişmezlik).

## Karar

### 1. Kapsam

Yeni aktarım motoru açılmadı; mevcut DXF/DWG okuma, `cad_open` ve ortak içe alma yolu korundu. Bu iş üç somut boşluğu
kapatır (§3–§5), kabul kümesini kurar (§6) ve kapsam matrisini nesne/özellik düzeyinde görüntü ve düzenlenebilirlik
ayrımıyla günceller (§7). Kapsam dışı ve açık kalanlar §8'de.

### 2. Veri modeli

Yeni `.kcad` şeması yok. Yeni sözleşmeler: `cad.entities.flatten` v1 (`EntitiesFlatten`, `EntitiesFlattened`),
`ExportDxf.replaceSource`, `ExportDwg.replaceSource`; yeni kodlar `source_overwrite`, `nothing_to_flatten`.

### 3. Eşit olmayan dönüşümde kesin geometri (CAD-IO-15)

- **Düz genişlikli kenar.** İki paralel doğru arasındaki şerit, afin dönüşümle yine şerittir. Kenar `i`'nin başlangıç
  ve bitiş genişlikleri |det M| / |M·t| ile çarpılır (t kenarın kaynaktaki birim doğrultusu). Yalnız tek kotlu (ya da
  kotsuz) yolda uygulanır; eğik (OCS) yolun genişliği kendi düzlemindedir, mevcut güvenli ret sürer. Uzunluğu sıfır
  kenarda ya da tekil dönüşümde eski kayıp yolu sürer. Uç ve köşe kesimleri kaynakta kenara dikti, burada değil: bu
  bilgi notu olarak söylenir (kayıp değil).
- **Yaylı, açık, genişliksiz, kotsuz çoklu çizgi.** İkinci derece rasyonel B-spline olur: her kenar bir rasyonel
  Bézier parçası. Düz kenar: uçları ve ortası, ağırlık 1. Yay: |çıkıntı| ≥ 1 iken ikiye bölünür (tan(θ/8) =
  b / (1 + √(1 + b²))); uçları ve uç teğetlerinin buluşma noktası (kirişin ortası + c/2 · tan(φ/2), tan(φ/2) =
  2b/(1 − b²)), ağırlık cos(φ/2) = (1 − b²)/(1 + b²). Denetim noktaları dönüşür, ağırlıklar kalır; parçalar çift iç
  düğümle birleşir. Sonuç örneklenmez; çoklu çizginin köşe düzenlemesi yerine NURBS düzenlemesi gelir (bilgi notu).
  Kapalı yaylı yol (alan), genişlikli yaylı yol ve kotlu yaylı yol eski yolda kalır (örnekleme + açık kayıp izni).

### 4. 2B düzleme indir (`cad.entities.flatten`)

Kalıcı kimlikleriyle verilen nesnelerin kotlarını kaldırır ya da `plane` ile hepsini o kota getirir; `measures: true`
ölçüleri (M) de kaldırır. Kapsanan: nokta (`z`, ek noktaların `z`'si), çizgi (`za`, `zb`, `ma`, `mb`), çoklu çizgi ve
alan (`zs`, `ms`, parçaların ve deliklerin `zs`/`ms`'i). Kotu olmayan köşe kotsuz kalır (0 değildir); düzlemde zaten
olan köşe değişmez ve sayılmaz. Başka türler atlanır.

Cevap (plan da): değişen nesneler (girdinin sırasıyla), kimlikleri, kotu giden ya da değişen köşe sayısı, o köşelerin
en küçük ve en büyük kotu, kaldırılan ölçü sayısı. Plan önizlemedir. Tek geri alma adımı “2B düzleme indir”.

Retler sırasıyla: `no_entities`, `invalid_uid`, `not_finite` (`plane`), `invalid_revision`, `revision_conflict`,
`entity_not_found`, `layer_locked` (hepsi kilitli; bir kısmı kilitliyse uyarıyla atlanır), `nothing_to_flatten`.

**Açılış kotları hiçbir zaman kendiliğinden silmez;** bu komut açıkça çalıştırılmadıkça Z ve M kalır.

### 5. Kaynağın habersiz ezilmemesi

`file.export.dxf`/`dwg`: hedefte bir dosya varsa `overwrite` gerekir (eskisi gibi). Ayrıca dosya **KentOS'un
yazmadığı bir CAD çiziminse** (APPID tablosunda KENTOS kaydı olmayan DXF; ya da aynı codec'le DXF'e çözüldüğünde
KENTOS kaydı çıkmayan ya da çözülemeyen DWG, `kentos_dwg::written_by_kentos`) `replaceSource: true` de gerekir; yoksa `source_overwrite` reddi, dosyaya dokunulmaz. CAD çizimi olmayan
dosya ve KentOS'un kendi dışa aktarması yalnız `overwrite` ister. Masaüstü `replaceSource` göndermez: kullanıcı
kaynağın üzerine yazmak isterse ret iletisi başka ad vermesini söyler.

### 6. Kabul kümesi ve aç–düzenle–kaydet–yeniden oku

`fixtures/exchange/v1/set.dxf`, KentOS'tan bağımsız bir yazıcıyla (ezdxf 1.4.5, sabit üst veriyle her çalıştırmada
aynı bayt) yazılır: kendi birimli iç içe bloklar (mm kapı, m kat), yaylı ve genişlikli çoklu çizgi, elips, rasyonel
NURBS, eşit olmayan ölçekli blokta yaylı ve düz genişlikli yol, kotlu çizgi ve nokta, karmaşık çizgi tipi (deseninde
yazı), desenli tarama, hizalı ölçü, iki pafta (her birinde görünüm).

### 7. Arayüz

Masaüstü: Giriş › Değiştir › **2B düzleme indir…** (`cad.flatten`; ikon `flatten2d`: düzlemin üstünde eğik bir
yüzey ve aşağı ok). Pencere seçimi alır ve komutun planını gösterir: kaç nesne, kaç köşe, hangi kotlar arasında, kaç
ölçü. Seçenekler: “Kotları bir düzleme getir” ve kotu, “Ölçüleri (M) de kaldır”. Uygula tek adım yazar.

### 8. Kapsam dışı ve açık sınırlar

- **Patlat (KentOS içinde)** eşit olmayan yerleştirmenin yaylı yolunu çekirdeğin mevcut kuralıyla açar; §3 yalnız
  dosyadan okumadadır. CAD-IO-15'in KentOS içi patlatma kısmı açık.
- **Kapalı yaylı alan** eşit olmayan ölçek altında kesin değildir (alan modelinin halkası dairesel yay taşır); örnekleme
  ve açık kayıp izni sürer.
- **Birim/OCS/blok/katman/kâğıt kararlarının tek yapılı rapor bölümü** bu işte eklenmedi; kararlar mevcut raporun
  notlarında ve `UnitDecision`'dadır (ADR 0589).
- **Gerçek AutoCAD denetimi yapılmadı** (CAD-IO-20 sahibin kararıyla kapsam dışı). OpenCADStudio/OpenCADCodec okuyucusu
  tam DWG yazıcısı diye sunulmaz; DWG yazımı DXF'ten dönüştürülür ve aynı codec'le yeniden açılır (ADR 0586).
- **NCZ** için NCZ-01 lisans/dağıtım kapısı korunur.

## Uygulama

- Çekirdek okuyucu: `crates/shared/formats/src/dxf/emit.rs` (`exact_widths`, `arcs_as_nurbs`, `bulge_point`).
- Sözleşme: `crates/shared/commands/src/cad_flatten.rs`; `crates/shared/contracts/src/file_export.rs`, `file_cad.rs`,
  `codes.rs`; katalog ve yetenekler.
- Uygulama: `crates/native/application/src/entities_flatten.rs`; kayıt `lib.rs`, `registry.rs`.
- Başsız: `crates/native/headless/src/file_commands.rs` (`foreign_cad`, `guard_source`).
- Masaüstü: `apps/desktop/src/flatten_window.rs`, `flatten_scenes.rs`; `catalog/v1/additions.json`,
  `catalog/v1/operations.json`.
- MCP: `cad.entities.flatten` yıkıcı işaretli (kot ve ölçü siler).

## Doğrulama

- Bağımsız başvurular: `python3 scripts/fixtures/flatten_command_cases.py --check` (27 durum),
  `python3 scripts/fixtures/cad_polyline_policy_cases.py --check` (20 kaynak, 35 durum; düz genişlikli beş kaynak artık
  kabul, noktaları ve genişlikleri INSERT'in kendi dönüşümünden), `.run/py/bin/python scripts/fixtures/exchange_set_cases.py
  --check` (ezdxf yazıcısı) ve `--roundtrip` (KentOS'un Python modülüyle aç–dışa aktar, ezdxf ile oku).
- Analitik birim testleri (`emit.rs`): yarım çember x×3, y×0,5 altında her noktası (x/3)² + (y/0,5)² = 1 üzerinde
  (1e-12); çeyrek saat yönü yay; 45° kenarın genişliği.
- Kabul testi `crates/native/headless/tests/headless/exchange_set.rs`: açılışta Z korunur (çizgi 3,5/4,25, nokta 12),
  elips ve ağırlıklı NURBS analitik kalır, mm kapı m kat içinde 0,9 m, iki pafta gelir, yabancı DIMENSION resmi olarak
  gelir ve raporda söylenir; patlatarak içe almada eşit olmayan yay kesin NURBS (her noktası eşlenmiş elips üzerinde,
  1e-9) ve düz genişlikler 0,2 × 1,5/|M t|; 2B düzleme indir önizlemesi 3 köşe, 3,5–12 m, önizleme yazmaz, tek Undo
  geri getirir; kaynağın üzerine dışa aktarma `source_overwrite`, kaynak bayt bayt aynı; `replaceSource` ile kopyaya
  yazılır, KentOS'un kendi dışa aktarmasına yalnız `overwrite` yeter; yeniden okumada her tür ve Z kalır.
- ezdxf çapraz okuması (`--roundtrip`, gerçekten çalıştırıldı): KentOS'un dışa aktarmasında elips, tarama, iki pafta,
  ölçünün resmindeki yazı, eşit olmayan bloğun ölçekleri ve içindeki çıkıntı 1, patlatılmış içe almanın dışa
  aktarmasında 2. derece ağırlıklı SPLINE'ın 17 örnek noktasından en az 11'i (yay parçaları) eşlenmiş elips üzerinde (1e-9),
  genişlikler 0,1 ve 0,6; çizgi ve nokta kotları; kaynak dosya değişmedi.
- Bulunup düzeltilen: ezdxf yazıcısı Python'un rastgele dize karıştırmasıyla birkaç baytı farklı yazabiliyordu;
  betik kendini `PYTHONHASHSEED=0` ile yeniden çalıştırır (`--check` ardışık altı çalıştırmada tutarlı). Ezdxf'in
  boyut stilinin yazısı “1000”dür (kaynakta), KentOS onu değiştirmeden taşır.
- Masaüstü resimleri: `KENTOS_SHOTS_ONLY=duzleme-indir,duzleme-indir-kot cargo test -p kentos-desktop --lib
  tools_screens -- --ignored --nocapture` (1440×900, 1100×650, açık ve koyu), `.run/gui-evidence/cad-adv-17/`.
