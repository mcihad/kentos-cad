# ADR 0749: Formların pencereleri her boyda ve yazı boyunda sığar

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin isteği:
  > form tasarımlarında tasarım yaparken ciddi manada sağa sola taşmalar var, üstteki özellikle sekmelerde;
  > formları açılınca pencere daracık kalıyor
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Sahibin görünüşü: 14 piksellik yazı, Plus Jakarta Sans, açık tema. Bu görünüşle ve 1100 × 650, 1440 × 900, 1920 × 1080
pencerelerde formların pencereleri resimlendi (`form_fit_screens`). Dört kusur bulundu:

- **Sekmeler taşıyordu.** Tasarımcının sağ bölmesi 340 piksel sabitti; yazı boyuyla büyümüyordu. “Özellikler |
  Önizleme | Değişiklikler | İş akışı” 14 piksellik yazıda bölmeye sığmadı: “İş akışı” iki satıra kırıldı ve bölmenin
  dışına taştı. KentOS UI'ın parçalı seçimi (`Segmented`) parçaları yazılarından dar yapmaz. Sığmayınca satır
  kutusundan taşıyordu.
- **Bölmeler sekmeyle kayıyordu.** Önizleme, Değişiklikler ve İş akışı sekmelerinde sağ bölme 400, Özellikler'de 340
  pikseldi. Sekmeye basınca orta bölme ve araç çubuğu 60 piksel yana kayıyordu.
- **Araç çubuğu kesiliyordu.** 1100 piksellik pencerede orta bölmenin araç çubuğu sığmadı; Geri al ve Yinele görünmedi.
- **Düğmeler pencerenin dışında kalıyordu.** Gövdenin yüksekliği sabitti: tasarımcıda 390, Formla düzenle'de 460, Alt
  türler'de 300 ve 280, Değer listeleri'nde 360, Değer bileşimleri'nde 320 piksel (12 piksellik yazıya göre, yazıyla
  büyür). 650 piksellik pencerede 14 piksellik yazıyla kutu pencereden uzun oldu. Kaydet ve Vazgeç pencerenin altında
  kaldı.

Başvurulan yollar (30 Ekim 2026'da okundu):

- Microsoft'un WinUI komut çubuğu: pencere daralınca komutlar sığmayanlardan başlayarak taşma menüsüne geçer. Sıkışık
  görünüşte komutların yalnız ikonları görünür, adları ipucundadır.
  <https://learn.microsoft.com/en-us/windows/apps/design/controls/command-bar>
- Microsoft'un Pivot'u: başlıklar sığarsa durur, sığmazsa kaydırılır; başlık satırı kutunun dışına taşmaz.
  <https://learn.microsoft.com/en-us/windows/apps/design/controls/pivot>
- Qt'nin `QSplitter`'ı (QGIS'in form tasarımcısındaki bölmeler): her bölme en az ve en çok boyu içinde kalır; kutu
  büyüyünce ya da küçülünce fark bölmelere paylarına göre dağıtılır.
  <https://doc.qt.io/qt-6/qsplitter.html>
- Kısalan yazının tamamı ipucundadır (ADR 0748'in Microsoft kaynakları).

## Karar

### 1. Parçalı seçim yerinden taşmaz

KentOS UI'ın `Segmented`'i, satırın yeri bilinince yerleşir. Kendi genişliğinde (`Shrink`) duran seçim de yeri bilir:
KentOS UI'ın `Responsive`'ine en çok genişlik olarak verilir.

- **Sığarsa:** eskisi gibidir. Parçalar yazıları kadar olur ya da yeri eşit paylaşır (`fill_widths`).
- **Sığmazsa, parçaların hepsinin ikonu varsa:** yalnız ikonlar görünür, adlar ipucundadır (komut çubuğunun sıkışık
  görünüşü).
- **Yine sığmazsa ya da ikon yoksa yazılar kısalır** (`shrunk_widths`):
  - dar parçalar kendi genişliğini tutar, geniş olanlar kalanı eşit paylaşır;
  - seçili parça, diğerlerine en az bir harf ve “…” kalıyorsa yazısının tamamını tutar;
  - kısalan yazı “…” ile biter (`Elided`) ve tamamı ipucundadır, ipucu yalnız kısalınca açılır (ADR 0748'in
    `Tip::when_cut`'ı).
- **`segmented::natural_width`:** yazıların kısalmadan sığması için gereken genişliği söyler (yuva ve aralıklar dahil).
  Seçimi taşıyan bölme bunu en az genişlik olarak alır.

### 2. Üç bölmeli düzen ve geniş-ya-da-dar

KentOS UI'a iki bileşen eklendi:

- **`Panes`:** solda liste, ortada çalışma alanı, sağda denetçi.
  - Yan bölmeler kutunun genişliğinden paylarını alır (`.left(pay, en az, en çok)`, `.right(…)`). Sınırlar 12
    piksellik yazıya göre verilir ve yazıyla büyür.
  - Ölçülen bir şeyin sığması gereken genişlik (`.least(Side, piksel)`) en azı geçer.
  - Orta bölmeye en azı (`.middle`) kalmazsa yan bölmeler kendi en azlarına dek geri verir, önce sağdaki.
  - Genişlik içerikten değil yerden bulunur. Denetçinin sekmesi değişince bölmeler yerinden oynamaz.
- **`fallback(geniş, dar)`:** geniş düzen yerine sığarsa o gösterilir, sığmazsa dar olan. İkisi de ağaçta durur, durumları
  kaybolmaz.

### 3. Form tasarımcısı

- **Bölmeler:** `Panes` ile yerleşir.
  - **Sol bölme:** kutunun %20'si, 200–260 piksel; en az, “Alan | İlişki | Liste | Bölüm”ün genişliği.
  - **Sağ bölme:** %36'sı, 330–480 piksel; en az, “Özellikler | Önizleme | Değişiklikler | İş akışı”nın genişliği.
  - **Orta bölme:** en az 300 piksel.
  - Sağ bölmenin genişliği dört sekmede aynıdır.
- **Araç çubuğu:** Sekme, Grup ve Not adlarıyla sığmazsa ikonlarıyla gösterilir, adları ipucundadır (`fallback`). Geri
  al ve Yinele, seçili öğenin araçlarının satırının sonuna taşındı: ilk satır yalnız ekleme araçlarıdır.
- **Yükseklik:** boyu verilmemiş pencerede bölmeler en çok 390 piksel yüksektir. Alçak pencerede kısalır (Dialog'un
  esnek parçası, `Fit`). Uyarılar üstte, Ad, Kapsam ve düğmeler altta, hep görünür. Boyu verilmiş pencerede bölmeler
  kalan yeri alır (ADR 0746).

### 4. Diğer form pencereleri

Gövde, düğmelerin üstünde esnek parçadır. Boyu verilmemiş pencerede en çok eski yüksekliği kadar olur, alçak pencerede
kısalır. Uzun içerik kendi içinde kayar, çubuğu içeriğin yanında durur:

- **Formla düzenle:** başlık satırı ve sekmeler üstte; sayfa en çok 460 piksel; altta sorunlar ile Vazgeç ve Kaydet.
- **Alt türler:** en çok 380 piksel; tür listesi ve kurallar tablosu kalan yeri doldurur.
- **Değer listeleri:** en çok 520 piksel; seçili liste kayar.
- **Değer bileşimleri:** en çok 400 piksel; seçili grup kayar.

## Doğrulama

- `cargo test -p kentos-ui --features snapshot --lib -- segmented panes fallback`:
  - parçaların paylaşımı ve seçili parçanın tamamı;
  - 220 piksellik bölmede dört sekme bölmenin dışına çıkmaz; ikonlu seçim yazı göstermez;
  - kısalan sekmenin üstünde 500 ms durunca ipucunda tamamı görünür;
  - bölme genişliklerinin kuralları;
  - geniş-ya-da-dar'ın seçimi.
- KentOS UI'ın bütün testleri (303) geçer; `clippy -D warnings`.
- `cargo test -p kentos-desktop --bin kentos-cad form_fit_screens -- --ignored --test-threads=1` (iki resim testi aynı
  anda çizerse süreç çöker; depodaki diğer resim testleri gibi tek tek): sahibin görünüşüyle (14 piksel,
  Jakarta, açık tema) üç pencere boyunda resimler, `.run/shots/form-sigar-*`:
  - tasarımcının dört sekmesi ve ilişki paleti;
  - tasarımcı tam ekran;
  - Formla düzenle, İlişkiler, Kayıtları denetle, Koşullu biçim, Alt türler, Değer bileşimleri, Kayıt kuralları ve
    Değer listeleri.
- `every_forms_window_keeps_its_buttons_in_a_low_window` (elle): formların 92 sahnesinin her biri 1100 × 650'de ve 14
  piksellik yazıyla kurulur. Vazgeç, Kaydet, Kapat, Tamam ve Uygula'nın hepsi pencerenin içindedir. Önceden Alt türler,
  Değer listeleri, Değer bileşimleri, tasarımcı ve Formla düzenle'de düğmeler pencerenin altında kalıyordu.
- Masaüstünün bütün testleri.

## Kalan

- Formların dışındaki pencerelerin sabit iç yükseklikleri: 92 sahnenin taraması diğer pencerelere de yayılabilir.
- Bölmelerin arasından fareyle çekilerek boyutlandırılması (Qt'nin `QSplitter`'ı gibi) ve boylarının saklanması.
