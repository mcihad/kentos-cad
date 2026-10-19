//! `cad.stac.search` v1 (docs/adr/0528 §2, §3; plan/gis.md `CBS-31`): a STAC API's items that cover an area in a time,
//! with their assets. Read-only. The request and what comes back are the services core's rules
//! (`kentos_services::stac`); the fetch is the host's (`stac_host`): a host with no network answers `stac_unavailable`.
//! The shared cases are `fixtures/commands/v1/cad.stac.search.json` (`scripts/fixtures/stac_cases.py`, recorded
//! answers in `fixtures/services/v1/stac`).

use kentos_contracts::{CommandError, CommandResult, StacSearch, StacSearched, codes};
use kentos_services::stac::{Problem, read, request};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::stac_host;

fn refuse(p: Problem) -> Stop {
    Stop::failed(checks::error(p.code, p.message, p.path))
}

/// The search made and answered, as a host without a drawing (a window on its own thread) asks it; the refusal is the
/// command's own error.
pub fn search(input: &StacSearch) -> Result<StacSearched, Box<CommandError>> {
    asked(input).map_err(|stop| Box::new(stop.into_error()))
}

fn asked(input: &StacSearch) -> Result<StacSearched, Stop> {
    let planned = request::plan(input).map_err(refuse)?;
    let Some(answer) = stac_host::send(&planned.request) else {
        return Err(Stop::failed(checks::error(
            codes::STAC_UNAVAILABLE,
            "Bu ev sahibi ağa erişmiyor; STAC araması masaüstünde (ya da ağ erişimi veren bir ev sahibinde) yapılır.".into(),
            None,
        )));
    };
    let bytes = answer.map_err(|e| {
        Stop::failed(checks::error(
            codes::STAC_REQUEST_FAILED,
            format!(
                "{} {} isteği cevap vermedi: {}",
                method_text(&planned),
                planned.request.url,
                e.message
            ),
            None,
        ))
    })?;
    let page = read::read_page(&planned.endpoint, &bytes).map_err(|why| {
        Stop::failed(checks::error(
            codes::STAC_RESPONSE_UNREADABLE,
            format!("STAC sunucusunun cevabı okunamadı: {why}"),
            None,
        ))
    })?;
    let (items, dropped) = read::within_cloud(page.items, planned.max_cloud_cover);
    Ok(StacSearched {
        request: planned.info(),
        items,
        returned: page.returned,
        dropped,
        matched: page.matched,
        next: page.next,
    })
}

fn method_text(p: &request::Planned) -> &'static str {
    match p.request.method {
        kentos_contracts::StacMethod::Get => "GET",
        kentos_contracts::StacMethod::Post => "POST",
    }
}

/// Only the request is checked; nothing is sent.
pub fn validate(_cx: &ExecutionContext<'_>, input: &StacSearch) -> CommandResult<()> {
    match request::plan(input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(p) => refuse(p).into(),
    }
}

pub fn plan(_cx: &ExecutionContext<'_>, input: &StacSearch) -> CommandResult<StacSearched> {
    match asked(input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StacSearch) -> CommandResult<StacSearched> {
    plan(cx, &input)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This unit-test binary registers no fetcher: a host without a network says so (the shared cases register a
    /// recorded one in their own binary).
    #[test]
    fn a_host_without_a_network_says_so() {
        let input = StacSearch {
            endpoint: "https://earth-search.aws.element84.com/v1".into(),
            ..StacSearch::default()
        };
        match search(&input) {
            Err(error) => assert_eq!(error.code, codes::STAC_UNAVAILABLE),
            Ok(_) => panic!("answered without a network"),
        }
    }

    #[test]
    fn a_refused_request_never_reaches_the_network() {
        let input = StacSearch {
            endpoint: "ftp://x".into(),
            ..StacSearch::default()
        };
        match search(&input) {
            Err(error) => assert_eq!(error.code, codes::INVALID_STAC_ENDPOINT),
            Ok(_) => panic!("answered"),
        }
    }
}
