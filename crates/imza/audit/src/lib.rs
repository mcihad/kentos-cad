//! Denetim zinciri: hash ile bağlı olay kaydı (docs/adr/9002 §3).
//!
//! **Denetim kaydı sıradan bir log DEĞİLDİR**. Belediyede kayıt hukukî
//! belgedir: *"bu imzayı kim, ne zaman, hangi belgeye attı"* sorusunun cevabı
//! sonradan sessizce değiştirilebiliyorsa, cevabın hiçbir değeri yoktur.
//!
//! Olaylar bir **hash zinciriyle** bağlanır:
//!
//! ```text
//! event_hash = SHA-256( previous_event_hash ‖ kanonik olay verisi )
//! ```
//!
//! **Bu mekanizma elektronik imzanın YERİNE GEÇMEZ**: zinciri
//! üreten de aynı makinedir; bir saldırgan zincirin tamamını baştan yazabilir.
//! Sağladığı şey, **ORTADAN** bir olayın çıkarılmasının ya da değiştirilmesinin
//! tespit edilebilir olmasıdır.
//!
//! ## Kanoniklik neden kritik
//!
//! Zincirin işe yaraması için, aynı olayın **her zaman aynı baytları** üretmesi
//! gerekir. `serde_json`ın nesne alan sırası kaynak yapının alan sırasına bağlıdır;
//! bir gün alanların sırası değişirse eski kayıtların özeti tutmaz ve zincir sahte
//! bir alarm verir. Bu yüzden özet, JSON'dan değil **sabit sıralı, ayraçlı bir
//! metinden** hesaplanıyor (aşağıdaki `canonical`).

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// Zincirdeki tek olay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: String,

    /// Zincirin bir önceki halkası — ilk olayda `None`.
    pub previous_event_hash: Option<String>,

    pub transaction_id: String,
    pub batch_id: String,
    pub package_id: Option<String>,
    pub file_id: Option<String>,

    pub signature_method: String,
    pub client_type: String,

    pub event_type: String,
    /// ISO-8601.
    pub timestamp: String,

    /// Serbest ayrıntı.
    ///
    /// **Loglanmayacaklar buraya KONULMAZ**: PIN · özel anahtar · ham belge
    /// içeriği · gizli jetonlar · gereksiz kişisel veri. Bu alan zincire girdiği için
    /// **kalıcıdır**: bir kez yazılan bir sır, silinse bile özeti bozar.
    pub details: serde_json::Value,

    /// Bu olayın özeti — zincirin bir sonraki halkasının girdisi.
    pub event_hash: String,
}

/// Zinciri kuran ve doğrulayan yapı.
#[derive(Debug, Default)]
pub struct AuditChain {
    events: Vec<AuditEvent>,
}

/// Zincir doğrulaması başarısız.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChainError {
    /// Bir olayın kendi özeti, içeriğinden hesaplanana uymuyor → **değiştirilmiş**.
    #[error("olay {index} ({id}) DEĞİŞTİRİLMİŞ: kayıtlı özet {recorded}, hesaplanan {computed}")]
    Tampered {
        index: usize,
        id: String,
        recorded: String,
        computed: String,
    },
    /// Bir olayın `previous_event_hash`i, öncekinin özetine uymuyor → **halka kopuk**.
    #[error(
        "olay {index} ({id}) zinciri KOPUK: beklenen önceki özet {expected:?}, kayıtlı {recorded:?}"
    )]
    Broken {
        index: usize,
        id: String,
        expected: Option<String>,
        recorded: Option<String>,
    },
}

impl AuditChain {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Zincire yeni olay ekler ve özetini **hesaplayarak** döndürür.
    ///
    /// Çağıran `event_hash` ve `previous_event_hash` **veremez**: onları dışarıdan
    /// almak, zinciri kuran mekanizmanın kendisini atlanabilir kılardı.
    #[expect(
        clippy::too_many_arguments,
        reason = "olay alanları denetim kaydının listesi"
    )]
    pub fn append(
        &mut self,
        id: impl Into<String>,
        transaction_id: impl Into<String>,
        batch_id: impl Into<String>,
        package_id: Option<String>,
        file_id: Option<String>,
        signature_method: impl Into<String>,
        client_type: impl Into<String>,
        event_type: impl Into<String>,
        timestamp: impl Into<String>,
        details: serde_json::Value,
    ) -> &AuditEvent {
        let previous = self.events.last().map(|e| e.event_hash.clone());
        let mut olay = AuditEvent {
            id: id.into(),
            previous_event_hash: previous,
            transaction_id: transaction_id.into(),
            batch_id: batch_id.into(),
            package_id,
            file_id,
            signature_method: signature_method.into(),
            client_type: client_type.into(),
            event_type: event_type.into(),
            timestamp: timestamp.into(),
            details,
            event_hash: String::new(),
        };
        olay.event_hash = hash_of(&olay);
        self.events.push(olay);
        // İfade `push`tan sonra: son eleman az önce eklenen.
        self.events
            .last()
            .unwrap_or_else(|| unreachable!("az önce eklendi"))
    }

    /// Zincirin tamamını doğrular.
    ///
    /// # Errors
    /// İlk bozuk halkada durur ve **hangi olayın** bozuk olduğunu söyler.
    pub fn verify(&self) -> Result<(), ChainError> {
        let mut beklenen_onceki: Option<String> = None;
        for (i, olay) in self.events.iter().enumerate() {
            if olay.previous_event_hash != beklenen_onceki {
                return Err(ChainError::Broken {
                    index: i,
                    id: olay.id.clone(),
                    expected: beklenen_onceki,
                    recorded: olay.previous_event_hash.clone(),
                });
            }
            let hesaplanan = hash_of(olay);
            if hesaplanan != olay.event_hash {
                return Err(ChainError::Tampered {
                    index: i,
                    id: olay.id.clone(),
                    recorded: olay.event_hash.clone(),
                    computed: hesaplanan,
                });
            }
            beklenen_onceki = Some(olay.event_hash.clone());
        }
        Ok(())
    }

    #[must_use]
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Olayın **kanonik** metni — özet bundan hesaplanır.
///
/// **JSON'dan hesaplanMAZ.** `serde_json` alan sırasını yapının tanım sırasından
/// alır; bir gün alanlar yeniden sıralanırsa eski kayıtların özeti tutmaz ve zincir
/// **sahte bir alarm** verir — ki bu, gerçek bir alarmdan daha kötüdür: kimse ona
/// güvenmez.
///
/// Ayraç `\u{1e}` (record separator): alan değerlerinde geçme ihtimali pratikte
/// yok. Boru (`|`) ya da virgül gibi bir ayraç seçilseydi, bir alanın içindeki aynı
/// karakter iki farklı olayı aynı metne indirebilirdi.
fn canonical(e: &AuditEvent) -> String {
    const RS: char = '\u{1e}';
    let mut s = String::new();
    s.push_str(e.previous_event_hash.as_deref().unwrap_or(""));
    s.push(RS);
    s.push_str(&e.id);
    s.push(RS);
    s.push_str(&e.transaction_id);
    s.push(RS);
    s.push_str(&e.batch_id);
    s.push(RS);
    s.push_str(e.package_id.as_deref().unwrap_or(""));
    s.push(RS);
    s.push_str(e.file_id.as_deref().unwrap_or(""));
    s.push(RS);
    s.push_str(&e.signature_method);
    s.push(RS);
    s.push_str(&e.client_type);
    s.push(RS);
    s.push_str(&e.event_type);
    s.push(RS);
    s.push_str(&e.timestamp);
    s.push(RS);
    // `details` serbest JSON; onu da kanonik yapmak için sıralı yazıyoruz.
    s.push_str(&canonical_json(&e.details));
    s
}

/// JSON'u **anahtarları sıralı** biçimde yazar.
fn canonical_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(m) => {
            let mut anahtarlar: Vec<&String> = m.keys().collect();
            anahtarlar.sort_unstable();
            let ic: Vec<String> = anahtarlar
                .into_iter()
                .map(|k| format!("{k}:{}", canonical_json(&m[k])))
                .collect();
            format!("{{{}}}", ic.join(","))
        }
        serde_json::Value::Array(a) => {
            let ic: Vec<String> = a.iter().map(canonical_json).collect();
            format!("[{}]", ic.join(","))
        }
        serde_json::Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    }
}

fn hash_of(e: &AuditEvent) -> String {
    hex::encode(Sha256::digest(canonical(e).as_bytes()))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test içinde başarısızlık zaten çökmeli"
)]
mod tests {
    use super::*;

    fn zincir_kur() -> AuditChain {
        let mut z = AuditChain::new();
        for (i, tur) in ["batch_created", "signing_started", "file_signed"]
            .into_iter()
            .enumerate()
        {
            z.append(
                format!("e{i}"),
                "tx-1",
                "b-1",
                Some("p-1".into()),
                Some(format!("f-{i}")),
                "ELECTRONIC_SIGNATURE",
                "WEB",
                tur,
                format!("2026-08-28T10:0{i}:00Z"),
                serde_json::json!({ "sira": i }),
            );
        }
        z
    }

    #[test]
    fn saglam_zincir_dogrulanir() {
        let z = zincir_kur();
        assert_eq!(z.len(), 3);
        assert!(z.verify().is_ok());
        // İlk olayın öncesi YOK; sonrakiler öncekine bağlı.
        assert!(z.events()[0].previous_event_hash.is_none());
        assert_eq!(
            z.events()[1].previous_event_hash.as_deref(),
            Some(z.events()[0].event_hash.as_str())
        );
    }

    #[test]
    /// Ortadaki bir olayın **içeriği değiştirilirse** tespit edilir.
    fn degistirilen_olay_yakalanir() {
        let mut z = zincir_kur();
        // Saldırgan olayın türünü değiştiriyor: "file_failed" → "file_signed" gibi.
        z.events[1].event_type = "file_signed".into();

        let hata = z.verify().unwrap_err();
        match hata {
            ChainError::Tampered { index, ref id, .. } => {
                assert_eq!(index, 1);
                assert_eq!(id, "e1");
            }
            other => panic!("beklenen Tampered, gelen {other:?}"),
        }
    }

    #[test]
    /// Ortadaki bir olay **silinirse** halka kopar.
    fn silinen_olay_zinciri_kirar() {
        let mut z = zincir_kur();
        z.events.remove(1);
        let hata = z.verify().unwrap_err();
        assert!(matches!(hata, ChainError::Broken { index: 1, .. }));
    }

    #[test]
    /// Değiştirilen olayın özeti **düzeltilse bile** zincir kopar.
    fn ozeti_duzeltilen_olay_da_yakalanir() {
        // Saldırgan akıllıysa özeti yeniden hesaplar. O zaman kendi halkası
        //    tutar ama BİR SONRAKİNİN `previous_event_hash`i tutmaz — zincirin
        //    değeri tam olarak budur.
        let mut z = zincir_kur();
        z.events[1].event_type = "file_signed".into();
        z.events[1].event_hash = hash_of(&z.events[1]);

        let hata = z.verify().unwrap_err();
        assert!(matches!(hata, ChainError::Broken { index: 2, .. }));
    }

    #[test]
    /// Kanonik metin **alan sırasından bağımsız** olmalı (details JSON'u).
    fn details_anahtar_sirasi_ozeti_degistirmez() {
        let mut a = AuditChain::new();
        a.append(
            "e",
            "tx",
            "b",
            None,
            None,
            "ELECTRONIC_SIGNATURE",
            "WEB",
            "t",
            "2026-08-28T10:00:00Z",
            serde_json::json!({ "b": 2, "a": 1 }),
        );
        let mut b = AuditChain::new();
        b.append(
            "e",
            "tx",
            "b",
            None,
            None,
            "ELECTRONIC_SIGNATURE",
            "WEB",
            "t",
            "2026-08-28T10:00:00Z",
            serde_json::json!({ "a": 1, "b": 2 }),
        );
        assert_eq!(a.events()[0].event_hash, b.events()[0].event_hash);
    }

    #[test]
    /// Boş zincir geçerlidir — henüz olay yok demek, bozuk demek değil.
    fn bos_zincir_gecerli() {
        assert!(AuditChain::new().verify().is_ok());
    }
}
