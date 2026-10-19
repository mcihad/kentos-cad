//! STAC requests over HTTP (docs/adr/0528 §2): the fetcher the hosts that reach a network (the desktop, the MCP
//! server) register for `cad.stac.search`. The request is the services core's (`kentos_services::stac`); this only
//! sends it on the cloud's runtime, waiting on the calling thread (never the interface's), at most four at once to
//! one host, and answers the body of a 2xx. Redirects are followed, a request gets 30 seconds, an answer larger
//! than 64 MB is refused. A catalog's credentials are not sent: the search is public, and a signed asset is not
//! fetched here.

use kentos_contracts::StacMethod;
use kentos_services::stac::{Fetch, FetchError, Request};

use crate::fetch::{self, Method};

/// The most bytes an answer may take.
const MOST: usize = 64 * 1024 * 1024;

/// The HTTP fetcher.
pub struct HttpFetch;

impl Fetch for HttpFetch {
    fn send(&self, request: &Request) -> Result<Vec<u8>, FetchError> {
        let post = request.method == StacMethod::Post;
        let mut headers = vec![(
            "accept".to_owned(),
            "application/geo+json, application/json;q=0.9".to_owned(),
        )];
        if post {
            headers.push(("content-type".to_owned(), "application/json".to_owned()));
        }
        let req = fetch::Request {
            method: if post { Method::Post } else { Method::Get },
            url: request.url.clone(),
            headers,
            body: request.body.as_ref().map(|b| b.clone().into_bytes()),
        };
        match fetch::wait(req, 4) {
            Ok(answer) if (200..300).contains(&answer.status) => {
                if answer.body.len() > MOST {
                    Err(FetchError {
                        message: "Cevap 64 MB'tan büyük.".into(),
                    })
                } else {
                    Ok(answer.body)
                }
            }
            Ok(answer) => Err(FetchError {
                message: format!("Sunucu HTTP {} dedi.", answer.status),
            }),
            Err(failure) => Err(FetchError {
                message: failure.message,
            }),
        }
    }
}
