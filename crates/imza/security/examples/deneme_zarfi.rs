//! Deneme zarfı: gerçek pencerede kabul ve elle deneme için sunucunun yaptığını yapar (docs/adr/9002 §6). Yalnız
//! `issue` özelliğiyle derlenir; ürünün hiçbir ikilisine girmez.
//!
//! ```bash
//! cargo run -p kentos-imza-security --features issue --example deneme_zarfi -- anahtar OZEL.txt   # açık anahtarı yazar
//! cargo run -p kentos-imza-security --features issue --example deneme_zarfi -- zarf OZEL.txt tx-1  # {"signedRequest":…}
//! ```
//!
//! Belgeler uydurmadır: adları ve içerikleri sabittir, özetleri o içeriklerden hesaplanır.

use std::process::ExitCode;

use kentos_imza_domain::{DigestAlgorithm, PayloadFile, PayloadPackage, SignedRequestPayload};
use kentos_imza_security::digest;
use kentos_imza_security::issue::{ServerKey, issue};

const BELGELER: [(&str, &str, bool); 4] = [
    ("f1", "Mimari proje paftaları.pdf", true),
    ("f2", "Statik hesap raporu.pdf", true),
    ("f3", "Zemin etüdü özeti.pdf", true),
    ("f4", "Bilgi notu.txt", false),
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["anahtar", dosya] => match ServerKey::generate() {
            Ok(k) => {
                if let Err(e) = std::fs::write(dosya, k.private_pkcs8_base64()) {
                    eprintln!("{dosya} yazılamadı: {e}");
                    return ExitCode::FAILURE;
                }
                println!("{}", k.public_base64());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", e.technical_message);
                ExitCode::FAILURE
            }
        },
        ["zarf", dosya, tx] => {
            let k = match std::fs::read_to_string(dosya)
                .map_err(|e| e.to_string())
                .and_then(|s| ServerKey::from_config(&s).map_err(|e| e.technical_message))
            {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("anahtar okunamadı: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let simdi = time::OffsetDateTime::now_utc();
            let yaz = |t: time::OffsetDateTime| {
                t.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default()
            };
            let payload = SignedRequestPayload {
                transaction_id: (*tx).to_owned(),
                batch_id: format!("{tx}-batch"),
                user_id: "deneme".into(),
                session_id: "deneme".into(),
                signature_method: "ELECTRONIC_SIGNATURE".into(),
                client_type: "WEB".into(),
                packages: vec![PayloadPackage {
                    id: "dosya-2026-0007".into(),
                    title: "Konut yapı ruhsatı başvurusu, 1244 ada 7 parsel".into(),
                    reference_number: Some("2026/0007".into()),
                    document_type: Some("basvuru".into()),
                    metadata: serde_json::json!({}),
                    files: BELGELER
                        .iter()
                        .map(|(id, ad, gerekli)| {
                            let icerik = format!("KentOS deneme belgesi: {ad}").into_bytes();
                            PayloadFile {
                                id: (*id).into(),
                                name: (*ad).into(),
                                role: if *gerekli {
                                    "MAIN_DOCUMENT"
                                } else {
                                    "ATTACHMENT"
                                }
                                .into(),
                                mime_type: "application/pdf".into(),
                                size: icerik.len() as u64 * 1000,
                                digest_algorithm: "SHA-384".into(),
                                digest: digest::compute(DigestAlgorithm::Sha384, &icerik),
                                signing_required: *gerekli,
                            }
                        })
                        .collect(),
                }],
                created_at: yaz(simdi),
                expires_at: yaz(simdi + time::Duration::minutes(15)),
                nonce: format!("{tx}-{}", simdi.unix_timestamp_nanos()),
                issuer: kentos_imza_security::DEFAULT_ISSUER.into(),
                audience: kentos_imza_security::DEFAULT_AUDIENCE.into(),
            };
            match issue(&payload, &k, None) {
                Ok(z) => {
                    println!("{}", serde_json::json!({ "signedRequest": z }));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", e.technical_message);
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("Kullanım: deneme_zarfi anahtar OZEL.txt | deneme_zarfi zarf OZEL.txt ISLEM");
            ExitCode::FAILURE
        }
    }
}
