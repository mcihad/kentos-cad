# ADR 8153: Subasman kotu — PAİY 4/1-yyy

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8098](8098-plan-building-height-and-storeys.md) §6 ("Tahvil (9/1-c): … subasman +1,20").
- **Bağlam:** ADR 8098 (bina yüksekliği ve kat adedi; bina yüksekliği binanın kot aldığı noktadan), 8072 (bina
  programının binası ve kendi ±0,00'ı, `ProgramBuilding.zero`), 8083 ve 8085 (kat yükseklikleri).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-yyy (RG 3.7.2017/30113), 9/1-c.
- **Sözleşme:** yeni komut yok. `kentos.auditFacts` 1'in katlarına `zero` (katın binasının ±0,00'ı).
  `tr.kentos.mimari-mevzuat` 0.41.0 (`subasman`). **`.kcad`:** değişmedi. **Web:** dokunulmadı; TODOS.md §0.2'de.

## §1. Sorun

Zemin katın taban kotu hiç okunmuyordu: zemin katı +1,50'ye yükseltilmiş ya da ±0,00'ın altına inmiş bir bina
denetimden geçiyordu. 9/1-c'nin bina yüksekliği ile kat adedi arasındaki tahvili de bu kota (en çok +1,20) dayanır.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-yyy (1.7.2026'ya kadar işlenmiş birleştirilmiş metin; değişiklik işareti yok) | "Subasman kotu (Zemin kat taban kotu): Binaların zemin kat taban döşemesi üst kotunu (İmar planlarında aksine bir hüküm bulunmaması halinde, ±0.00 kotunun altına düşemez ve +1.20 kotunun üzerine çıkamaz.)" | Zemin katın kotu ±0,00 ile +1,20 arasında (K395) |
| PAİY 9/1-c | Kat adetleri binanın kot aldığı noktaya göre; tahvilde kot alınan noktaya en fazla 1,2 m eklenerek belirlenen subasman kotu | Binanın ±0,00'ı kot aldığı noktadır (K395) |
| ADR 8072 | Bina programının binası kendi ±0,00'ını söyleyebilir (bir blok öbüründen yüksek ya da alçak girilir) | Katın binasının sıfırından ölçülür (K396) |

## §3. Karar

- **K395 — Kural (`subasman`):** her binanın zemin katı (adında Zemin geçen katı, yoksa asma kat olmayan katlarından
  kotu binanın ±0,00'ına en yakını) ±0,00'ın altına düşemez, +1,20'nin üstüne çıkamaz; dışındaki kaldı
  (`paiy_plinth_level`, katın mahalleriyle). İmar planının aksine hükmü KentOS'ta okunmaz: bulgunun onarımı onu uzman
  onayına bırakır.
- **K396 — Binanın sıfırı:** olguların katları binasının ±0,00'ını (`zero`, bina programından) taşır; zemin katın kotu
  katın kotu eksi o. Binası olmayan ya da sıfırını söylemeyen binanın katları projenin ±0,00'ından.

## §4. Uygulama

- `crates/shared/extensions/src/audit_facts/building.rs` (`StoreyHolds.zero`), `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/subasman.py` (yeni), `mimari.py`, `denetim.py`
  (`subasman_denetimi`), `manifest.json`, `data/surumler.json` (4/1-yyy'nin 1.10.2017 hâli).

## §5. Sınamalar

- **Kural** (`test_subasman.py`; 4/1-yyy'nin metninden elle): 8 bağımsız durum: zemin kat ±0,00 ve +1,20 geçer; +1,50
  ve −0,30 kalır; adsız katlarda ±0,00'a en yakın kat (+0,45) geçer; asma kat zemin sayılmaz (+1,60'taki kat kalır);
  iki binanın +1,40'taki zemini kalır; kendi sıfırı +2,00 olan binanın +3,00'teki zemini (+1,00) geçer.
- **Masaüstü:** kural seti satırına "PAİY 4/1-yyy (1.10.2017) geçti" (`regulation_dates_tests`: Zemin kat ±0,00'da);
  bina programı sahneleri (kendi sıfırlı iki blok), çıkma, emsal, bodrum listesi, taş duvar, kesme taş, kullanım ve ortak
  süzgeçlerle 178 masaüstü sınaması yeşil; olguların altı, SDK şeması ve iş çalıştırıcının ulusal kural sınamaları yeşil.
- **Resim:** yok (arayüz değişmedi; Denetim'in yeni bulgusu öbür kural bulgularıyla aynı yerde).

## §6. Kalanlar

- İmar planının subasman hükmü (plan notunun konusu olarak).
- Öbür kurallar (`kat_yuksekligi.ustteki`) katların kotunu bugün projenin ±0,00'ından okur; binanın kendi sıfırıyla
  okunması — yapıldı: ADR [8154](8154-rules-by-building-zero.md).
- Tahvil (9/1-c): planın yalnız birini verdiği durumda bina yüksekliği ile kat adedinin birbirine çevrilmesi — yapıldı:
  ADR [8157](8157-height-storeys-conversion-9-1-c.md).
