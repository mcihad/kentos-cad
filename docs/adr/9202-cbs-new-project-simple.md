# ADR 9202: CBS'nin yeni projesi sade: ad ve konum; proje boş açılır, yeri kaydederken sorulur

- **Durum:** kabul edildi (2026-11-24). [ADR 9184](9184-cbs-project-on-one-page.md)'ün amaç ve saklama kartlarını ve
  [ADR 9196](9196-storage-by-project-kind.md) §2'nin sihirbazdaki veri yerini kaldırır.
- **İstek:** sahibin sözleri (24 Kasım 2026), sırasıyla:
  - “cbs uygulamasında yani gis modunda yeni proje wizard çok kötü bence, burada amaç bölümü gereksiz gerçekten, çok
    karmaşa var, mesela qgis yeni proje deyince tamamen boş bir proje geliyor ve harici kaynaklar katmanlar ekleniyor
    ve kaydederken soruyor bu yöntem de güzel”;
  - “wizard kaldırma ama sadeleştir”;
  - “çok ciddi kafa karıştırıyor”.
- **Kural:** CLAUDE.md §5 (koordinat sistemi tahmin edilmez, sorulur), §25 kural 5 (CBS'nin katman verisi
  veritabanında, açıklamalar proje dosyasında).
- **Web:** iptal edildi (sahibin sözü, 24 Kasım 2026). **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Araştırma

- **QGIS, *Working with Project Files*** (ADR 9184'te okundu, 23 Kasım 2026): Project › New boş bir proje açar;
  katmanlar sonra eklenir; Kaydet dosya sorar, Project › Save to GeoPackage ya da PostgreSQL'e kaydeder.
  <https://docs.qgis.org/3.44/en/docs/user_manual/introduction/project_files.html>
- KentOS'ta karşılıkları vardır: Harita ▸ Veri ekle var olan tabloları veri katmanı olarak ekler; Veri ▸ Yeni katman
  tabloyu GeoPackage'ta ya da PostgreSQL'de kurar, yerini sorar (ADR 9107 §7); ilk Kaydet `.kcad` dosyasını sorar;
  Dosya ▸ Veri kaynağına kaydet projeyi GeoPackage'ta ya da şemada saklar (ADR 9107 §5); Buluta yükle bulutta.

## §2 Karar

1. **Sihirbaz kalır, CBS'de tek sayfadır: “Ad ve konum”.** Proje adı, il ve koordinat sistemi (dilim şeridiyle).
   Amaç (Boş kent haritası, Kadastro paftası, Kurum şablonu), veri yeri (GeoPackage, PostgreSQL, KentOS Bulut) ve
   proje dosyasının yeri kartları kalktı. Adın altında bir not ne olacağını söyler: proje boş açılır, katmanlar
   Veri ekle ve Yeni katman ile eklenir, yer ilk Kaydet'te sorulur.
2. **Proje boş açılır.** Veri ya da çizim katmanı yoktur; yalnız metin ve notlar için **Açıklama** katmanı vardır
   (`kentos_project::new_project::cbs_start_layers`). Belge modeli en az bir katman ister (“Katmanlar: en az bir katman
   olmalı”, `.kcad` sözleşmesi); açıklama katmanı CBS'de proje dosyasındadır (kural 5), veri taşımaz. Hiçbir dosya
   ya da tablo yazılmaz.
3. **Çizim araçları veri katmanı bekler.** Etkin katman açıklama katmanıyken ve türüne uyan katman yokken Nokta,
   Çizgi, Çoklu çizgi, Kapalı alan kapalıdır; ipucu yolu söyler: “Nesneler veri katmanlarına çizilir: Harita ▸ Veri
   ekle ile bir tablo ya da Veri ▸ Yeni katman ile yeni bir katman ekleyin.” Açıklama katmanını Karışık yapmak
   önerilmez (kaydı reddettirirdi). Yazı açıklama katmanına yazılır.
4. **Kaydet sorar.** Yeri olmayan projenin ilk Kaydet'i dosya penceresini açar (`.kcad`); veritabanında saklamak
   Dosya ▸ Veri kaynağına kaydet'tir. Açılış iletisi bunu söyler.
5. **Kaldırılanlar:** `kentos_project::cbs_tables` (amaçların tablo ağacı), masaüstü sihirbazının `data_place.rs` ve
   `template.rs`'i (Kurum şablonu), ortak taslağın `purpose`, `data`, `storage`, `template` alanları ve kartları.
   Kurum şablonu bir değerlendirme raporunun maddesiydi (ADR 9184), sahibin doğrudan isteği değildi; gerekirse ayrı
   bir “Şablondan yeni proje” komutu olarak döner.
6. **Yan düzeltme:** sağ tuş menüsüne yönlendiren dört ileti ADR 9201'e göre Düzenle sekmesini söyler (ortak
   çekirdeğin bütünlük denetimi ve düzenleme oturumu, CBS'nin çizim uyarısı ve bekleyen düzenlemeler iletisi).

## §3 Kanıt

- `kentos-project`: `a_cbs_project_asks_its_name_and_place_and_starts_empty`; ortak örnek
  `fixtures/project/v1/wizard.json` yeniden yazıldı (CBS özetinde Amaç, Veri, Proje dosyası satırları yok; Katmanlar
  “Yok: Veri ekle ya da Yeni katman ile eklenir”).
- Masaüstü: `a_cbs_project_opens_empty_and_its_first_save_asks_where` (yalnız Açıklama, hiçbir şey yazılmaz, çizim
  araçları nedenli kapalı, yazı açık, ilk Kaydet dosyayı sorar ve yazar); `a_cbs_project_takes_its_provinces_zone_…`.
  Çapraz takım (CBS saklama, modül, katalog, ayarlar, katman türleri, şerit, proje): 306 sınama; programlar arası 37;
  `kentos-domain` ve `kentos-project` sınamaları geçer.
- Resim: `.run/gui-evidence/kullanim-9202/` (`once/` eski sayfa; `1-tek-sayfa`, `2-bos-proje`, açık 1440×900 ve koyu
  1100×760). Git'e eklenmez.

## §4 Kalan

- **Betiklerin yeni CBS projesi.** Python'un `Document.new` ve MCP'nin `drawing.new`'i ortak taslaktan geçer ama
  katman ağacını `new_project`'ten alır: CBS projesi bugün de Kadastro paftası ağacıyla (çizim katmanları) açılır.
  Kural 4 gereği masaüstüyle aynı başlangıç (`cbs_start_layers`) olmalı. Python'un 51 `Document.new` çağrısı ve
  başsız oturumun sınamaları öntanımlı katmanlara dayanıyor; ayrı dilimde ölçülerek çevrilir (plan/ayrim.md AY-05).
