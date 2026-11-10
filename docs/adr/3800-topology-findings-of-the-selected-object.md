# 3800 — Seçili nesnenin topoloji bulguları (veri formu yolu)

Tarih: 2026-11-09. plan/map-cad.md MAP-07, yedinci tur. Durum: taslak. ADR 1980'in “Sonra”sındaki “Veri formu yolu:
seçili nesnenin topoloji bulgularının Öznitelikler panelinde ve formda görünmesi”; ADR 2200, 2600, 3000 ve 3400'ün aynı
satırı.

İlgili:
- [ADR 0202](0202-topology-rules.md): topoloji kuralları ve Topoloji sekmesi;
- [ADR 0589](0589-topology-check-bound-to-sources.md): `cad.topology.check`;
- [ADR 1260](1260-topology-quality-review-and-repair-recipe.md): bulgunun yaşam döngüsü (Durum);
- [ADR 1980](1980-topology-check-in-processing.md): denetimin İşlem araç kutusundan yolu;
- [ADR 0401](0401-forms-core.md): Formlar ve Formla düzenle.

## Dayanak

- **ArcGIS Pro, Validate and fix topology errors**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/help/editing/validate-and-fix-geodatabase-topology.html),
  9 Kasım 2026'da okundu):
  - Error Inspector'ın süzgeçleri kurallar, sınıflar, kural türleri, hata geometrisi, istisnalar ve harita kapsamıdır;
    süzgeçler VE ile birleşir;
  - Select Features hatanın nesnelerini seçer; Zoom To Selection ve Zoom to Error hataya gider;
  - Details sekmesi nesne ve hata hakkında bilgi verir.

  Bulgudan nesneye gidilir; belge nesneden onun bulgularına giden bir süzgeç ya da nesnenin kendi panelinde bir liste
  anlatmıyor.
- **QGIS, Topology Checker** ([belge](https://docs.qgis.org/3.40/en/docs/user_manual/plugins/core_plugins/plugins_topology_checker.html),
  9 Kasım 2026'da okundu): Validate All ve Validate Extent; hata listesi hatanın türü, katman ve nesne kimliğiyle;
  kurala göre süzgeç; satıra basınca harita nesneye gider. Nesnenin öznitelik formunda hata gösterilmez.
- **Plan (MAP-07, CBS ortaklığı):** GIS'te de Processing, veri formu ve MCP üzerinden aynı denetim sonucu erişilir.
  Processing yolu ADR 1980'dir.
- **KentOS bugün:** bir parselin bulgularını görmek için Topoloji sekmesinin listesi gözle taranıyordu. Python ve MCP
  bütün denetimi alıp kendileri süzüyordu.

## Karar

1. **`cad.topology.check`'in `uids`'i** (ek alan; komut sürümü 1 kalır):
   - yalnız bu nesnelerden birini anan bulgular listelenir;
   - sayılar (`total`, `open`, `exceptions`, `lapsed`, `errors`) ve düşen istisnaların uyarıları bu bulgularındır;
   - öbürleri (`rules`, `objects`, `complete`, damga) bütün denetiminkidir: denetim yine bütün kurallarla yapılır,
     yalnız cevap süzülür;
   - `limit` süzülenlere uygulanır;
   - retler: `invalid_input` (`uids` boş), `invalid_uid` ve `entity_not_found` (`uids[i]`); kural retlerinden sonra,
     önizleme retlerinden önce. Kimlik projenin kuralıyla küçük harflidir.
2. **Öznitelikler › Topoloji** (Harita ve CBS, Topoloji sekmesi olan programlar; tek nesne seçiliyken, Geometri'nin
   altında):
   - bölüm yalnız projenin bir kuralı nesnenin katmanını anıyorsa görünür;
   - **kendi denetimi yoktur:** Topoloji sekmesinin son denetiminden okunur, seçim yavaşlamaz;
   - denetim yoksa “Denetlenmedi: Topoloji sekmesinde Denetle.”;
   - her bulgu bir satır: sorun ve ölçü (yoksa durum), altında sessiz satır “Değerlendirilmedi · kural: Çakışmamalı”;
     satırdaki düğme Topoloji sekmesini o bulguda açar (bütün bulgular listelenir, satırı seçilir, nesneleri seçilir,
     görünüm ona gider);
   - bulgu yoksa “Son denetimde bulgusu yok.”; çizim denetimden sonra değiştiyse uyarı: “Topoloji sekmesinde
     Değişenleri denetle.”;
   - bölüm önbelleğe girmez (denetim çizimi değiştirmez, önbelleğin anahtarı onu görmez).
3. **Formla düzenle:** tek kaydın formunun başında, bulgu varsa özet: “Topoloji: son denetimde 2 bulgu.” ve her bulgu
   bir satır (sorun · kural · durum · ölçü). Bulgu yoksa satır yok; form sade kalır. Tasarımcının önizlemesinde ve
   çoklu kayıtta yok.
4. **Sözcükler** Topoloji sekmesininkidir (`plan.rs`, `review.rs`): sorun, kural, durum ve gerekçe, ölçü.
5. **Kapsam işareti:** **Harita + CBS** (MAP-07'nin CBS ortaklığı); komutun süzgeci Python ve MCP'de.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/topology_check_cases.py`, KentOS kodu yok; bulgular topology_rules_cases.py'nin
  başvurusundan, süzgeç betikte):
  - 12 yeni durum (35):
    - A1'in bulguları, A3 ile A5'inkiler;
    - bulgusu olmayan nesne (boş liste);
    - süzgeçle liste sınırı;
    - istisnanın süzülen bulguda sayılması;
    - düşen istisnanın yalnız süzülen bulguda uyarılması;
    - retler: boş liste, UUID olmayan, büyük harfli, çizimde olmayan kimlik; bilinmeyen kural kimliklerden önce;
  - eski 23 durum ve `cad.topology.exceptions`'ın 39 durumu aynen üretildi. Rust ve Python geçer; yetenek testi geçer.
- **Python** (`python/tests/test_topology_object_findings.py`): her parselin süzülen bulguları, bütün denetimin onu anan
  bulgularıdır (testin kendi süzmesiyle); iki parselin birleşimi her bulguyu bir kez verir; retler çizimi değiştirmez.
- **MCP** (`crates/native/mcp/tests/topology_object_findings.rs`): A3 ve A5'in bulguları ve sayıları durumunkiler;
  çizimde olmayan kimlik `uids[1]` yoluyla reddedilir.
- **Masaüstü** (`topology::object_tests`):
  - Harita'da A1: denetimden önce “Denetlenmedi”, sonra “Topoloji (1)”, sözcükler denetiminkiler;
  - düğme sekmeyi bulguda açar, A1 ile A2 seçilir;
  - A5'in kısa kenarı;
  - A2 kayınca çizimin değiştiği söylenir;
  - CBS'de bölüm var, Yol'da yok;
  - Formla düzenle'de 15 m kısa kenar kuralıyla binanın iki bulgusu, denetimden önce hiç satır yok.

  Öznitelikler, topoloji, form ve İşlem araç kutusunun testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/topoloji-nesne-3800/`.

  Not: `typing_tests::project_settings_scale_and_heights_draw_each_key_at_once` bütün bu testlerle aynı koşuda düştü,
  tek başına ve 4 iş parçacığıyla geçer: tuşa basma ile çizim arasındaki zamanlamaya bağlı, bu değişiklikle ilgisiz.

## Sonra

- ~~Bulgunun Öznitelikler'den Hata işaretle ya da İstisna yap'ı; çoklu seçimde nesnelerin bulgu sayıları~~
  ([ADR 4600](4600-topology-findings-marked-from-attributes.md)).
- ~~Köşe sil'de birleşen kenarın içindeki köşe~~ (ADR 4200); tutamaçla birden çok nesne; büyük veri ölçümü (MAP-07'nin
  kalanı).
- Web (TODOS §0.2).
