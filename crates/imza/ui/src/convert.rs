//! Alan modelinden pencerenin görünüm modeline (docs/adr/9002 §6): batch → istek, kart ve sertifika → seçenek.

use kentos_imza_domain::{CertificateInfo, DeviceInfo, SigningBatch};

use crate::model::{CertificateOption, DocState, Document, Request};

impl Request {
    /// Batch'in belgeleri, sırasıyla. Başlık batch'inki, sayı ilk evrakınki.
    #[must_use]
    pub fn from_batch(batch: &SigningBatch) -> Self {
        Self {
            id: batch.id.clone(),
            title: batch.title.clone(),
            reference: batch
                .packages
                .iter()
                .find_map(|p| p.reference_number.clone()),
            documents: batch
                .packages
                .iter()
                .flat_map(|p| p.files.iter())
                .map(|f| Document {
                    id: f.id.clone(),
                    name: f.name.clone(),
                    size: f.size,
                    digest_algorithm: f.digest_algorithm.name().to_owned(),
                    digest: f.digest.clone(),
                    signing_required: f.signing_required,
                    state: if f.signing_required {
                        DocState::Waiting
                    } else {
                        DocState::Informative
                    },
                })
                .collect(),
        }
    }
}

/// Sertifika imzalamaya uygun mu: anahtar kullanımında `nonRepudiation` ya da `digitalSignature`. Şifreleme
/// sertifikasını listelemek, kullanıcıya çalışmayacak bir seçenek sunmaktır.
#[must_use]
pub fn signing_capable(c: &CertificateInfo) -> bool {
    c.key_usage
        .iter()
        .any(|k| k == "nonRepudiation" || k == "digitalSignature")
}

impl CertificateOption {
    /// Bir karttaki sertifika. Sahibin adı konunun `CN`'idir: konunun tamamı (kimlik numarası dahil) pencerede
    /// gösterilmez, çünkü ekranı gören herkes onu okurdu.
    #[must_use]
    pub fn from_info(device: &DeviceInfo, c: &CertificateInfo) -> Self {
        let cihaz = if device.token_label.trim().is_empty() {
            format!("Kart {}", device.slot_id)
        } else {
            device.token_label.clone()
        };
        Self {
            slot_id: device.slot_id,
            device: cihaz,
            certificate_id: c.id.clone(),
            holder: common_name(&c.subject),
            issuer: common_name(&c.issuer),
            valid_until: day_of(&c.not_after),
            qualified: c.qualified,
            currently_valid: c.currently_valid,
            key: if c.key_parameters.is_empty() {
                c.key_algorithm.clone()
            } else {
                format!("{} {}", c.key_algorithm, c.key_parameters)
            },
        }
    }
}

/// Bütün kartlardaki imzalamaya uygun sertifikalar, kart sırasıyla.
#[must_use]
pub fn options(cards: &[(DeviceInfo, Vec<CertificateInfo>)]) -> Vec<CertificateOption> {
    cards
        .iter()
        .flat_map(|(d, list)| {
            list.iter()
                .filter(|c| signing_capable(c))
                .map(move |c| CertificateOption::from_info(d, c))
        })
        .collect()
}

/// Ayırt edici adın `CN`'i; yoksa adın tamamı.
#[must_use]
pub fn common_name(dn: &str) -> String {
    dn.split(',')
        .map(str::trim)
        .find_map(|p| p.strip_prefix("CN="))
        .map_or_else(|| dn.trim().to_owned(), str::to_owned)
}

/// RFC 2822 ya da RFC 3339 tarihin gün hâli: `05.05.2028`. Çözülemezse olduğu gibi.
#[must_use]
pub fn day_of(when: &str) -> String {
    use time::format_description::well_known::{Rfc2822, Rfc3339};
    let t = time::OffsetDateTime::parse(when, &Rfc2822)
        .or_else(|_| time::OffsetDateTime::parse(when, &Rfc3339));
    match t {
        Ok(t) => format!("{:02}.{:02}.{}", t.day(), u8::from(t.month()), t.year()),
        Err(_) => when.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ortak_ad_ve_gun() {
        assert_eq!(
            common_name("C=TR, serialNumber=DENEME-1, CN=Deneme İmzacı"),
            "Deneme İmzacı"
        );
        assert_eq!(common_name("O=Kurum"), "O=Kurum");
        assert_eq!(day_of("Wed, 05 May 2028 10:00:00 +0000"), "05.05.2028");
        assert_eq!(day_of("2036-01-01T00:00:00Z"), "01.01.2036");
        assert_eq!(day_of("bilinmiyor"), "bilinmiyor");
    }
}
