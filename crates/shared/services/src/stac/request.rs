//! A search's request (docs/adr/0528 §3): `POST <endpoint>/search` with a JSON body of the STAC API's item search
//! (`bbox`, `collections`, `datetime`, `ids`, `limit`, and the cloud cover as the query extension's
//! `{"eo:cloud_cover": {"lte": n}}`), or the request a `next` link names. The body is compact JSON with its keys
//! in alphabetical order, integral numbers without a point (`canonical`). A `next` link is taken only when it
//! leads to the endpoint's own host: a server's word never sends a request somewhere else.

use kentos_contracts::{
    STAC_DEFAULT_LIMIT, STAC_MOST_LIMIT, StacMethod, StacNext, StacRequestInfo, StacSearch, codes,
};
use serde_json::{Map, Value, json};

use super::{Problem, Request, canonical, datetime, endpoint};

/// What a search asks: the request, the endpoint (normalized) and the filter the answer is checked against.
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub endpoint: String,
    pub request: Request,
    pub max_cloud_cover: Option<f64>,
}

impl Planned {
    /// The request as the command answers it.
    pub fn info(&self) -> StacRequestInfo {
        StacRequestInfo {
            method: self.request.method,
            url: self.request.url.clone(),
            body: self.request.body.clone(),
        }
    }
}

fn number(x: f64) -> Value {
    if x.is_finite() && x == x.trunc() && x.abs() < 1e15 {
        json!(x as i64)
    } else {
        json!(x)
    }
}

fn bbox_problem(b: &[f64; 4]) -> Option<String> {
    let [w, s, e, n] = *b;
    if !b.iter().all(|v| v.is_finite()) {
        return Some("Kutunun her sayısı sonlu olmalı.".into());
    }
    if !(-180.0..=180.0).contains(&w) || !(-180.0..=180.0).contains(&e) {
        return Some(format!(
            "Boylam −180 ile 180 arasında olmalı (batı {w}, doğu {e})."
        ));
    }
    if !(-90.0..=90.0).contains(&s) || !(-90.0..=90.0).contains(&n) {
        return Some(format!(
            "Enlem −90 ile 90 arasında olmalı (güney {s}, kuzey {n})."
        ));
    }
    if w >= e {
        return Some(
            "Batı doğudan küçük olmalı; 180° meridyenini aşan kutu iki aramaya bölünür.".into(),
        );
    }
    if s > n {
        return Some("Güney kuzeyden büyük olamaz.".into());
    }
    None
}

fn id_problem(what: &str, text: &str) -> Option<String> {
    (text.trim().is_empty() || text.chars().any(char::is_control))
        .then(|| format!("{what} boş olmamalı ve denetim karakteri içermemeli."))
}

/// The search's request, or the first thing wrong with it (the order of `StacSearch`'s refusals).
pub fn plan(input: &StacSearch) -> Result<Planned, Problem> {
    let endpoint = endpoint::normalize(&input.endpoint)
        .map_err(|m| Problem::new(codes::INVALID_STAC_ENDPOINT, m, "endpoint"))?;
    if let Some(b) = &input.bbox
        && let Some(why) = bbox_problem(b)
    {
        return Err(Problem::new(codes::INVALID_STAC_BBOX, why, "bbox"));
    }
    let when = match &input.datetime {
        Some(text) => Some(
            datetime::normalize(text)
                .map_err(|m| Problem::new(codes::INVALID_STAC_DATETIME, m, "datetime"))?,
        ),
        None => None,
    };
    if let Some(c) = input.max_cloud_cover
        && !(c.is_finite() && (0.0..=100.0).contains(&c))
    {
        return Err(Problem::new(
            codes::INVALID_STAC_CLOUD_COVER,
            format!("En çok bulut oranı 0 ile 100 arasında olmalı; {c} verildi."),
            "maxCloudCover",
        ));
    }
    if let Some(l) = input.limit
        && !(1..=STAC_MOST_LIMIT).contains(&l)
    {
        return Err(Problem::new(
            codes::INVALID_STAC_LIMIT,
            format!("Sayfa boyu 1 ile {STAC_MOST_LIMIT} arasında olmalı; {l} verildi."),
            "limit",
        ));
    }
    for (i, c) in input.collections.iter().enumerate() {
        if let Some(why) = id_problem("Koleksiyon kimliği", c) {
            return Err(Problem::new(
                codes::INVALID_STAC_COLLECTION,
                why,
                &format!("collections[{i}]"),
            ));
        }
    }
    for (i, id) in input.ids.iter().enumerate() {
        if let Some(why) = id_problem("Öğe kimliği", id) {
            return Err(Problem::new(
                codes::INVALID_STAC_COLLECTION,
                why,
                &format!("ids[{i}]"),
            ));
        }
    }
    let mut body = Map::new();
    if let Some(b) = &input.bbox {
        body.insert(
            "bbox".into(),
            Value::Array(b.iter().map(|v| number(*v)).collect()),
        );
    }
    if !input.collections.is_empty() {
        let list = input.collections.iter().map(|c| json!(c.trim())).collect();
        body.insert("collections".into(), Value::Array(list));
    }
    if let Some(w) = when {
        body.insert("datetime".into(), json!(w));
    }
    if !input.ids.is_empty() {
        let list = input.ids.iter().map(|c| json!(c.trim())).collect();
        body.insert("ids".into(), Value::Array(list));
    }
    body.insert(
        "limit".into(),
        json!(input.limit.unwrap_or(STAC_DEFAULT_LIMIT)),
    );
    if let Some(c) = input.max_cloud_cover {
        body.insert(
            "query".into(),
            json!({ "eo:cloud_cover": { "lte": number(c) } }),
        );
    }
    let request = match &input.next {
        None => Request {
            method: StacMethod::Post,
            url: format!("{endpoint}/search"),
            body: Some(canonical(&Value::Object(body))),
        },
        Some(next) => follow(&endpoint, next, body)?,
    };
    Ok(Planned {
        endpoint,
        request,
        max_cloud_cover: input.max_cloud_cover,
    })
}

/// The request a `next` link names, its body merged over the search's own when the link says so.
fn follow(endpoint: &str, next: &StacNext, own: Map<String, Value>) -> Result<Request, Problem> {
    let href = next.href.trim();
    if !endpoint::same_origin(endpoint, href) {
        return Err(Problem::new(
            codes::STAC_NEXT_FOREIGN,
            format!(
                "Sonraki sayfanın adresi (“{href}”) uç noktanın sunucusunda ({endpoint}) değil."
            ),
            "next.href",
        ));
    }
    match next.method {
        StacMethod::Get => Ok(Request {
            method: StacMethod::Get,
            url: href.to_owned(),
            body: None,
        }),
        StacMethod::Post => {
            let mut body = if next.merge { own } else { Map::new() };
            if let Some(Value::Object(theirs)) = &next.body {
                for (k, v) in theirs {
                    body.insert(k.clone(), v.clone());
                }
            }
            Ok(Request {
                method: StacMethod::Post,
                url: href.to_owned(),
                body: Some(canonical(&Value::Object(body))),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> StacSearch {
        StacSearch {
            endpoint: "https://earth-search.aws.element84.com/v1/".into(),
            bbox: Some([36.9, 39.7, 37.1, 39.8]),
            datetime: Some("2024-07-01/2024-07-20".into()),
            collections: vec!["sentinel-2-l2a".into()],
            max_cloud_cover: Some(20.0),
            limit: Some(2),
            ..StacSearch::default()
        }
    }

    #[test]
    fn a_search_is_one_post_with_a_sorted_body() {
        let p = plan(&base()).unwrap();
        assert_eq!(p.request.method, StacMethod::Post);
        assert_eq!(
            p.request.url,
            "https://earth-search.aws.element84.com/v1/search"
        );
        assert_eq!(
            p.request.body.as_deref(),
            Some(
                "{\"bbox\":[36.9,39.7,37.1,39.8],\"collections\":[\"sentinel-2-l2a\"],\"datetime\":\"2024-07-01T00:00:00Z/2024-07-20T23:59:59Z\",\"limit\":2,\"query\":{\"eo:cloud_cover\":{\"lte\":20}}}"
            )
        );
    }

    #[test]
    fn the_first_problem_is_the_one_told() {
        let mut s = base();
        s.endpoint = "ftp://x".into();
        s.bbox = Some([1.0, 2.0, 0.0, 3.0]);
        assert_eq!(plan(&s).unwrap_err().code, codes::INVALID_STAC_ENDPOINT);
        s.endpoint = "https://x/v1".into();
        assert_eq!(plan(&s).unwrap_err().code, codes::INVALID_STAC_BBOX);
        s.bbox = None;
        s.datetime = Some("yarın".into());
        assert_eq!(plan(&s).unwrap_err().code, codes::INVALID_STAC_DATETIME);
        s.datetime = None;
        s.max_cloud_cover = Some(120.0);
        assert_eq!(plan(&s).unwrap_err().code, codes::INVALID_STAC_CLOUD_COVER);
        s.max_cloud_cover = None;
        s.limit = Some(0);
        assert_eq!(plan(&s).unwrap_err().code, codes::INVALID_STAC_LIMIT);
        s.limit = Some(101);
        assert_eq!(plan(&s).unwrap_err().code, codes::INVALID_STAC_LIMIT);
        s.limit = None;
        s.collections = vec![" ".into()];
        assert_eq!(plan(&s).unwrap_err().code, codes::INVALID_STAC_COLLECTION);
    }

    #[test]
    fn a_next_link_merges_or_replaces_and_stays_on_the_host() {
        let mut s = base();
        s.next = Some(StacNext {
            method: StacMethod::Post,
            href: "https://earth-search.aws.element84.com/v1/search".into(),
            body: Some(json!({ "next": "abc", "limit": 2 })),
            merge: true,
        });
        let p = plan(&s).unwrap();
        let body: Value = serde_json::from_str(p.request.body.as_deref().unwrap()).unwrap();
        assert_eq!(body["next"], "abc");
        assert_eq!(body["bbox"][0], 36.9);
        s.next.as_mut().unwrap().merge = false;
        let p = plan(&s).unwrap();
        assert_eq!(
            p.request.body.as_deref(),
            Some("{\"limit\":2,\"next\":\"abc\"}")
        );
        s.next.as_mut().unwrap().method = StacMethod::Get;
        s.next.as_mut().unwrap().href =
            "https://earth-search.aws.element84.com/v1/search?token=x".into();
        let p = plan(&s).unwrap();
        assert_eq!(p.request.method, StacMethod::Get);
        assert_eq!(p.request.body, None);
        s.next.as_mut().unwrap().href = "https://evil.example/v1/search".into();
        assert_eq!(plan(&s).unwrap_err().code, codes::STAC_NEXT_FOREIGN);
    }
}
