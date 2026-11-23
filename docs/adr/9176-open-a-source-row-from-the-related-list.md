# ADR 9176: Kaynaktaki alt kayıt ilişkili listeden kendi formuyla açılır

- **Durum:** kabul edildi (2026-11-23); adım VY-75. Plan: [plan/veri.md](../../plan/veri.md) VY-75.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”; 23 Kasım: “küçük kalanlarla”.
  Değerlendirmenin 22. maddesinin kalanı ([ADR 9165](9165-related-rows-from-the-source.md) §4): kaynak satırının
  kendi formunu ilişkili listeden açmak.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam ve araştırma

- ADR 9165'ten beri veri katmanı üzerindeki formun ilişkili listesi, alt veri katmanının satırlarını kaynaktan okur.
  Bu satırlar çizimde değildir. Listede “kaynakta” diye görünüyorlardı ama açılamıyorlardı: araçların yerinde soluk
  bir “kaynakta” yazısı vardı.
- Tablonun satırı formla açılabiliyordu:
  - düzenlenen katmanda satır oturuma alınıp kaydı açılır ([ADR 9149](9149-form-edit-from-a-table-row.md));
  - salt okunur bağlantının satırı okunup yalnız gösterilir (ADR 9159).
- Salt okunur gösterilen üst kaydın ilişkili öğesi, “Aç” dahil her eylemi “Önce kaydı kaydedin” diye reddediyordu.
  Kaydın çizimde kimliği olmadığı için kaydedilmemiş sanılıyordu.
- **Araştırma.** QGIS 3.44 kullanıcı kılavuzunun “Connecting and Editing Data Across Layers” bölümüne bakıldı
  ([1-N ilişkinin formları](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/joins_relations.html),
  okundu 23 Kasım 2026). Masaüstü CBS'lerde 1-N ilişkinin alt kayıtları, üst kaydın formunda tablo ya da form
  görünümünde gösterilir.
  - Alt kayıtları değiştirmek alt katmanın düzenleme kipini ister.
  - Bu kip üst kaydın formundan açılsa da alt katmanındır.

## §2 Karar

1. **Aç kaynaktaki satırda da vardır.**
   - Listede “Aç” düğmesi görünür; tablo ve kart görünümünde aç simgesi ve yanında “kaynakta” yazısı.
   - Göster, Bağı kopar ve Sil kaynaktaki satırda yoktur, çünkü satır çizimde değildir.
2. **Açılış** (`open_related_row`).
   - Satır, anahtarıyla kaynaktan okunur; okuma arayüzün iş parçacığında yapılmaz.
   - Alt katmanın formu üst kaydın formunun üstünde açılır; üstteki altta bekler (ADR 0440 §7). Vazgeç üst kaydın
     formunu ilişkili listesiyle geri getirir.
   - Öğe alt kaydı değiştirmeye izin veriyor ve alt katman düzenleniyorsa satır düzenleme oturumuna alınır. Form ile
     oturum tek bekleyen değişikliği paylaşır. Listeye dönünce satır çizimin nesnesi olarak bir kez görünür.
   - Aksi hâlde satır yalnız gösterilir ve formun başı nedenini söyler:
     - alt katman düzenlenmiyorsa: “‘bakim’ düzenlemeye açık değil: kayıt kaynaktan okunup gösteriliyor,
       değiştirilmez. Değiştirmek için katmanın düzenleme oturumunu başlatıp kaydı listeden yeniden açın.”;
     - üst kayıt yalnız okunuyorsa: “Altta bekleyen ‘Mahalle formu’ yalnız okumaya açık; ondan açılan bu kayıt da
       değiştirilmez.”;
     - öğenin başka bir reddinde (alt katman kilitli, rol) o reddin kendi sözü.
   - Salt okunur bağlantının sözü (ADR 9159) ve yalnız okumayı sunan öğenin sözü değişmedi.
   - Arada çizime alınmış satır, çizimin nesnesi olarak açılır.
   - Yanıt geldiğinde üst kaydın formu kapanmışsa ya da başka bir form açılmışsa yanıt bayattır; hiçbir şey açılmaz.
3. **Salt okunur gösterilen kayıt kaydedilmiş sayılır.** Kaydın kaynağında durduğu için ilişkili öğesi “Aç” ve “Git”
   eylemlerini sunar. Yazan eylemler, kaydın yalnız okunduğunu söyleyerek kapalı kalır.

## §3 Kanıt

- **Masaüstü** (`gis/forms/source_rows_tests.rs`), değerlendirmenin GeoPackage'ıyla:
  - **Raporun düzeni** (mahalleler salt okunur):
    - M1'in listesinde iki “Aç” var.
    - B02'nin “Aç”ı Bakım formunu açıyor: fid 2, kod B02, bedel 100, yalnız okunur, neden üst formun sözüyle.
    - Mahalle formu altta bekliyor; bakım çizime alınmadı.
    - Vazgeç M1'in formunu iki bakımıyla geri getiriyor.
  - **Mahalleler düzenleniyor, bakım düzenlenmiyor:** B01 gösteriliyor, form düzenleme oturumunu başlatmayı söylüyor.
    Bakım çizime alınmadı.
  - **İkisi düzenleniyor:** B02 oturuma alınıyor ve formu onu değiştirebiliyor. Dönünce M1'in listesinde B02 bir kez,
    çizimin nesnesi olarak görünüyor.
  - Form, ilişki, satır hedefi ve ortak masaüstü sınamaları geçiyor (197).
- **Resimler** `.run/gui-evidence/kullanim-9176/` altında:
  - `01-liste-ac`: M1'in listesi, her bakımda Aç;
  - `02-kaynaktan-acilan-form`: B02'nin Bakım formu, Mahalle formu altta.
  - `kullanim-9165/01-kaynaktan-iliskili` tablo görünümündeki aç simgesini gösteriyor.

## §4 Kalan

- 500'den çok alt kaydı olan kayıtta sayfalı okuma (ADR 9165 §4).
- Gösterilen kayıtta düzenleme oturumunu başlatıp aynı formu yazılabilir açmak (bugün kayıt listeden yeniden açılır).
- Gerçek pencerede deneme; web (TODOS.md §0.2).
