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

## §5. 2+1 dairesinin paketi (B14'ün kalanı)

Kabul akışının 2+1 dairesi (`fixtures/acceptance/v1/flat.json`, komutlarıyla çizilen) aynı beyanlarla gerçek kurala
verildi (`tests/headless/bim_package.rs::the_flat_s_declared_package_holds`). Kuralın ilk bulguları yazıcının değil,
çizimin eksikleriydi; müellifin tamamladıkları:

- Duvarların stili yok: malzeme ve sıva yok (`IfcMaterial.Name`, `WallCoveringThickness`) → dış duvarlara Dış duvar 25,
  bölmelere İç duvar 15.
- Döşemenin malzemesi ve katmanı yok (`IfcMaterial.Name`, `FloorCoveringThickness`) → Betonarme, zemine oturan diziliş.
- Katın üstünde döşeme yok: yükseklikleri ve tavan sıvası bilinmiyor (`Height`, `EavesHeight`, `CeilingCoveringThickness`)
  → katın yüksekliğinde teras döşemesi.
- Parsel modelin katmanında serbest çizgi (`digital_free_on_model_layer`) → Vaziyet katmanı.
- 5. soru: kapı ve pencerelerin adında açıklama yok (K49) → müellifin cevabı.

Tamamlanınca paket yayımlandı: beş sonuçtan dördü Tutuyor, idare Bekleniyor; formun kaydı müellifin dört cevabını taşır.

## §6. Ajanın yolu (MCP, uçtan uca)

`crates/native/mcp/tests/bim_package.rs`: gerçek MCP sunucusunda ajan, yazar gibi doldurulmuş ve ruhsat takımı yapılmış
kesit binasını açar (`fixtures/arch/v1/kesit-binasi-tam.kcad`; `KENTOS_WRITE_BIM_DRAWING=1` başsız testten yazar),
`file.publish.sheets`'i `bim` ile çağırır ve müellife kalan sorularda durdurulur (hiçbir şey yazılmaz); `cad.form.answer`
ile cevaplar, yeniden yayımlar: beş sonuçtan dördü Tutuyor, model klasörde, formun kaydında çizimin cevapları.

## §7. Cevabın eskimesi (K59)

**Karar.** Cevap dayandığı kanıtı tutar (`FormAnswer.basis`: cevaplandığı andaki KentOS notu; en çok 600 karakter, tek
satır). Formun kuralı (`form.cevapli`) sorunun şimdiki KentOS notunu dayanakla karşılaştırır: farklıysa cevap eskidir —
soru yeniden müellife kalır, resmî cevap boş, not "Müellifin cevabı bu kanıttan önce verildi: soruyu yeniden cevaplayın";
yayında `digital_form_answer_stale` hatası yayını durdurur. Dayanak söylenmemişse cevap eskimez (eski çizimlerin ve
dayanak vermeyen ajanların cevapları).

- **Neden kanıt, sürüm değil:** çizimin sürümü her değişiklikte (bir yazı, bir pafta) artar; cevabı ona bağlamak her
  düzenlemede bütün cevapları eskitirdi. Sorunun KentOS notu tam onun kanıtını söyler: 1. soruda dosyanın boyutu (0,1 MB'a
  yuvarlı), 10. soruda zonların sayısı ve durumu; 7. ve 18. sorunun notu sabittir, cevapları kanıttan eskimez.
- **Dayanağı kim verir:** yayının satırları KentOS'un kendi notunu taşır (`FormRow.basis`, Python'da `gerekce`); masaüstü
  penceresi Evet/Hayır'ı o notla yazar; ajan `rows[].basis`'i `cad.form.answer`'ın `basis`'ine verir.
- **Sınama:** paket (`test_form.MuellifCevaplari.test_kaniti_degisen_cevap_eskir`, `test_bim.…yayini_durdurur`),
  başsız (`a_stale_answer_stops_the_package`: raporun dayanaklarıyla geçer, 10. sorunun dayanağı başka olunca durur),
  masaüstü (pencerenin yazdığı cevap 1. sorunun "Dosya … MB" notunu taşır), ortak durumlar (33: dayanağın tutulması,
  kalması, değişmesi, kaldırılması, retleri).

## §8. Açık işler

- **7. sorunun eskimesi:** gelişim seviyesi notu sabit; model ne kadar değişirse değişsin cevabı kanıttan eskimez (EK-8'in
  ölçüsü KentOS'ta yok).
- **Bağımsız Python `.kcad` okuyucusu** (`tools/kcad/kcad.py`) mimari verisi 1–10'u bilmiyor (`MODULE_VERSIONS` arch 0):
  ADR 8068'den beri her mimari sürüm artışında güncellenmedi. Python SDK'nın tipli katmanından ortak durumlar
  (`test_command_cases`) mimari verisi ≥1 olan ilk belgede duruyor; bu dilimin durumları da oradan geçemiyor. Ayrı iş:
  TODOS.md.
