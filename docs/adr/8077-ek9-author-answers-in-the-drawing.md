# ADR 8077: EK-9'un müellif cevapları çizimde — basılan form ve yayın aynı cevabı okur

- **Durum:** yapıldı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-14 (B14'ün kalanı: "formun paftası
  müellifin cevabını göstermiyor"), [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-08.
- **Bağlam:** ADR [8074](8074-ek9-from-the-written-file.md) (EK-9 yazılmış dosyadan; 1, 7, 10. sorular ve bellekteki
  dosyada 18. soru müellifin incelemesine kalır), [8075](8075-bim-permit-package.md) (BIM ruhsat paketi: cevaplar yalnız
  yayının girdisindeydi), [8068](8068-fire-performance-material-declarations.md) (beyanlar çizimde, kendi geri alma
  adımıyla), ADR 8054 §10 (modül verisi sürümleri).
- **Sorun:** 8075'te müellifin Evet/Hayır'ı ve notu yalnız yayın penceresinin girdisinde duruyordu. Basılan form (çizimdeki
  EK-9 tablosu, FR/MKF paftası) bu soruları boş gösteriyordu; pencere kapanınca cevaplar kayboluyordu; Python ve MCP
  yayını çağıran her seferinde cevapları yeniden vermek zorundaydı.

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Hazırlama Yön. Md. 15, EK-9 Tablo 9.1 | Her model için müellifin doldurduğu kontrol formu; cevap "Evet/Hayır", açıklama sütunu | Form bir belgedir: müellifin cevabı formda yazılı durur, yayın onu taşır |
| e-PYS Yön. Md. 5/1–2 | Dosyanın doğruluğundan müellif sorumlu | Cevap müellifin beyanıdır; KentOS doldurmaz, varsayılan vermez |
| ADR 8068'in beyan kuralı | "Varsayılan yoktur: null değeri kaldırır" | Aynı kural: null cevabı kaldırır, boş not notu kaldırır |

## §2. Karar

- **Cevaplar çizimde:** `settings.arch.formAnswers` (soru numarası 1–21, Evet/Hayır, en çok 300 karakterlik tek satırlık
  not; soru başına bir cevap). `FormAnswer::problem` ve `list_problem` ölçer.
- **Komut:** `cad.form.answer` v1 ("Kontrol formunun cevapları"): her değişiklik bir soru — `yes` true/false ya da null
  (cevabı ve notu kaldırır), `note` verilmezse var olan kalır, boş metin kaldırır, iki ucundaki boşluk alınır. Retler
  sırasıyla: `nothing_to_set`, `invalid_input` (1–21 dışı numara, iki kez verilen soru, null ile not, uzun ya da denetim
  karakterli not), `invalid_revision`, `revision_conflict`. Biri reddedilirse hiçbiri yazılmaz; değişiklik yoksa geri alma
  adımı da yok. Tek geri alma adımı (`ArchOp::FormAnswers`). Kimlik kuralı (küçük harf, `alan.nesne.eylem`) gereği adı
  `cad.form.answer`'dır.
- **Hangi soru alır:** komut cevabı tutar; okuyan formun kuralıdır (`dijital_proje.form.cevapli`): yalnız incelemeye kalan
  sorunun resmî cevabı müellifinkidir, notu "<not> (müellif)". KentOS'un dosyadan cevapladığı soruya verilen cevap tutulur
  ama okunmaz: kanıt dosyanındır.
- **Basılan form:** `form.tablo(ifc, cevaplar)` çizimin cevaplarını yazar; çizimdeki tablo bellekte yazılan dosyadan
  kurulduğundan 18. soru (teslim edilen dosya) de müellife kalır.
- **Yayın:** `file.publish.sheets`'in `bim` bölümü çizimin cevaplarını okur; çağrıda verilen cevap aynı sorunun çizimdekinin
  önüne geçer. Formun kaydı (`kalite-kontrol-formu.json`) birleşmiş cevapları yazar. Yayın çizimi değiştirmez.
- **Masaüstü:** yayın penceresinin BIM anahtarı çizimin cevaplarını yükler; Evet/Hayır ve not yazılırken pencerede taslaktır
  (her tuş bir geri alma adımı olmasın diye); Denetle ve Yayınla önce taslağı `cad.form.answer` ile çizime yazar (değiştiyse
  tek adım), sonra denetler.
- **Dosya:** mimari verisi sürüm 10 (`ARCH_WITH_FORM_ANSWERS`); cevabı olmayan çizim eski sürümüyle bayt bayt aynıdır, cevabı
  olan çizimi eski KentOS "yeni sürüm" diye reddeder (ADR 8054 §10). Belge şeması değişmez.
- **Proje ayarları penceresi** (Proje › Mimari) kaydederken binanın yangın koruma sınıfını siliyordu (`performance: None`);
  artık o ve formun cevapları projede olduğu gibi kalır.

## §3. Uygulama

- Sözleşme: `ArchSettings::form_answers`, `FormAnswer::problem`/`list_problem`, `MAX_FORM_NOTE`;
  `kentos_commands::cad_arch_form_answers` (`ArchFormAnswers`, `FormAnswerChange`, `ArchFormAnswersDone`).
- Belge: `kentos_domain::form_answers` (`form_answers`, `set_form_answers`), `history::ArchOp::FormAnswers`.
- Komut: `kentos_native_application::arch_form_answers`; katalog, yetenekler, masaüstü komut listesi, ajanların yıkıcı listesi
  (`agent_tools::DESTRUCTIVE`), komut geçmişinin adı.
- `.kcad`: `ARCH_WITH_FORM_ANSWERS = 10`, `uses_form_answers`, okumada eski sürüme ret.
- Standart paket 0.8.0: `form.cevapli`, `form.tablo(ifc, cevaplar)`; `bim.denetle` aynı birleştirmeyi kullanır.
- Masaüstü: `ifc_facts::form_rows_of(planned, answers)`, `App::permit_bim_keep`, BIM anahtarında yükleme.
- Başsız yayın: `publish_bim::package` çizimin cevaplarını birleştirir.

## §4. Sınamalar

- Ortak durumlar: `fixtures/commands/v1/cad.form.answer.json` (26 durum; `scripts/fixtures/form_answers_cases.py`,
  KentOS kodu olmadan): yazma, notu tutma/değiştirme/kaldırma, soru sırası, kırpma, 300 karakter, Türkçe harf, dosyadan
  cevaplanan soruya cevap, değişmeyen iki çağrı, her ret ve "biri geçersizse hiçbiri".
- `kentos-kcad` (`arch_form_answers`): cevaplar sürüm 10 ile gidip gelir; cevapsız çizim olduğu gibi.
- Paket (`test_form.MuellifCevaplari`): inceleme sorusu müellifin cevabını ve notunu alır, cevaplanmayan boş kalır, dosyadan
  cevaplanan soruya verilen cevap tabloyu değiştirmez.
- Masaüstü: `the_author_s_answers_are_the_drawing_s_and_the_printed_form_shows_them` (Denetle'ye kadar taslak, sonra
  çizimde; basılan formda "Evet" ve "(müellif)"; yeni anahtar yükler; geri alma tek adımda kaldırır); proje ayarları
  penceresi beyanları ve cevapları tutar (`project_form_preserves_…`).
- Başsız: yazar gibi doldurulmuş bina, cevapları `cad.form.answer` ile çizimde, yayına cevap verilmeden yayımlanır;
  formun kaydı cevapları taşır.
- Resimler: `.run/gui-evidence/ek9-cevaplar-8077/` (basılan form, sorular ve cevaplar).

## §5. Açık işler

- **Cevabın eskimesi:** model değişince bir cevap eskiyebilir (10. sorunun alan tabloları gibi); cevap modelin revizyonunu
  taşımıyor, uyarı yok.
- **Bağımsız Python `.kcad` okuyucusu** (`tools/kcad/kcad.py`) mimari verisi 1–10'u bilmiyor (`MODULE_VERSIONS` arch 0):
  ADR 8068'den beri her mimari sürüm artışında güncellenmedi. Python SDK'nın tipli katmanından ortak durumlar
  (`test_command_cases`) mimari verisi ≥1 olan ilk belgede duruyor; bu dilimin durumları da oradan geçemiyor. Ayrı iş:
  TODOS.md.
- 2+1'de paket ve MCP'nin uçtan uca sınaması B14'ün kalanıdır.
