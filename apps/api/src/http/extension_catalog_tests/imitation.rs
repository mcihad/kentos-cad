//! The publisher gate's imitation rule, run by the catalog when a version is sent (docs/adr/8059 §14, D13): another
//! publisher's version that reads as a listed extension goes back to its author at once; an honest name goes on. On
//! both stores.

use super::*;

on_both_stores!(an_imitation_of_a_listed_extension_goes_back_when_it_is_sent);

/// The other publisher: Kerem publishes for it.
const OTHER: &str = "baska-yazilim";

/// Kerem's package: the example extension under another id, name and publisher.
fn others(id: &str, name: &str) -> Vec<u8> {
    kek_with(|m| {
        m["id"] = json!(id);
        m["name"] = json!(name);
        m["publisherId"] = json!(OTHER);
    })
}

/// Uploaded by Kerem, every gate clean, sent by his program: the answer to sending.
async fn sent(app: &Router, id: &str, name: &str) -> Value {
    let (status, view) = upload(app, PUBLIC, "kerem", others(id, name)).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let at = format!("{PUBLIC}/{id}/1.0.0");
    scan_all(app, &at, &package_of(&view)).await;
    let (status, view) = act(
        app,
        Via::Python,
        "kerem",
        &at,
        json!({ "action": "submit" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

async fn an_imitation_of_a_listed_extension_goes_back_when_it_is_sent(bench: Bench) {
    let app = catalog(&bench).await;
    bench
        .grant(Where::In(CatalogId::Public), "kerem", Role::Publisher)
        .await;
    bench.join("kerem", OTHER).await;
    flow::reviewed_and_published(&app, "1.0.0", release("1.0.0")).await;
    // The listed extension's name in capitals, with a dotless I: it reads the same.
    let view = sent(&app, "tr.baska.altlik", "ÖRNEK BELEDİYESİ ALTLIKLARI").await;
    assert_eq!(state_of(&view), "changesRequested", "{view}");
    let gates = view["release"]["gates"].as_array().expect("gates");
    let publisher = gates
        .iter()
        .find(|g| g["kind"] == "publisher")
        .expect("the publisher gate");
    assert_eq!(publisher["status"], "blocked", "{publisher}");
    let finding = &publisher["findings"][0];
    assert_eq!(finding["code"], "imitates_name", "{finding}");
    assert_eq!(finding["dependency"], "tr.ornek.altlik");
    assert!(
        finding["message"]
            .as_str()
            .is_some_and(|m| m.contains("“Örnek Belediyesi altlıkları” (tr.ornek.altlik)")),
        "{finding}"
    );
    // An honest name goes on to review.
    let view = sent(&app, "tr.baska.sergi", "Başka Yazılım sergisi").await;
    assert_eq!(state_of(&view), "submitted", "{view}");
}
