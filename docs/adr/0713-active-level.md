# ADR 0713: Etkin kat

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-UX-04'ün ilk dilimi.
- **Bağlam:** ADR 0639 (duvar dönüşümünde açık kat), ADR 0641 (mahal sınırında açık kat), ADR 0643 (kolon düşümü), ADR 0703
  (mimari denetim), ADR 0704 (merdivenin kat yüksekliği), ADR 0707 (aks ağı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Projede kat tanımlandığında (Proje ayarları › Mimari ya da Mimari şablonu, ADR 0659) çizim araçları yeni nesneye kat
vermiyordu: Duvar, Dikdörtgen duvar, Ölçülü bölme, Kolon, Merdiven, Rampa ve Aks ağı kat seçeneği taşımıyordu, mahal araçları
her oturumda “Belirtilmedi” ile başlıyordu. Bir dairenin baştan sona çizildiği GUI yürüyüşünde (ADR 0344'ün 27 Ekim eki)
Denetim her duvar ve mahal için “Duvarın katı yok; projede 1 kat var.” diyordu ve bunu düzeltmenin tek yolu her nesnenin
Öznitelikler'inde Kat'ı tek tek seçmekti.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Archicad 29 — Home Story](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-4.htm): planda
  yeni elemanın ana katı varsayılan olarak geçerli kattır; araç ayarlarından başka kat seçilebilir, eleman o kata konur.
- [Archicad 29 — About Stories](https://help.graphisoft.com/AC/29/INT/_AC29_Help/050_ViewsVB/050_ViewsVB-5.htm): proje katlarla
  düzenlenir, plan bir katın görünüşüdür.

KentOS kararı: Archicad'in “geçerli kat” kavramı **Etkin kat** adıyla, etkin katman gibi: yeni nesnelerin nereye gittiğini
söyleyen oturum durumu. Kat görünümü (yalnız etkin katı gösterme, öbürlerini soluk gösterme) bu dilimde yoktur.

## Karar

### 1. Durum

- Etkin kat masaüstü oturumunundur, çizime yazılmaz. Çizim açılınca zemin kat (kotu 0 olan kat, yoksa listenin ilki) etkin
  kattır; kullanıcı başka kat ya da **Katsız** seçebilir. Seçilen kat projeden silinirse yine zemin kata dönülür. Katı
  olmayan projede etkin kat yoktur.
- Araçların bağlamı etkin katı taşır (`Context::level`); araçlar onu komutlarının girdisine açıkça yazar (`arch.level`,
  CMD-07). Komutlar ve Python/MCP değişmez: kat her zamanki gibi girdidedir.

### 2. Araçlar

- **Yeni nesneler etkin kata:** Duvar, Dikdörtgen duvar, Ölçülü bölme, Kolon (aks kesişimleri dahil), Merdiven, Döner
  merdiven, Rampa ve Aks ağı (aksları ve kolonları) etkin katla yazar. Çizgiden duvar'ın Kat ayarı etkin katı izler.
- **Duvar arayan araçlar:** Mahal, Toplu mahal bul, Mahal ayırıcı ve İç ölçü zinciri'nin Kat ayarı, çizimde o katta duvar
  varsa etkin katı izler; yoksa eskisi gibi bütün katlar (katlardan önce çizilmiş duvarların katı yoktur: eski bir çizimde
  mahal bulma bozulmaz).
- Bir araçta kat elle seçilince o araç oturumu boyunca seçilen kat kalır; izlenen kat ayarda “Zemin kat (etkin kat)” diye
  yazılır.

### 3. Masaüstü

- Durum çubuğunda **Kat** hücresi (yalnız projede kat varken): “Kat: Zemin kat”; menüsü katlar, Katsız ve Katları
  düzenle…. Hücre yer kalmayınca yalnız ikonuyla kalır (sözleri mod hücresinden sonra gider).
- Yeni nesne çizen araçların ayarlarında (Öznitelikler › Etkin aracın ayarları) ilk satır **Kat**: aynı menü.
- Seçim söylenir: “Etkin kat: 1. kat. Yeni duvar, kolon, merdiven ve rampalar bu kata çizilir.”
- Yeni ikon `archLevel` (kesitte iki kat ve geçerli katı gösteren ok).

## Sınırlar

- Etkin kat dosyaya yazılmaz; çizim her açılışta zemin katta başlar.
- Kat görünümü yoktur: bütün katlar birlikte çizilir; katları planda ayırmak katmanlarla ya da yan yana çizimle yapılır.
- Açıklık katını duvarından, etiket kaynağının katını kendi kuralından alır (ADR 0663); onlara etkin kat yazılmaz.
- Katsız çizilmiş nesnelere toplu kat atama (Denetim'in düzeltmesi) bu dilimde yoktur.

## Doğrulama

- `cargo test -p kentos-interaction --test all active_level`: duvar, kolon ve merdiven etkin kata; etkin kat yokken katsız;
  mahal aracı etkin katı yalnız o katta duvar varken izler, elle seçilen kat kalır. Bütün etkileşim takımı (760) geçti.
- `cargo test -p kentos-desktop --bin kentos-cad active_level`: çizim zemin katta açılır (kotu 0, ilki değil), menü etkin katı
  değiştirir, Katsız katsız yazar, silinen kat zemine döner. Masaüstü takımı (1343) ve izler geçti.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=kat-arac,kat-menu,kat-duvar,kat-duvar-oznitelik,daire-mahal-onizleme,daire-denetim
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`: aracın Kat satırı ve menüsü,
  1. katta çizilen duvarın Öznitelikler'i, Toplu mahal bul'un “Zemin kat (etkin kat)”ı, daire yürüyüşünde Denetim'in
  “Bulgu yok”u (önce dört kat bulgusu).
