# ADR 0346: Mimari çalışma şablonu ve evre gösterimi

- Durum: Kabul edildi (18 Ekim 2026)
- Kapsam: plan/cad.md `CAD-ARCH-41`, `CAD-ARCH-42`; 7. grubun ilk parçası (7a). `43`, `44`, `45` ADR 0347'de.
- İlgili: ADR 0312 (çizim standardı, `mimari-ofis`), ADR 0318 (KentOS blok kitaplığı), ADR 0339 (duvar stilleri, katlar),
  ADR 0342–0344 (mahal, aks, kolon, merdiven, etiket, çizelge; `arch.phase`), ADR 0251 (pafta kitabı), ADR 0211 (katman süzgeci),
  ADR 0210 (zaman penceresi).

## Bağlam

Mimari nesneler (duvar, açıklık, mahal, aks, kolon, merdiven, rampa) bir evre taşır (`arch.phase`: mevcut, yıkılacak, yeni, geçici)
ama çizim onu göstermiyordu. Bir mimari proje de her seferinde katmanlar, stiller, bloklar, duvar stilleri, katlar ve paftalar
elle kurularak başlıyordu. Plan iki şeyi ister: bunların tek bir ürün paketi olarak kurulması ve evrelerin görünüşle, etiket ve
çizelge süzgeçleriyle çalışması.

## Karar

### 1. Mimari çalışma şablonu (`CAD-ARCH-41`)

- Paket veri olarak `catalog/v1/templates/mimari.json`: adı, kullanıcı birimi (`cm`), çizim ölçeği (1:50), katlar (Zemin kat 0,
  3 m; 1. kat 3 m, 3 m), mahal yöntemi (net), çizim standardı (`mimari-ofis`: katmanlar, yazı ve ölçü stilleri, çizgi tipleri),
  blok kitaplığı (`kentos-temel`: kapı, pencere, mobilya, donatı, kot işareti, kuzey oku, antet), KentOS'un duvar stilleri ve
  paftalar (A-101 Zemin kat planı 1:50, A-301 Kesitler 1:50, A-501 Detaylar 1:20; A3 yatay, birer harita çerçevesi; çıktı
  profili 300 dpi; pafta PDF olarak `file.export.pdf` ile yazdırılır).
- `cad.templates.apply` (“Mimari şablonu uygula”): `name` (`mimari`), isteğe bağlı `unit` (`mm` ya da `cm`; yoksa paketin).
  Parçaları sırayla uygular ve var olanı korur:
  - birim: çizim boşsa yazılır; nesnesi olan çizimin birimi değişmez (`template_unit_kept` uyarısı, ADR 0256'nın
    `cad.units.change`'i ayrı bir karardır);
  - ayarlar: çizim ölçeği, eksik katlar (kimliğiyle), mahal yöntemi yoksa;
  - standart: `cad.standards.apply`'ın kuralıyla (eksik katmanlar ve stiller, var olan katmanın görünüşü standardınki);
  - bloklar: kitaplığın çizimde adı olmayan blokları;
  - duvar stilleri: projede adı olmayan KentOS stilleri;
  - paftalar: kitapta kimliği olmayan paftalar (pafta çekirdeğinin `addSheet`'i).
  Nesneler, katmanlar ve bloklar tek geri alma adımı “Mimari şablonu”; ayarlar ve pafta kitabı geri alma adımı değildir (ADR
  0251, 0023). Çıktı eklenen katmanları, blokları, duvar stillerini, katları ve paftaları sayar.
- Masaüstünde Mimari › Proje'de Mimari şablonu; Python `kentos.cad.templates.apply`, MCP.

### 2. Evre gösterimi (`CAD-ARCH-42`)

- Projenin `arch.phaseView`'u (`.kcad` şema 82): `all` (Tümü), `existing` (Mevcut durum), `demolition` (Yıkım planı),
  `proposed` (Yeni durum). Yoksa Tümü.
- Her görünümde her evrenin görünüşü (`PhaseLook`):

  | Görünüm | Mevcut | Yıkılacak | Yeni | Geçici |
  |---|---|---|---|---|
  | Tümü | olduğu gibi | olduğu gibi | olduğu gibi | olduğu gibi |
  | Mevcut durum | olduğu gibi | olduğu gibi | gizli | gizli |
  | Yıkım planı | olduğu gibi | kesikli | gizli | gizli |
  | Yeni durum | soluk | gizli | olduğu gibi | kesikli |

  Evresi olmayan nesne her görünümde olduğu gibidir. Kesikli: nesnenin ve parçalarının çizgi tipi `dashed`; soluk: %60 saydam,
  dolgular da. Gizli: çizilmez, seçilmez, kenetlenmez (deponun görünüm süzgeci, ADR 0210 ve 0211 gibi), paftanın haritasında da.
- Açıklık duvarının evresini, etiket nesnesininkini alır (`Entity::phase`).
- Etiketin `all`'ı `phases` süzgeci alır (verilen evrelerin nesneleri); çizelgenin süzgecinde evreler zaten vardır (ADR 0344 §3).
- Kural tek yerde: `kentos_contracts::PhaseView::look`; çizimde `kentos_native_style::phases` nesnenin görünüşünü yazar,
  masaüstünün deposu gizlileri işaretler.
- Masaüstünde Mimari › Proje'de Evre ▾ (dört görünüm, işaretli); `cad.settings.set`'in `arch`'ıyla yazılır.

## Sonuçlar

- Şablon yeni proje sihirbazına bağlanmadı: paket var olan bir çizime komutla uygulanır; sihirbazın seçeneği ayrı bir iştir.
- Evre görünüşünün renkleri sabittir (kesikli ve soluk, katmanın rengiyle); görünüm başına renk ve tarama ayarı ileride.
- DXF'e evre görünüşü yazılmaz: nesneler kendi görünüşleriyle gider (evre KENTOS verisindedir).

## Doğrulama

- Ortak durumlar: `python3 scripts/fixtures/template_command_cases.py --check` (cad.templates.apply, KentOS kodu olmadan,
  en az 20 durum) ve `python3 scripts/fixtures/tag_command_cases.py --check` (etiketin `phases` süzgeci).
- Evre tablosu: `python3 scripts/fixtures/phase_cases.py --check` ve `cargo test -p kentos-native-style --test all phases`.
- `.kcad` şema 82: `fixtures/kcad/v2/phases.json` ve bozuk örnekleri (`python3 scripts/fixtures/kcad_v2_reference.py --check`).
- Resimler: `KENTOS_SHOTS_ONLY=mimari-sablon,evre-tumu,evre-yikim,evre-yeni cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
