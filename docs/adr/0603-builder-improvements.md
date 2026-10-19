# ADR 0603: İfade oluşturucu — yorumlar, Biçimle, Son kullanılanlar, örnekler

- **Durum:** kabul edildi (2026-10-19). ADR 0600'ün İfade oluşturucu dilimi; sahibin sözü: “expression editör … yeterince iyi değil”.
- **Bağlam belgesi:** ADR 0100 §5 (İfade oluşturucu), 0601 (iç içe ifadeler), 0600.
- **Web:** ertelendi (ADR 0228); çekirdek ortaktır.

## Bağlam

Uzun bir koşul tek satırda yazılıyordu ve öyle kalıyordu: açıklama yazılamıyor, satırlara bölünemiyordu. Yardımdaki örnekler
yalnız okunuyordu. Her açılışta işlevler kategorilerin içinde yeniden aranıyordu. `dizi_her`'in gövdesindeki `@öğe` düzenleyicide
“değişken yok” diye işaretleniyordu, oysa derleme onu biliyordu.

## Karar

1. **Yorumlar** dilin parçasıdır:
   - `// …` satır sonuna kadar, `/* … */` aralık;
   - ikisi de geçerli bir ifadede başka anlama gelemez (değer `/` ya da `*` ile başlamaz);
   - kapanmayan `/*` derleme hatasıdır;
   - metnin içinde yorum yoktur (`'a//b'`).
   - QGIS'in `--`'i alınmadı: dilde `--1` çift eksidir ve dondurulmuş durumlarda öyle geçer.
   - Düzenleyici yorumu soluk renkle gösterir (`Class::Comment`).
2. **Biçimle** (sekmelerin yanında, `editor::format`): bir satıra (60 harf) sığan kısım satırında kalır. Sığmayanda:
   - `durum`'un her kolu ayrı satırdadır;
   - uzun çağrının her bağımsız değişkeni ayrı satırdadır;
   - `ve` / `veya` zincirinin her öğesi ayrı satırdadır.
   - Yazılan, akışın ağacıyla aynı ifadeyi okur.
   - Yorumlu ifade biçimlenmez (ağaçta yorumun yeri yoktur), nedeni söylenir.
3. **Son kullanılanlar:** Tamam'la kapanan ifadenin işlevleri ve alanları ağacın başında, en yenisi önde, en çok sekiz tanesi
   (program açık kaldıkça).
4. **Örnekler tıklanır:** yardımdaki örneğe basmak onu imlecin yerine yazar (Akış'ta düğüm olarak koyar).
5. **İç içe ifadenin adları uyarı değildir:** gövdede `@öğe`, `@sayaç` (ve `@element`, `@counter`) ya da `değişkenle`'nin adı
   bilinmeyen `@` değeri sayılmaz (`complete::bound_names`); gövdenin dışında sayılır.

## Doğrulama

- Çekirdek:
  - `tests/qgis.rs`'in `comments_are_between_tokens`'ı (yorumlar, `--`, metin, kapanmayan yorum, renk sınıfı);
  - `editor::format`'ın testleri (satırlar, genişlik, aynı ağaç, yorumda ret);
  - `a_bound_name_is_not_warned_of`.
- Masaüstü:
  - `the_builder_remembers_formats_and_takes_examples` (Tamam'ın hatırladıkları, örneğin yazılması, Biçimle'nin aynı ağacı);
  - `expression::tests::screens`'e `bicimli` ve `son` durumları, iki boy, iki tema (`.run/shots/ifade-olusturucu-*`).
