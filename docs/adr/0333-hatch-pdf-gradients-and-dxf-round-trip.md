# ADR 0333: Taramanın PDF'te degradesi ve DXF'te ilişkili gidiş-dönüşü

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §12 `CAD-HATCH-12`, `13` (sahibin sözü: “Hatch
  bitene kadar durma … tüm maddeler bitecek”; 2–3 maddelik grupların sonuncusu). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0186 (desenler, degrade, ilişkili tarama), 0328 (kaynak sınır, yüzün bağı), 0329 (bölgeler),
  0331 (desenler, başlangıç), 0332 (bütçe, ekranın tonu), 0252 (başsız kitap PDF'i), 0258 (kapalı çoklu çizgi), 0144 (DXF
  blokları), 0327 (DXF dışa aktarmanın girdisi).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Paftanın PDF'i taramanın düz dolgusunu ve desen çizgilerini vektör olarak yazıyordu (çizgiler çizim hattının kuralıyla:
`n·p − offset` tam aralıkta, kesikler `dashCover`'ın evresiyle, nokta 0,3 mm; ADR 0332 §3), ama degradeyi yazamıyor, “vektörle
yazılamadı” notuyla haritanın resminde bırakıyordu. Böylece aynı tarama ekranda degrade, kâğıtta resim ya da hiç oluyordu.

DXF'te ilişkili tarama ilişkisiz geliyor ve gidiyordu: okuyucu 71'i okuyordu ama yolların kaynak nesnelerini (97, 330'lar) ve
tohum noktalarını (98) atıyordu; yazıcı her taramayı `71 0` yazıyordu. AutoCAD'den gelen ilişkili tarama KentOS'ta sınırını
izlemiyor, KentOS'un ilişkili taraması AutoCAD'de izlemiyordu. Gidiş-dönüş de yalnız KentOS'un kendi çıktısıyla sınanmıştı.

## Karar

### 1. PDF'te degrade (`CAD-HATCH-12`)

Degrade, çizim hattının gölgelendiricisinin renk kuralıyla (WGSL `gradientFill`, ADR 0186 §5) bantlara bölünür ve her bant
kendi dolu şekli olarak yazılır (`kentos_plot::vectors::gradient`):

- doğrusal: doğrultusu boyunca 64 bant, her biri bölgenin dışbükey kutusuyla kesilmiş bir şerit;
- silindir: ortadan iki yana simetrik, 128 şerit (renk ortada uç, kenarda öbür uç);
- küre: 64 halka, her halka 128 dilimli dışbükey yamuklarla; dilimler dışbükey olduğundan çift-tek dolgu deliksiz kalır;
- ters: iki renk yer değiştirir; ikinci renk yoksa ana rengin açığı (gölgelendiricinin kuralı).

Bantlar taramanın bölgesiyle (halkası ve delikleri) PDF'in kırpma yolunda kesilir, deseni ve dolgusu olan taramalarla aynı yoldan.
Opak bantların arasında saç teli kalmasın diye her bant kendi renginde 0,08 mm'lik bir kenarla da çizilir (saydam bantta çizilmez:
iki kez boyanırdı). Bandın rengi bandın ortasının rengidir; gölgelendiricinin sürekli renginden farkı en çok bir bandın yarısıdır
(test `the_bands_are_the_shader_s_colours_and_cover_the_region_once`: bantların alanı bölgeninkidir, bölgedeki her örnek nokta tek bir
banttadır ve rengi gölgelendiricininkinden renk aralığının en çok 1/128'i, artı 8 bitin yarısı kadar uzaktır).

Ekranla kâğıdın ortak kuralları yazıya geçti: desenin çizgileri, kesikleri ve noktaları kâğıtta çizim hattının bağıntılarıyla
kesilir; kalınlık kâğıdın kalınlığıdır (ADR 0312); ekranın 3 pikselden sık deseni düz tona çevirmesi yalnız ekranındır, kâğıt
çizgi çizgi yazar (ADR 0332 §3); bir partinin en çok 200 000 parçası (`MAX_HATCH_PIECES`) aşılırsa parti resim olarak kalır ve
not düşülür. Degrade artık bu nedenle resim olmaz.

### 2. DXF'ten ilişkili tarama (`CAD-HATCH-13`)

Okuyucu her yolun bayrağını (92), kaynak nesnelerinin tutamaçlarını (97, 330'lar) ve tohum noktalarını (98) tutar. Model
uzayındaki, kaynak nesnesi olan ve tek bir tarama olarak alınan ilişkili tarama bütün dosya okunduktan sonra bağlanır
(`Emitter::tie_hatches`, delikler birleştirildikten sonra; birleştirme tutamaçları ve bağları yeni yerlerine taşır):

- dış yol (92'nin 1. biti, yoksa ilk yol) tek bir kapalı nesneyse (alan, daire, kapalı çoklu çizgi, tam elips, kapalı NURBS)
  o `outer`'dır; değilse yolun nesneleri yüzün çizgileridir (`bounds`, ADR 0328 §3);
- öbür yolların nesneleri adalardır;
- tohum dosyanınkidir, bölgede değilse dış halkanın ortası, o da değilse tarama ilişkisiz alınır ve söylenir;
- bir kaynağı alınmayan (atlanan ya da bloğun içindeki) tarama ilişkisiz alınır ve söylenir; bloktaki ve birden çok bölgeye
  bölünen ilişkili tarama da.

Okuyucu nesnelerin kalıcı kimliklerini bilmez: bağ, nesnelerin içe aktarmadaki sırasını taşıyan yer tutucu kimliklerle yazılır
(`kentos_contracts::import_placeholder`: “KDXF”, sıfırlar, sıra; `import_placeholder_index`). İçe aktarmanın uygulaması
(`import_apply::tie_placeholders`) nesneleri ekledikten sonra aynı adımda yer tutucuları eklenen nesnelerin kimlikleriyle
değiştirir; belge taramayı o adımda kuralıyla yeniden keser (ADR 0186 §6). Eklenmeyen (süzülen katmandaki) kapalı nesnesi ya da
yüzün nesnelerinden biri eklenmeyen tarama ilişkisiz kalır; eklenmeyen ada listeden düşer. Büyük içe aktarma (kare kare) da
gruplu adımın sonunda aynı yoldan bağlar.

Bu iş belgenin tarama izleyicisinde eski bir hatayı da ortaya çıkardı: izleyici nesneyi çekirdeğe sözleşmenin JSON'uyla
geçiriyordu ve JSON yolu çoklu çizgiyi açık okur (ADR 0258'in “tipli ev sahiplerinin” kuralı); kapalı çoklu çizginin içindeki
ilişkili tarama ilk düzenlemede bağını yitiriyordu. İzleyici artık çoklu çizginin kapalılığını sözleşmeden alır
(`hatch_ties::core`; test `a_hatch_follows_a_closed_polyline`).

### 3. DXF'e ilişkili tarama

Dışa aktarmanın girdisi nesnelerin kalıcı kimliklerini taşır (`DxfWriteInput.uids`: nesnenin girdideki kimliğinden kalıcı
kimliğine; boşsa hiçbir tarama ilişkili yazılmaz, eski çağıranlar değişmez). Yazıcı nesneleri yazmadan önce ilişkili taramaları
ve kaynaklarını bulur ve tutamaçlarını ayırır (`writer::hatch_ties`), böylece taramadan önce yazılan nesne de onu anar:

- kapalı nesnesinin ya da yüzün bütün nesnelerinin de yazıldığı tarama `71 1` yazılır; dış yolun `97 n` ve 330'ları kapalı
  nesne ya da yüzün nesneleri, her deliğin kutusu yaylarıyla bir adanın kutusuyla (boyutunun %2'si içinde) eşleşirse o adadır;
  eşleşmeyen delik (taramanın boş bıraktığı yazınınki gibi) kaynaksızdır;
- kaynak nesne taramanın tutamacını `{ACAD_REACTORS` içinde anar;
- tohum `98 1` ile yazılır;
- kaynaklar ortak başlıkla kendi nesnesi olarak yazılanlardır (alan, çoklu çizgi, çizgi, daire, yay, elips, NURBS); çok parçalı
  alanın parçası değildir. Biri yazılmazsa (seçilmeyen katmanda) tarama ilişkisiz gider.

### 4. Bağımsız dosyalarla gidiş-dönüş

`scripts/fixtures/dxf_hatch_cases.py` KentOS kodu olmadan DXF başvurusundan (AutoCAD 2007, AC1021) beş girdi yazar
(`fixtures/formats/v1/dxf-hatch/in/`): kapalı çoklu çizginin içinde dairesi ada olan ilişkili ANSI31 (ölçek 0,5, tohumlu);
dört çizginin kapattığı ilişkili dolu tarama; kenar yolları (çizgi, yay, eliptik yay, eğri); üç degrade (LINEAR, CYLINDER,
INVSPHERICAL; 421 gerçek renkleri); kesikli ve noktalı iki aileli özel desen ve kullanıcı tanımlı çift tarama. Başsız sunucu
her birini `file.import.dxf` ile yeni yerel çizime alır, ilişkililerin yer tutucusuz bağlandığını denetler ve
`file.export.dxf` ile yazar (`dxf_hatches_go_round`; `KENTOS_WRITE_DXF=1` çıktıları `out/`'a yazar). Betik `--check` ile iki
dosyayı kendisi okur ve tarama tarama karşılaştırır: yollar aynı sınırı çizer (her yolun yay, elips ve eğrileri betikte
hesaplanır; iki yönde 5·10⁻⁵ m), ilişkili tarama ilişkili kalır ve kaynakları girdidekilerle aynı geometrideki nesnelerdir, her
biri taramayı reaktörlerinde anar, tohum bölgededir; desen aileleri aynı çizgilerdir (açı 180° kipinde, aralık, çizgilerin
aralık kipinde yeri, kesikler); degrade adını, açısını ve iki rengini korur.

## Sonuçlar

- Degradeli tarama PDF'te vektördür; bant sayısı sabittir (64), kalitesi ekranın sürekli rengine bant yarısı kadar yaklaşır.
  Saydam degradenin bantları kenarsızdır: çok büyük yakınlıkta bantlar arasında ince bir çizgi görülebilir.
- AutoCAD'in ilişkili taramaları KentOS'ta sınırını izler, KentOS'unkiler AutoCAD'de izler (AutoCAD'le canlı açılış sınanmadı;
  dosyalar başvuruyla ve bağımsız okuyucuyla sınandı). Taramanın boş bıraktığı yazılar (`cutouts`) DXF'e ilişki olarak gitmez,
  delik olarak gider.
- Kimlikler girdide olduğundan DXF'in baytları aynı çizimde aynıdır (yeniden kayıtta aynı tutamaçlar); kimliği olmayan
  çağıranın çıktısı değişmedi (`dxf_write` örnekleri aynı).

## Doğrulama

```bash
python3 scripts/fixtures/dxf_hatch_cases.py --check
cargo test -p kentos-headless --test headless dxf_hatches_go_round   # KENTOS_WRITE_DXF=1 çıktıları yeniden yazar
cargo test -p kentos-formats --test all an_associative_hatch; cargo test -p kentos-formats --test dxf_write
cargo test -p kentos-domain --test all hatch; cargo test -p kentos-plot
KENTOS_PLOT_OUT=$PWD/.run/shots cargo test -p kentos-plot --test all a_gradient_and_a_dense_hatch   # tarama-pdf.pdf
python3 scripts/fixtures/dxf_write_reference.py --check
```
