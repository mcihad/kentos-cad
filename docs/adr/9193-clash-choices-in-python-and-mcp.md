# ADR 9193: Python ve MCP çakışmaları alan alan okur ve seçimle kaydeder

- **Durum:** kabul edildi (2026-11-23); adım VY-92. Plan: [plan/veri.md](../../plan/veri.md) VY-92.
- **İstek:** sahibin sözü (23 Kasım 2026): “Yarım kalan maddeleri tamamla”. Değerlendirmenin 26. maddesinin kalanı (ADR
  9148 §4): “Python ve MCP. `cad.layers.session` çakışmayı yeni sözle söylüyor; alan alan seçim orada `force` ya da
  rollback/take ile yapılıyor, seçim işlemi yok.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `cad.layers.session` v1'e `conflicts` eylemi ve `save`'e `choices` eklendi. Yanıtta `conflicts` var. Yeni ret
  kodu `clash_choice`. Katalog, TypeScript türleri ve `kentos.cad` yeniden üretildi; MCP aracı katalogdan gelir.

## §1 Bağlam

- ADR 9148 masaüstünde çakışmayı alan alan soruyordu: ilk değer, benim değerim, tablodaki değer ve seçim. Seçilenleri
  kaydet satırları seçildiği gibi kuruyor ve yeniden kaydediyordu.
- Komut ise yalnız reddediyordu: `rows_changed` ve bir söz. Python ya da bir yardımcı çakışmayı yapılandırılmış olarak
  okuyamıyordu. Seçenekleri ya hepsinin üstüne yazmak (`force`) ya da değişikliği bırakıp satırı yeniden almaktı.
- Seçimi uygulayan adım masaüstündeydi (`data_conflict_settle`).

## §2 Karar

1. **`conflicts`.** Kaydetmenin şimdi karşılaşacağı çakışmaları söyler, hiçbir şey yazmaz.
   - Kayıt gibi okunur: yazmanın planı kurulur, alınmış satırlar bir işlemin içinde okunur, işlem bırakılır
     (`data_write::clashes_of_group`). Yazma planı yazmayla ortaktır (`plan_group`).
   - Her çakışan satır için katman, anahtar ve nesnenin kalıcı kimliği verilir. Ardından:
     - çakışan her alan ilk, benim ve tablodaki değeriyle (boş değer yazılmaz);
     - şeklin iki yanda taşınıp taşınmadığı;
     - satırın nerede silindiği (`theirs`: tabloda, `here`: burada);
     - tabloda silinmiş satırın yeni satır olarak yazılıp yazılamayacağı (`mayAdd`, bağlantının ekleme hakkı).
2. **`save`'in `choices`'ı.** Her çakışan satır için katman ve anahtar, alan başına taraf (`mine`, `theirs`), şekil
   için taraf, bir yanda silinmiş satır için `row`. `row: mine`, tabloda silinmiş satırı yeni satır olarak yazar ve burada
   silinmiş satırı yine siler; `theirs` değişikliği bırakır ve satırı tutar.
   - **Uygulama, masaüstüyle aynı adım** (`data_clash::settle`, Seçilenleri kaydet de onu çağırır):
     - nesne seçimden kurulur, tek geri alma adımıdır (“Çakışmayı çöz”);
     - satır bundan sonra tablodan okunanla karşılaştırılır;
     - yeni satır olarak yazılacak satırın alınmışlığı unutulur, bırakılacak satır bırakılır.
   - Adım komutun kayıtlı adımıdır, günlükte komutun satırı olarak durur. Oturumun satır tabanı geri alınmaz, satır
     almanın geri alınmadığı gibi.
   - Ardından grubun yazımı seçilmiş çizimden yeniden kurulur ve yazılır; tablo bir kez daha denetlenir. Tablo yine
     değiştiyse `rows_changed` döner, seçim uygulanmış kalır (masaüstünde de öyle).
   - Yanıtın `conflicts`'i çözülen çakışmalardır.
3. **Retler.**
   - Çakışmayan bir satırı, alanı, şekli ya da iki yanı duran satırda `row`'u seçmek `clash_choice` olur, `choices[i]`
     yoluyla. Yeni satır olarak yazılamayacak silinmiş satırda `mine` de öyledir.
   - Seçilmemiş bir çakışma `rows_changed` olur: söz çakışmayı ve seçilmeyenleri sayar (“Seçilmeyen: satır 3 kullanim.”).
     Hiçbir şey uygulanmaz.
   - Yalnız tek yolu olan satır sorulmaz, bırakılır: iki yanda silinmiş ya da bağlantının ekleyemediği tabloda silinmiş
     satır.
   - `force` eskisi gibi hepsinin üstüne yazar.

## §3 Kanıt

- **Ortak durumlar** (`scripts/fixtures/layer_session_command_cases.py`, 34'ten 38 duruma; `--check`):
  - çakışmalar alan alan okunuyor ve tabloya hiçbir şey yazılmıyor; benimkini seçip kaydetmek tabloda “Pazar”ı ve öbür
    değişikliği (“Yeni”) bir arada yazıyor;
  - tablodakini seçince satır tablodaki gibi kalıyor, yazılacak bir şey kalmıyor;
  - boş seçim `rows_changed` ve seçilmeyeni söylüyor; çakışmayan alan ve satır `clash_choice`; tablo değişmiyor;
  - başkasının sildiği satır `row: mine` ile yeni satır (anahtar 11) olarak yazılıyor.
- Uygulamanın bütün ortak komut durumları (22.475) ve yetenek sınamaları geçiyor.
- **Python:** yerli modül yeniden derlendi. `cad.layers.session`'ın 38 durumu tipli Python katmanından geçiyor
  (`test_command_cases`, `KENTOS_CASES_ONLY=cad.layers.session`).
- **Masaüstü:** Seçilenleri kaydet ortak adıma geçti; çakışma, veri düzenleme ve ortak sınamalar (168) geçiyor.
- Görünür arayüz değişmedi; resim çekilmedi.

## §4 Kalan

- Uzun metinler hücrede kısaltılmadan yazılıyor (ADR 9148 §4).
- Web (TODOS.md §0.2).
