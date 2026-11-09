# 4300 — Nokta bulutunda dönüşümün eksen ve açıyla verilmesi

Tarih: 2026-11-10. plan/map-cad.md MAP-13, sekizinci tur. Durum: taslak. ADR 3900'ün “Sonra”sındaki “eksen-açı ya da
Euler açılarıyla matris girişi (sözleşmesi ve kaynağı araştırılmalı)”.

İlgili:
- [ADR 3100](3100-apply-a-transform-to-a-point-cloud.md): bulutta dönüşüm uygulama (`pointcloud.transform`);
- [ADR 3500](3500-point-cloud-inverse-and-history.md): ters matris ve nesnenin dönüşüm geçmişi;
- [ADR 1800](1800-scan-registration-check.md): tarama kaydı, tarifi p ↦ R(p − c) + c + t.

## Dayanak

- **CloudCompare, Apply Transformation**
  ([belge](https://cloudcompare.org/doc/wiki/index.php/Apply_Transformation), 9 Kasım 2026'da okundu):
  - dönüşüm dört biçimde verilir: 4 × 4 matris; eksen ve açı; Euler açıları (phi, theta, psi) ve öteleme; tersinin
    uygulanması;
  - eksen ve açıda dönme bir 3B eksen çevresinde tek açıdır, açı derecedir; X, Y ya da Z çevresinde basit dönmenin en
    kısa yoludur;
  - Euler açılarının sırası ve ekseni (ZXZ ya da başka) sayfada yazmıyor, yalnız genel bir maddeye bağlanıyor.
- **Karar için:** Euler açılarının sırası yazılı bir kaynakta bulunamadı. Sırası tahmin edilen bir giriş yanlış dönme
  verir; KentOS değeri uydurmaz. Eksen ve açı ise sıra istemez: sağ el kuralı ve dereceyle tek anlamlıdır (Rodrigues
  formülü).
- **KentOS bugün** (ADR 3100, 3500): dönüşüm yalnız satır satır 16 sayıyla veriliyordu. Basit bir dönme için
  kullanıcı matrisi elle kuruyordu.

## Karar

1. **Dönüşüm uygula'ya Dönüşüm seçimi:** Matris (öntanımlı, eskisi gibi) ya da **Eksen ve açı**. Son dönüşümü geri al
   açıkken ikisi de gizlenir. Eksen ve açının alanları:
   - **Eksen:** X Y Z, uzunluğu önemsiz, öntanımlı 0 0 1;
   - **Açı:** derece. Sağ el kuralıyla: eksenin ucundan bakınca saatin tersine;
   - **Merkez:** X Y Z, projenin koordinatları; boş: bulutun kutusunun ortası;
   - **Öteleme:** X Y Z metre; boş: 0 0 0.
2. **Matris** (`Affine::axis_angle`, sabit sıra; bağımsız başvuru aynı bitleri bulur):
   - eksen uzunluğuna bölünür, √(x·x + y·y + z·z);
   - açı radyana çevrilir, derece × π/180; s ve c sinüs ve kosinüs, k = 1 − c;
   - satırlar [k·x·x + c, k·x·y − s·z, k·x·z + s·y], [k·x·y + s·z, k·y·y + c, k·y·z − s·x],
     [k·x·z − s·y, k·y·z + s·x, k·z·z + c];
   - ötelemenin satırı c_i + t_i − (R_i0·c0 + R_i1·c1 + R_i2·c2): Tarama kaydı'nın tarifi p ↦ R(p − c) + c + t.

   Tersini uygula bu matrisin tersini (ADR 3500 §1) uygular. Geçmişe (Dönüşümler) yapılan matris yazılır, öbür
   matrisler gibi.
3. **Retler:**
   - sıfır uzunlukta eksen: “Dönme ekseninin uzunluğu sıfır olamaz; ekseni üç sayıyla yazın (Z için 0 0 1).”;
   - üç sayı değil: “Eksen 3 sayıdır (X Y Z, boşlukla ayrılmış); 2 sayı verildi.”; merkez ve öteleme de aynı sözle;
   - sayı olmayan söz: “Merkez: “a” bir sayı değil; ondalık için nokta kullanın.”;
   - sonlu olmayan sayı.
4. **Euler açıları eklenmedi:** sırası kaynakta yazılı olunca ayrı karar.
5. **Kapsam işareti:** ADR 1800'deki gibi **Harita + CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/pointcloud_processing_cases.py`; laspy, numpy; matris aynı sırayla betikte),
  6 yeni durum (53):
  - Z çevresinde 0,5°, merkez boş, öteleme;
  - eğik eksen (2, 2, 1) çevresinde −0,25°, verilen merkez;
  - ters;
  - sıfır eksen, iki sayılı eksen ve sayı olmayan merkez retleri.

  Noktalar bayt bayt eşleşir; eski 47 durum değişmedi.
- **Çekirdek** (`ops::transform::tests::an_axis_and_an_angle_make_the_map`): (10, 0, 0) çevresinde Z'de çeyrek dönme
  (11, 0, 0)'ı (10, 1, 0)'a götürür; eksenin uzunluğu önemsiz.
- **Python** (`python/tests/test_cloud_axis_angle.py`): testin kendi kuaterniyonu (cos θ/2, sin θ/2 · n), Rodrigues'ten
  ayrı bir formül.
  - geçmişin matrisinin dönmesi kuaterniyonunki;
  - merkez kendisine artı ötelemeye gider;
  - X çevresinde sağ el çeyrek dönme +Y'yi +Z'ye götürür.

  `test_headless_clouds` yeni durumları da koşar.
- **MCP** (`crates/native/mcp/tests/cloud_transform_axis.rs`): sayı, özet ve geçmiş durumunki; sıfır eksen reddi.
- **Masaüstü** (`register_window_tests::an_axis_and_an_angle_from_the_pane_move_the_scan`): İşlem bölmesinden Eksen ve
  açı. Yazılan bulutun geçmişindeki matrisin dönmesi testin kuaterniyonununki, merkezin görüntüsü merkez artı öteleme.
  İşlem ve çapraz denetimler geçer. Resimler `.run/gui-evidence/eksen-aci-4300/`.

## Sonra

- Euler açıları (sırasının yazılı kaynağıyla).
- Çoklu tarama demeti; dönüşümün dosyasız nesnede tutulması; gerçek veride süre ve bellek ölçümü.
- Web (TODOS §0.2).
