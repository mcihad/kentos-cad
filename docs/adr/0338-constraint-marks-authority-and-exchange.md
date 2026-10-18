# ADR 0338: Kısıt işaretleri, Parametreler penceresi, otorite ve DXF sınırı

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §13 `CAD-PARAM-11`, `14`, `15` (sahibin sözü: “cad param
  ile devam et ve bitene kadar durma”; beş grubun beşincisi ve sonuncusu). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0334 (model ve çözücü), 0335 (kısıt komutları ve aracı), 0336 (parametreler), 0337 (düzenlemelerde
  kısıtlar), 0249 ve 0320 (ilişkili ölçü ve bağ vurgusu), 0300 (ilişkili dizi), 0186 ve 0328 (ilişkili tarama), 0175 (bağlı yazı).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0334–0337 kısıtları çözdü, komutlarla ekletti, parametrelerle sürdürdü ve her düzenlemede korudu. Kullanıcı ise çizimde hangi
kısıtın nerede durduğunu göremiyor, parametreleri yalnız komutla değiştirebiliyor ve çelişkide hangi kısıtın sorun olduğunu
çizimde bulamıyordu. AutoCAD bunu kısıt çubuklarıyla (her parçanın yanında kısıtın simgesi; üzerine gelince bağlı parçalar
vurgulanır) ve Parametre yöneticisiyle çözer.

İki sınır da açıktı. Birincisi: kısıt sistemi bir nesneye, kendi geometrisini başka bir ilişkiden alan nesneyi (ilişkili tarama,
bağlı yazı, ilişkili ölçü, ilişkili dizinin öğesi) de bağlayabiliyordu. O nesneyi iki otorite taşır; düzenlemede hangisinin
kazanacağı belirsizdi. Mimari nesneler gelince (`CAD-ARCH-13`: duvara gömülü kapı) aynı soru yeniden doğar. İkincisi: DXF
alışverişinde kısıtların ne olduğu söylenmiyordu. AutoCAD'in kısıtları okunmuyor, KentOS'unkiler yazılmıyordu; kullanıcı bunu
sessizce yaşıyordu.

## Karar

### 1. Kısıt işaretleri ve bağlı geometri vurgusu (`CAD-PARAM-11`)

- Çekirdek `kentos_interaction::constraint_marks`: `glyphs(doc)` her kısıtın andığı her parça için bir işaret verir. İşaretin yeri
  kenarın ortası, nokta ya da çemberin tepesidir. Aynı yerdeki iki parça (Çakışık'ın iki ucu) tek işaret alır. Ölçü kısıtının adı ve
  değeri (`genislik = 10`) ilk işaretinde yazılır. Çizimin tutmadığı kısıt (`unmet`) ve yalnız ölçen başvuru ayrıca işaretlenir.
  `bound(doc, i)` kısıtın andığı parçaları, ilişkili ölçünün bağ vurgusuyla aynı biçimde (`Bond`) verir. En çok 2000 kısıt
  işaretlenir; daha büyük küme Parametreler'de listelenir.
- Masaüstü (`constraint_view.rs`): işaretler çizimin işaret tuvalinde kutucuklar olarak çizilir. Her kutucukta türün ikonu
  (`catalog/v1/additions.json`'un kısıt ikonları), varsa ad ve değer yazılır. Aynı yerdeki işaretler yan yana dizilir. Tutmayan
  kısıt tehlike rengindedir, başvuru soluktur. İmleç bir işaretin üstündeyken kısıtın bütün parçaları vurgu renginde çizilir ve
  adı (“Paralel”, “· başvuru”, “· tutmuyor”) kutucuğun üstünde görünür.
- İşaretler çizim her değiştiğinde bir kez hesaplanır, karelerde hesaplanmaz. Önbellek belgenin revizyonu ve kuşağıyla
  anahtarlanır; kısıtsız çizimde hiçbir şey okunmaz.
- **Kısıtları göster** (`constraint.show`, oturumun ayarı) işaretleri gizler ve yeniden gösterir. CAD'de Değiştir ›
  Kısıtlar'dadır, CBS'de Düzenle › Kısıtlar'da.

### 2. Parametreler penceresi ve çelişkinin çözüm akışı (`CAD-PARAM-11`)

- **Parametreler** (`constraint.parameters`; `parameters_window.rs`) penceresinde üç bölüm vardır:
  - **Parametre tablosu:** ad, ifade, hesaplanan değer ve türü, Uygula ve Sil. Yalnız ölçen parametrenin ifadesi düzenlenmez
    (“yalnız ölçer”).
  - **Yeni kullanıcı parametresi:** ad ve ifade.
  - **Kısıt listesi:** sıra, tür, parametre, durum (tutuyor, tutmuyor, çelişiyor) ve Sürücü yap / Başvuru yap ile Kaldır.
- Pencere yazdığını komutlarla yazar. Parametreler `cad.parameters.edit` ile, kısıtların kipi ve kaldırılması
  `cad.constraints.edit` ile yazılır. Penceredeki her yazma tek geri alma adımıdır ve komut geçmişinde komutun adıyla görünür.
  Pencerenin ayrı bir yazma yolu yoktur.
- **Çelişki akışı:** çözülemeyen değerde komut reddeder ve hiçbir şey yazılmaz (ADR 0334). Pencere çözücünün iletisini gösterir.
  Reddin andığı kısıtlar listede “çelişiyor” olarak işaretlenir ve satırları vurgulanır. Kullanıcı değeri geri alır, kısıtı başvuru
  yapar ya da kaldırır. Aynı reddi Python ve MCP de aynı kodla ve aynı kısıt kimlikleriyle alır.
- Pencerenin iki listesi pencerenin yüksekliğini paylaşır; 1100 × 650'de de Kapat ekranda kalır.

### 3. Otorite: kısıt başka ilişkinin taşıdığı nesneye konmaz (`CAD-PARAM-14`)

- Bir nesnenin geometrisinin tek otoritesi olur. Geometrisini başka bir ilişkiden alan nesneye kısıt konmaz;
  `cad.constraints.edit` onu `constraint_governed` koduyla, nedeniyle ve nesnenin kimliğiyle reddeder. Kural `Document::governed`'dadır:
  - ilişkili tarama: sınırını izler, kısıt sınıra konur;
  - bağlı yazı: nesnesini izler, kısıt nesneye konur;
  - ilişkili ölçü: bağlı olduğu noktaları izler; ölçü yerine ölçü kısıtı kullanılır;
  - ilişkili dizinin öğesi: yerini dizi verir, Diziyi düzenle'yle değişir.
- Sıra tektir: önce kısıtlar çözülür (ADR 0337), sonra izleyenler kısıtlı nesneleri izler (tarama, yazı, ölçü, dizi). Böylece bir
  nesne iki sistemden taşınmaz.
- Mimari nesneler gelince (`CAD-ARCH-13`) aynı kural geçerli olur. Duvara gömülü kapının yerini duvar verir; kapı bu listeye
  eklenir ve kısıt duvara konur. Kural tek yerdedir, yeni ilişki onu bir satırla genişletir.

### 4. DXF'te kısıtların sınırı (`CAD-PARAM-15`)

- **Okuma:** DXF okuyucusu AutoCAD'in kısıt gruplarını (`ACDBASSOC2DCONSTRAINTGROUP`) okumaz; geometri çizildiği gibi, kısıtsız
  gelir. Dosyada kısıt grubu varsa raporda bir “Kısıt” notu sayısıyla söyler (`dxf::constraint_groups`). Gruplar baytlarda aranır:
  maliyeti dosyanın bir kez taranmasıdır.
- **Yazma:** kısıtlar ve parametreler DXF'e yazılmaz; geometri çözülmüş hâliyle gider. Kısıtlı çizimin dışa aktarma raporu bunu bir
  “Kısıt” satırıyla söyler (başsız `export`, masaüstü ve Python aynı rapordan). Rapor, kısıtların başka programda düzenlenebilir
  kısıt olarak gelmediğini de belirtir.
- KentOS'ta kısıtın bulunması, onun dışarıda düzenlenebilir kısıt olarak korunduğunu kanıtlamaz. Kısıtların tek taşıyıcısı `.kcad`
  (şema 73) ve veritabanı projesidir (ADR 0336). DWG zaten okunmuyor.

## Sonuçlar

- CAD-PARAM'ın on beş maddesi bitti (ADR 0334–0338). Planın kabulü sağlanıyor: dikdörtgenin genişliği değişince karşı kenarlar
  paralel, köşeler dik kalıyor. Çelişen değer her host'ta aynı kodu ve aynı kısıt kimliklerini veriyor ve hiçbir geometri
  yazılmıyor.
- Yeni ret kodu `constraint_governed` (`codes.rs`; `cad.constraints.edit`'in yeteneklerinde).
- Yeni arayüz komutları `constraint.parameters` (semantic, `cad.parameters.edit` ve `cad.constraints.edit` yoluyla) ve
  `constraint.show` (oturum); kayıtları `catalog/v1/additions.json` ve `operations.json`'dadır.
- Bilinen sınırlar:
  - İşaretler çizimin işaret tuvalindedir: paftaya ve PDF'e girmez.
  - Kutucuklar yalnız aynı yerde dizilir; yakın yerlerdeki işaretler üst üste binebilir.
  - Çelişki satırları yalnız reddin kimlik verdiği durumda işaretlenir. Çözücünün yakınsamadığı durumda kimlik yoktur; yalnız
    ileti görünür.

## Doğrulama

- **Ortak durumlar:** `python3 scripts/fixtures/constraint_command_cases.py --check`. `cad.constraints.edit`'in iki yeni durumu
  (ilişkili taramaya ve ilişkili ölçüye kısıt reddi) KentOS kodu olmadan denetlenir. Masaüstünün ve Python'un oynatıcıları geçer.
- **İşaretler:** `cargo test -p kentos-interaction --test all constraint_marks`. Kapsar: kısıtsız çizim, buluşma noktasının tek
  işareti, kenar ortaları, ölçünün adı ve değeri, bağlı parçalar.
- **DXF:** `cargo test -p kentos-formats --test all autocad_constraint_groups_are_said_not_read` ve
  `cargo test -p kentos-headless --test headless constraints_are_said_not_written_to_dxf`.
- **Resimler:** `KENTOS_SHOTS_ONLY=kisit-isaretler,kisit-isaret-vurgu,kisit-parametreler,kisit-parametre-celiski cargo test -p
  kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-kisit-isaret*`, `arac-kisit-parametre*`).
