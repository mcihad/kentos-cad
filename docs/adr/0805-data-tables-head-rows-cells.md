# ADR 0805: Veri tablosu: başlık bandı, satır çizgisi, hücre ve sürüklenen sütun

- **Durum:** kabul edildi (2026-10-30), dal `launcher-zarif`.
- **İstek:** sahibin sözü: “DataGrid veya datatable üzere de iyice çalışmanı istiyorum. Başlıkları falan dikkat edersen tamamıyla içerikle bütünleşmiş
  ve satır gibi gözüküyor ayrıca daha iyileştirme ve daha estetik hale getirme güzel olur”.
- **Bağlam belgesi:** DESIGN.md §5.2, §7.5, §7.6, §7.13; [ADR 0804](0804-one-control-height-cells-and-frames.md).
- **Web:** bir şey değişmez (ADR 0228); TODOS.md §0.2. **`.kcad`, komutlar, Python, MCP:** değişmez.
- **Bağımlılık:** yeni dış kitaplık yok.

## Bağlam

`Table`'ın başlığı, panelle aynı zeminde, 3 piksel iç boşluklu, soluk ve küçük bir yazıydı: altında çizgi yoktu; ilk satır gibi okunuyordu. Sütunlar 8 piksel
boşlukla ayrılıyordu, hiçbir yerde çizgi yoktu; uzun değer sütunun sınırında kesiliyordu, sayı son basamaklarını sessizce yitiriyordu. Satır yalnız tıklanabiliyorsa
aydınlanıyordu. Düzenlenen hücre satırın içinde küçük bir kutuydu. Yatay kayan sanal tabloda dikey kaydırma çubuğu içeriğin sağında, ekranın dışında kalıyordu; yatay
çubuk son satırın üstüne biniyordu. Sütun genişliği değiştirilemiyordu. Metraj, Hesap pencereleri, Blok çizelgesi, Sözlük ve on kadar pencere başlığı elle kuruyordu:
her biri ayrı görünüyordu.

Başvurulan yol (30 Ekim 2026'da okundu):

- IBM Carbon, *Data table*: başlık satırı “should always match the row size of the table”; sıralama “indicated with an arrow icon on hover and when a column is
  sorted”, sıralanmamış ok “only visible on hover”; “row hover state should always be enabled”. <https://carbondesignsystem.com/components/data-table/usage/>
- Apple, *Lists and tables*: başlığa basmak sıralar, ikinci basış yönü çevirir; “Let people resize columns … to help them concentrate on different areas or reveal
  clipped data”. <https://developer.apple.com/design/human-interface-guidelines/lists-and-tables.md>

KentOS'un çıkardığı: tablo ekrandaki bir çizelgedir, paftadaki kapı ve mahal çizelgesi gibi: başlık bandı, altında daha kalın çizgi, satırlarda ince çizgi, sayılar sağda.
Vurgu tablonun içinde harcanmaz; tek cesur şey başlıktır.

## Karar

### 1. Başlık

Başlık satırla aynı yüksekliktedir (`metrics::row`) ama satır gibi değildir: kendi bandında (`Tokens::table_head`: panel başlığından bir ton belirgin), yazısı yarı kalın ve ana
metin renginde (`table::title`), altında `border_strong` çizgisi. Başlıklı iki sütun arasında bandı boydan boya geçen çizgi; son sütun bandın sonundan önce bitiyorsa ondan sonra da.
Başlığı olmayan sütunda (renk örneği, ikon) çizgi yoktur. Ağacın başlığı aynıdır.

Sıralanabilir sütunun **bütün başlık hücresi** basılır, imleç üstündeyken aydınlanır. Ok sıralı sütunda vurgu renginde durur, öbürlerinde yalnız imleç üstündeyken soluktur. Ok için yer
hep ayrılır ve yazının sütunun dışına bakan yanındadır: sağa hizalı başlığın yazısı sayıların hizasından ayrılmaz, sıralanınca kaymaz. Sığmayan başlık “…” ile biter, tamamı ipucundadır.

### 2. Satır

`table::RowBox` (ağacın satırı da): en az `metrics::row`, hücreler dikeyde ortalı, altında `Tokens::rule` çizgisi (ağaçta yok). İmleç üstünde, tıklanmasa da aydınlanır (`layer(0.04)`,
kendi zemini varsa üstüne); seçili satır `selection`, imleç üstünde `selection_hover`. İmleç ok kalır: masaüstü listesidir, bağlantı değil. Bir satır düğme ve kapların yerine tek bileşendir.

### 3. Hücre

Hücreler bitişiktir, payı kendilerindedir (`cell_pads`: içeride 6, tablonun iki ucunda 10). `Cell` payını kendisi bilir ve sütunu çizgiden çizgiye doldurur (çizginin pikseli hariç): düzenlenen
değer, koşullu biçimin ve kurala uymayan değerin zemini (`Cell::tint`). Düzenleyicinin yazısı hücrenin yazısının yerinde başlar: düzenlemeye geçince kaymaz. Hücrenin tamamı çift tıklanır,
boş değer de.

`table::text` ve `table::number` tek satırdır, sığmazsa “…” ile biter: bir sayının son basamakları sessizce gitmez.

### 4. Izgara

`Table::grid()`: sütun çizgileri satırların arasından da geçer; hücre hücre okunan tablolar için: Öznitelik tablosu, Kaynağın tümü, birleşik görünüm, Noktalar, Koordinat listesi, Metraj.
Öbürleri yalnız satır çizgilidir.

### 5. Sütun genişliği

`Column::resizable(on_resize)` sağ kenarı sürükletir, `Column::on_fit(message)` çift tıkla içeriğe sığdırır. Tablo durum tutmaz, genişlik çağıranındır; birimi `Column::width`'inkidir
(`typography::unscaled`), sürüklenen piksel geri tam gelir. En dar 24 piksel. Tutulan kenarın çizgisi vurgu rengindedir. Öznitelik tablosu, Kaynağın tümü ve birleşik görünüm oturum boyunca saklar
(`FeaturesPanel::widths`, katman ve sütunla); bir sürükleme çift tıkın yarısı sayılmaz.

### 6. Kaydırma

Yatay kayan tablonun dikey çubuğu görünen kısmın sağ kenarındadır (`VirtualList::bar`, `viewport`); yatay çubuk kendi yerini alır, son satırın üstüne binmez. Başlık dikey kaydırmada yerindedir.
Band ve çizgiler panelin kenarına dek gider (`min_width`).

### 7. Tek başlık her yerde

Elle kurulan başlıklar `table::band` ve `table::title`'a geçti (katman birleştirme ve çevirme, standart, profil, kaynak durumu, alt ağlar, bul ve değiştir, sensörler, senaryo, katlar, Duvar stilleri, Alanlar,
Alan eşleme). Düzenlenmeyen tablolar `Table` oldu: Metraj (sabit başlık, sanal satır), Blok çizelgesi, Sözlük, Hesap sonuçları. Hesap tablosu `band` ve `Cell` ile: yazılabilir hücre gömük zeminde, sabit olan düz.

## Sonuçlar

- Başlık bir bakışta başlıktır; açık ve koyu temada aynı hiyerarşi.
- Uzun değer okunur: sütun açılır, “…” kesildiğini söyler.
- `style::button::header` kalktı.

## Sınırlar

- Klavyeyle sütun boyutu ve sıralama yok (Tab başlığa gitmez); tablonun klavyesi ayrı iştir.
- Düzenlenen alanlardan oluşan form ızgaraları (Duvar stilleri, Alanlar, Alan eşleme, Katlar) yalnız başlığı aldı; satırları hâlâ kutulu alanlardır.
- Sütun genişlikleri dosyaya ve yerleşime yazılmaz: oturumluktur.
- Elle gerçek pencerede denenmedi; doğrulama gerçek olaylı testler (`widget/table/tests.rs`) ve ekran resimleriyledir.
