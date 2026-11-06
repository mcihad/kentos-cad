# ADR 1016: Kişisel araç paleti: ofis araçlarının favorisi ve kendi araçlar

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` maddesi “Parametreleri hazır bir aracı
  veya mevcut nesneyi KentOS araç paletine kaydetme, açıklama/önizleme, arama ve favori akışı …; mevcut komut kaydına
  bağlı araç tarifleri kullanılsın” ve ADR 1014 §4'ün sınırı.
- **Bağlam belgesi:** ADR 1014 (ofis paletleri, anahtar ve özet), ADR 1100 §4 (kişisel tercih belgeye karışmaz),
  `apps/desktop/src/templates_panel.rs` (nesne şablonlarının kişisel paleti).

## 1. Karar

- Kişisel araç paleti kullanıcının ayar klasöründe `arac-paleti.json`'dur (`kentos.tool-palette` sürüm 1), çizime
  yazılmaz: en çok 100 favori, en çok 100 kendi araç. Okunmayan dosya yok sayılır ve söylenir.
- **Favori:** ofis aracının ☆'sı aracı anahtarı ve tarif özetiyle kaydeder. Ofisin yeni revizyonunda araç değişirse
  (aynı anahtar, başka özet) favori **çalışmaz**, “ofis revizyonunda değişti; yeniden seçmek için ★” der; ★ yeni tarifi
  onaylar. Artık yayımlanmayan favori bunu söyler. Favori başka bir araca sessizce kaymaz.
- **Kendi araç:** “Son komutu araç yap” çizimin günlüğündeki son komut satırını (komut, sürüm, girdi) kendi araç yapar;
  başlığı komutun katalog başlığıdır. Kendi araç ofis aracı gibi komut kaydından çalışır, tek geri alma adımıdır;
  komut reddederse nedeni söylenir.
- Şablonlar'ın araması favorileri, kendi araçları ve ofis araçlarını başlıklarıyla süzer. Nesneyi şablon olarak
  kaydetmek mevcut “Seçili nesneden şablon” akışıdır (değişmedi).

## 2. Kanıt

`office_palettes::tests::a_favourite_keeps_its_recipe_and_an_own_tool_is_the_journals_last_call`: favori dosyada kalır,
yeni revizyonda özeti tutmaz, yeniden seçilince yeni özeti tutar; çalışan ofis aracı günlükten kendi araç olur
(“Çizgi oluştur”), kendi araç çizgiyi yeniden çizer ve dosyada kalır. Resim `ofis-paleti-favori`.

## 3. Sınırlar

- Kendi aracın girdisi tamdır (çalıştığı çizimin değerleri); eksik alanı kullanıcıya soran yarım tarif yok.
- Kendi araçların yeniden adlandırılması, silinmesi ve sıralanması pencerede yok; dosya elle düzenlenebilir.
- Kişisel palet Python/MCP'den okunmaz (kullanıcı tercihidir, belge değildir).
