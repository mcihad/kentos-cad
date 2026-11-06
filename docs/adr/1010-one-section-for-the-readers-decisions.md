# ADR 1010: Okuyucunun birim, düzlem, blok, katman ve kâğıt kararları tek yapılı bölümde

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-17` açığı “birim/OCS/blok/katman/kâğıt
  kararlarının tek yapılı rapor bölümü” (ADR 1061 §8).
- **Bağlam belgesi:** ADR 0589 (`UnitDecision`), ADR 1061 (eşit olmayan blokta kesin geometri), ADR 0588 (çoklu çizgi
  kaybı), `crates/shared/formats/src/report.rs`.

## 1. Kaynak

Autodesk'in DXF başvurusu
([Object Coordinate Systems (OCS) in DXF](https://help.autodesk.com/cloudhelp/2019/ENU/AutoCAD-DXF/files/GUID-D99F1509-E4E4-47A3-8691-92EA07DC88F5.htm))
çizgi ve nokta gibi nesnelerin dünya koordinatında, daire, yay, yazı ve 2B çoklu çizgi gibi nesnelerin kendi
düzleminde (OCS, 210 doğrultusu ve kot) saklandığını söyler. OCS'nin X ekseni “arbitrary axis” kuralıyla seçilir
(ezdxf'in [aynı kuralı aktaran belgesi](https://sources.debian.org/src/ezdxf/1.4.1-1/docs/source/concepts/ocs.rst)).
KentOS bu kuralı uygular (`geom::ocs_axes`). Ters doğrultulu düzlem plana aynalanarak kesin gelir; eğik düzlem 2B
modelde ancak izdüşümle temsil edilir. Belgeler okuyucunun kararlarını raporlama biçimi koymaz; aşağıdaki karar
KentOS'undur.

## 2. Karar

- `ImportReport.decisions` (yeni, boşken yazılmaz): `ImportDecision { topic, code, subject, decision, count, lines }`;
  `topic` ∈ `unit`, `ocs`, `block`, `layer`, `paper` (`ImportDecisionTopic`). Liste konuya göre, sonra alındığı
  sırayla dizilir. Kodlar sabittir:
  - **unit:** `unit_scale_reference`, `unit_chosen`, `unit_chosen_over_declared`, `unit_chosen_undeclared`,
    `unit_declared_not_scaled`, `unit_metres`, `unit_converted`, `unit_declared_same`, `unit_assumed_project`;
  - **ocs:** `ocs_flipped_mirrored`, `ocs_tilted_projected`, `ocs_block_tilted`, `ocs_invalid_extrusion`;
  - **block:** `block_own_unit`, `block_insert_scale_differs`, `block_exploded_too_deep`,
    `block_nonuniform_arcs_nurbs`, `block_nonuniform_widths_exact`, `block_nonuniform_arcs_sampled`;
  - **layer:** `layer_unused_left_out`, `layer_missing_created`, `layer_state_kept`;
  - **paper:** `paper_layout_invalid_size`, `paper_units_unsupported`, `paper_inches_to_mm`, `paper_default_a3`,
    `paper_viewport_frozen_layers_dropped`, `paper_empty_layout_left_out`, `paper_layouts_read`.
  - **style** ([ADR 1128](1128-dxf-table-styles.md) §4 ekledi): `table_style_from_acad_table`, `table_style_lossy`,
    `table_style_unused_left_out`.
- Eskiden not olarak söylenen kararlar (birim, blok birimi/ölçeği/derinliği, eşit olmayan blokta NURBS ve genişlik,
  pafta boyutu, varsayılan A3, görünümde dondurulmuş katman) aynı sözlerle `notes`'ta da kalır: yalnız notları okuyan
  istemci bozulmaz. Yeni kararlar (değişmeyen birim, ters/eğik düzlem, nesnesiz/tabloda olmayan/durumu korunan
  katman, inç pafta, boş Layout1, alınan pafta sayısı) yalnız `decisions`'tadır; notlar eskisi gibi kalır.
- Düzlem kararı OCS'de saklanan türler içindir: daire, yay, LWPOLYLINE, yazı, blok yerleştirmesi, tarama. Çizgi,
  nokta, 3B çoklu çizgi, spline ve elips dünya koordinatındadır, düzlem kararı almaz.
- Paftaların okuyucusu kendi raporunu ana rapora katarken (`absorb_losses`) kararları da taşır.
- Masaüstü Çizim al penceresi “Okuma kararları” başlığı altında kararları konu adıyla
  (“Katman “YOK”: …”, “Düzlem “ARC”: …”) gösterir; karar olan notu ikinci kez göstermez.

## 3. Kanıt

- `scripts/fixtures/import_decisions_cases.py` ezdxf ile `fixtures/exchange/v1/decisions.dxf`'i yazar (milimetre;
  kapalı, dondurulmuş, tabloda olmayan ve nesnesiz katmanlar; ters ve eğik düzlemli daire/yay; 3 × 0,5 ölçekli
  yaylı ve genişlikli blok; bir pafta ve boş Layout1) ve dosyanın kuruluşundan beklenen kararları
  `decisions.json`'a yazar (KentOS kodu kullanmaz). `crates/shared/formats/tests/all/import_decisions.rs` konu, kod,
  konu adı ve sayıyı sırasıyla, nesnesiz katman listesini ve pafta/görünüm sayısını sözlerde karşılaştırır; eski
  notların notlarda kaldığını, sessiz kararın notlara girmediğini sınar.
- İlk koşuda tek fark betikteydi: geçersiz ezdxf biçim dizgesi yayı düz yazmıştı; düzeltilince eşleşti. KentOS'un
  kuralına dokunulmadı.
- `kentos-formats` testleri geçti (önceden bilinen `tests/gis.rs` yay örneklemesi son basamak farkı dışında).
- Masaüstü: `import_decision_scenes.rs` testi ve resmi (`okuma-kararlari`).

## 4. Sınırlar

- DWG okuyucusu (OpenCADCodec üzerinden DXF'e çevrilen yol) aynı DXF okuyucusundan geçtiği için aynı kararları
  verir; ayrı DWG dosyasıyla sınanmadı.
- Görünüm başına katman durumu, kâğıt birimi dışı plot ayarları ve blok içindeki nesnelerin katmanları karar olarak
  yazılmaz. AutoCAD ile karşılaştırma yapılmadı (CAD-IO-20).
- Web dururken (ADR 0228) web'in rapor görünümü güncellenmedi; TS türleri üretildi.
