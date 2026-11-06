# ADR 1110: Pafta penceresinin dış başvuru katmanları için kendi ezmesi

- **Durum:** kabul edildi (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-14`'ün açık bıraktığı “pafta nesnesinin
  kendi katman override'ı” (ADR 1060 §12). İlke “Performance First”; alt madde tek parçada.
- **Bağlam belgesi:** ADR 0314 §1 (haritanın katman başına geçersiz kılması `layerOverrides`), 0348 §2 (dış başvurunun
  katmanları grubunda, VISRETAIN açık gibi), 1060 §5 (Katmanları kaynağa döndür, üç düzey override), 1100 §5 (“Bu değer
  nereden geliyor?”).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 1060 §12 pafta nesnesinin dış başvuru katmanları için kendi ezmesini “veri modeli yok, kurulmadı” diye bırakmıştı.
Kod doğrulandığında bu söz eksik çıktı: ADR 0314 §1'in harita öğesi `layerOverrides`'ı (dondur, yazdır, renk, çizgi
tipi, desen, kalınlık; katman kimliğiyle) AutoCAD'in VP Freeze / VP Color / VP Linetype / VP Lineweight / VP Plot
karşılığıdır ve dış başvurunun katmanları da çizimin katman ağacında kimlikli düğümlerdir. Açık olanlar şunlardı:

1. **Kalıcılık kanıtlanmamıştı.** Yeniden yükleme, Yolu değiştir, Boşalt ve Katmanları kaynağa döndür'ün pencere
   ezmesini koruduğunu söyleyen bir test ya da ortak durum yoktu.
2. **İnceleme söylemiyordu.** `cad.xrefs.list`'in `review`'i yalnız katman durumlarını adlandırıyordu; bir pafta
   penceresinin dış başvuru katmanını dondurduğu ya da başka renkte bastığı görünmüyordu.
3. **Değerin kaynağı eksikti.** ADR 1100 §5'in açıklaması pencerede başka çizilen katmanı söylüyordu ama pencerenin
   **dondurduğu** katmanı söylemiyordu (dondurulan katman `map_layers`'ın listesinde hiç yoktu); dış başvurunun
   yerleştirmesi seçilince kaynağın, çizimin ve pencerenin üç düzeyi ayrı görünmüyordu.

### Araştırma (resmî belgeler)

- AutoCAD “To Override Layer Properties in Layout Viewports” (AutoCAD Architecture 2017/2020/2022 belgeleri): Katman
  Özellikleri Yöneticisi bir yerleşim penceresi etkinken VP Color, VP Linetype, VP Lineweight, VP Plot Style ve VP
  Freeze sütunlarını gösterir; ezme yalnız o pencerededir; VPLAYEROVERRIDESMODE 0 ezmeleri geçici olarak gizler.
  Aynı sayfa “VISRETAIN 0 iken dış başvuru katmanlarındaki özellik ezmeleri korunmaz” der
  (help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Architecture/files/GUID-8909084C-770C-41A3-A782-CCABE9DC433A.htm).
- AutoCAD “About Xref Layer Property Overrides” (ADR 1060'ta anılan GUID-5070C6C7-EAEF-45F0-ABB8-F894E585C54D):
  VISRETAIN/VISRETAINMODE çizimin kendi katman değişikliğinin yeniden yüklemede kalıp kalmadığını belirler.
- Autodesk forum kayıtları (resmî değil, risk notu): VISRETAIN 0 iken yeniden yüklemenin VP Freeze/VP Color'ı da
  sildiği, VISRETAIN 1 iken kaynağa döndürmenin pencere ezmelerine dokunmadığı bildiriliyor; VPLAYER › Reset bir
  penceredeki ezmeyi siler.
- **KentOS kararı:** pencere ezmesi katmanın kimliğine bağlıdır ve hiçbir dış başvuru işlemi onu değiştirmez (KentOS
  zaten VISRETAIN 1 gibi davranır; VISRETAIN 0 karşılığı yoktur). Çizimin kendi ezmesi (`resetLayers` döndürür),
  pafta penceresinin ezmesi ve katman durumu üç ayrı düzeydir.

## Karar

### 1. Veri: ADR 0314'ün harita ezmesi, yeni şema yok

Pafta penceresinin dış başvuru katmanı için kendi ezmesi, harita öğesinin `layerOverrides` satırıdır; `layer` dış
başvuru grubunun altındaki katmanın (ya da grubun kendisinin: bütün dış başvuru o pencerede donar) kimliğidir. Yazma
`cad.sheets.edit` (`setViewport` ya da öğe özellikleri) ile, okuma pafta çekirdeğinin `map_layers`'ıyla olur; ekran,
PDF ve başsız kitap aynı işlevi çağırır. `.kcad` şeması değişmez (pafta kitabı opak, ADR 0251).

**Kalıcılık (koddan doğrulanan ve sınanan):** dış başvurunun katmanları yolla eşlenir; yolu çizimde olan düğüm kimliğini
korur (`take_layers`, `Same::Skip`); yeniden yükleme ve Yolu değiştir hiçbir katmanı silmez (`put_nodes` yalnız ekler
ve çizimin stilini kaynağınkiyle aynı tutar), Boşalt katmanlara dokunmaz, Katmanları kaynağa döndür yalnız çizimin
düğümlerini yazar; pafta kitabını hiçbiri yazmaz. Bu yüzden pencere ezmesi bütün bu işlemlerden sonra yerindedir.
Kaldır (detach) katman grubunu, üzerinde çizilen kalmadıysa siler; o zaman ezme kimliği olmayan bir katmanı adlandırır
ve hiçbir şey çizmez (AutoCAD'de de kaldırılan başvurunun katmanı ve VP ezmeleri gider).

### 2. İnceleme (`cad.xrefs.list` `review` → `viewportOverrides`)

`review: true` her dış başvuruya `viewportOverrides` ekler: pafta kitabının sırasıyla her paftanın, her harita öğesinin,
her `layerOverrides` satırından dış başvurunun grubundaki düğümü adlandıranlar. Satır: `sheet` (paftanın adı), `item`
(harita öğesinin kimliği, `setViewport`'un aldığı), `viewport` (öğenin adı), `layer` (düğümün çizimin ağacındaki yolu:
`Dış başvuru: Plan / Mimari / Duvar`) ve ezmenin alanları (`frozen`, `plot`, `color`, `lineType`, `linePattern`,
`lineWeight` µm). Çizimin kendi katmanının ve olmayan kimliğin ezmesi listelenmez. `review` yokken cevap eskisiyle bayt
bayt aynıdır.

### 3. Değerin kaynağı (ADR 1100 §5'in genişlemesi)

- Pencerenin kendi ezmesi bir katmanı **donduruyorsa** açıklamanın her parçasında (renk, çizgi tipi, kalınlık) o pencere
  `Pafta · pencere: dondurulmuş` olarak söylenir (önceden hiç söylenmiyordu).
- Dış başvurunun **yerleştirmesi** seçilince yeni “Dış başvuru katmanları” parçası: grubun katman sayısı, çizimin
  görünüşü düzeyi (“kaynağınkinden ayrıldıysa yeniden yüklemede kalır”), her pencere ezmesi (`A-101 · Plan: Dış
  başvuru: Plan / Duvar — dondurulmuş, renk #FF0000`) ve iki sıfırlamanın erimi: “Katmanları kaynağa döndür: çizimin
  ezmesi” (grubun katmanları) ve “pencerelerin kendi ezmeleri (dış başvuru işlemleri dokunmaz)”.
- Ezmenin sözü tek yerdedir (`xrefs_review::override_words`): pencere, inceleme ve Denetçi aynı cümleyi kurar.

### 4. Arayüz (masaüstü)

- Dış başvurular penceresinde seçili başvurunun ayrıntısı “Pafta pencerelerinin kendi ezmeleri (yeniden yükleme, Yolu
  değiştir ve Katmanları sıfırla dokunmaz)” başlığıyla her ezmeyi bir satırda gösterir.
- Katmanları sıfırla'nın iletisi “Katman durumları ve pafta pencerelerinin kendi ezmeleri değişmedi” der.
- Denetçi'nin “Bu değer nereden geliyor?” bölümü §3'ü gösterir. Ezmenin kendisi pafta tasarımcısının Denetçi'sindeki
  haritanın Katmanlar bölümünden (ADR 0314 §1) verilir. Oradaki katman listesi katmanları yalnız adlarıyla veriyordu:
  dış başvurunun Duvar'ı çizimin kendi Duvar'ından ayırt edilemiyordu; artık ağaçtaki yollarıyla verir (`Dış başvuru:
  Plan / Mimari / Duvar`, koordinat listesinin katman seçimi de aynı listeyi kullanır).
- Yeni ikon gerekmedi.

### 5. Komutlar ve otomasyon

Yeni komut yok: `cad.xrefs.list` v1'e `XrefInfo.viewportOverrides` (`XrefViewportOverride`) eklendi; eski girdi ve
cevap aynı. Katalog örneği, TS türleri ve Python SDK üretildi; MCP komutu katalogdan geçirir.

### 6. Performans

İnceleme pafta kitabını bir kez okur (yalnız `review` ile); başvuru başına grubun düğümleri bir kez gezilir. Sıcak yola
(çizim, kenet) dokunulmadı.

### 7. Kapsam dışı

- Bağla › Ekle (Bind › Insert) dış başvuru katmanlarını çizimin aynı yollu katmanlarına katar; o katmana işaret eden
  pencere ezmesi kimliği gitmiş katmanı adlandırır ve kendiliğinden taşınmaz (pafta kitabının yazımı geri alma
  adımının dışındadır, ADR 0314 §2). Bağla › Bağla (Bind) kimlikleri korur, ezme kalır. Taşıma sonraki iş.
- VPLAYEROVERRIDESMODE karşılığı (ezmeleri geçici gizleme) yoktur.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/xref.rs` (`XrefViewportOverride`, `XrefInfo.viewport_overrides`).
- Uygulama: `crates/native/application/src/xrefs_review.rs` (`group_paths`, `viewport_overrides`, `override_words`),
  `xrefs.rs` (listeye bağlama), `appearance_origin.rs` (dondurma ve dış başvuru katmanları parçası).
- Masaüstü: `apps/desktop/src/xrefs_review_view.rs`, `xrefs_window.rs`.

## Doğrulama

- **Bağımsız başvuru:** `python3 scripts/fixtures/xref_command_cases.py --check` — `cad.xrefs.list` 46 durum (yeni 9:
  dondurma, renk/tip/kalınlık/yazdırma, desen, çizimin kendi katmanı ve olmayan kimlik, grup ve iki paftanın sırası,
  `review`'siz, plan, Katmanları kaynağa döndür'den sonra, Boşalt'tan sonra).
- **Kabul testi** (`crates/native/headless/tests/headless/xref_network.rs`): iki paftanın iki penceresi Plan'ın katmanını
  biri dondurur biri başka renkte basar; yeniden yükleme, Yolu değiştir, Katmanları kaynağa döndür ve paketin başka
  klasörde açılmasından sonra ezmeler yerinde ve incelemede; değerin kaynağı dondurmayı söyler.
- Test sonuçları ve çalıştırılmayanlar: plan/cad-advanced.md CAD-ADV-14'ün 6 Kasım 2026 paragrafı.

## Web'e ertelenenler

`viewportOverrides`'ın TS türü üretildi; web'in Dış başvurular penceresinde ve Denetçi'de gösterilmesi.
