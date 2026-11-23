# ADR 9165: Formun ilişkili listesi veri katmanının satırlarını kaynaktan okur

- **Durum:** kabul edildi (2026-11-23); adım VY-64. Plan: [plan/veri.md](../../plan/veri.md) VY-64.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 22. maddesinin
  kalanı (ADR 9135 §4): “M1 ve M2 formlarında ikişer bakım, M3'te sıfır bakım olmalı”. Veri katmanı üzerindeki
  formda ilişkili listenin satırları çizimdeki nesnelerden geliyordu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- İlişki üst katmanın bir alanını anahtar alabiliyor (ADR 9135, `parentKey`). Formun ilişkili öğesi kaydın kodunu
  (M1) tutan alt kayıtları listeliyor.
- Listenin satırları formun “dünyası”ndan gelir. Dünya çizimdeki nesnelerden kurulur (`Document::form_world`).
  Veri katmanının satırları kaynağındadır (ADR 9101); çizimde yalnız alınmış olanlar vardır. Bu yüzden M1'in formu
  bakımları göstermiyordu.
- **Araştırma.** Masaüstü CBS'lerde 1-N ilişkinin üst kaydının formu, alt katmanın o kayda bağlı nesnelerini bir
  tabloda gösterir. Tablo alt katmanın kendisini gösterir; düzenleme kipi de alt katmanınkidir. Kaynak:
  [QGIS “Connecting and Editing Data Across Layers”, Forms for 1-N relations](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/joins_relations.html).

## §2 Karar

1. **Ne sorulur** (`forms_source::source_asks`). Bir ilişkili öğe kaynağa yalnız şu durumda sorar:
   - öğenin katmanı bir veri katmanıdır;
   - ilişkisi kaydı bir üst anahtarla bağlar;
   - kaydın o alanda bir değeri vardır.

   Kalıcı kimlikle bağlanan öğe sormaz, çünkü kaynağın satırları çizimin kimliklerini tutmaz.
2. **Okuma** (`forms_source::read_related`).
   - Kaynaktan alanı koda eşit satırlar okunur, en çok 500; sayı alanında kod sayı olarak karşılaştırılır.
   - Okuma arayüzün iş parçacığında yapılmaz. Form açılınca istenir, yanıt gelince formun ekranı yeniden
     hazırlanır.
   - Önceki bir formun yanıtı bayattır.
3. **Dünyaya katma** (`forms_source::world_with`, `forms::prepared_with_rows`).
   - Satırlar çizimin nesnelerinin yanına, kaynak kimliğiyle (`kaynak:<anahtar>`) girer.
   - Çizime alınmış satır girmez: onun nesnesi düzenlendiği haliyle zaten dünyadadır. Bir satır iki kez sayılmaz.
   - Formun yenilenmesi (öbür nesneler değişince) satırları korur.
4. **Gösterim.**
   - Liste, tablo, kart ve grafik görünümleri kaynak satırlarının değerlerini gösterir; tablonun toplamları da onları
     sayar.
   - Kaynak satırı gösterilir, yazılmaz. Satırın araçları (aç, göster, bağı kes, sil) yerine soluk “kaynakta” yazar;
     listede satır “B01 · satır 1 (kaynakta)” der.
   - Okunamayan kaynağın nedeni öğenin altında söylenir; çizimin nesneleri yine gösterilir.

## §3 Kanıt

- **Masaüstü** (`gis/forms/source_rows_tests.rs`): değerlendirmenin GeoPackage'ı kullanılıyor; mahalleler salt
  okunur, bakım veri katmanı, ilişki koda göre.
  - M1'in formu kaynaktan iki bakım listeliyor, M2'ninki iki, M3'ünki sıfır. Okuma hatası yok.
  - B01 çizime alınıp bedeli 75 yapılınca M1'in listesinde iki bakım kalıyor: alınmış olan çizimin nesnesi olarak,
    öbürü kaynaktan; B01 iki kez sayılmıyor. Tablonun hücreleri alınmış olanda 75, kaynaktakinde 100.
- Uygulamanın kaynak kimliği sınaması, form sınamaları ve paylaşılan komut durumları geçiyor. Form, ilişki, veri, iz
  ve ortak masaüstü sınamaları geçiyor (380).
- **Resim:** `.run/gui-evidence/kullanim-9165/01-kaynaktan-iliskili-{light,dark}.png`. M1'in salt okunur formunda
  “Bakımlar 2”, tabloda B01 0 ve B02 100 “kaynakta”, Toplam 100.

## §4 Kalan

- Kaynak satırının kendi formunu ilişkili listeden açmak (ADR 9149'un satır hedefiyle).
- 500 satırdan çok bakımı olan kayıtta sayfalı okuma.
- Silme davranışının izole veride gerçek pencerede denenmesi (ADR 9135 §4).
- Web (TODOS.md §0.2).
