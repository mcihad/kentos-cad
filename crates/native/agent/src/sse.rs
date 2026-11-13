//! Server-sent events, one parser for every provider (docs/adr/8056 §4; the research note's “Akış ayrıştırıcısı”).
//! Named events (`event:` + `data:`, Anthropic) and data-only lines (the others), several `data:` lines joined by a line
//! break, comment lines starting with `:` skipped (`: keep-alive`, `: OPENROUTER PROCESSING`), lines ended by `\n`,
//! `\r\n` or `\r`, chunks cut anywhere (in a line, in a character). `[DONE]` is a data like any other: the wire's
//! decoder knows it.

/// One event: its name (empty when the stream names none) and its data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SseEvent {
    pub event: String,
    pub data: String,
}

/// Turns the bytes of a stream, as they come, into events.
#[derive(Debug, Default)]
pub(crate) struct SseParser {
    /// Bytes of a line not ended yet.
    pending: Vec<u8>,
    event: String,
    data: String,
    has_data: bool,
}

impl SseParser {
    /// The events `bytes` completes.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();
        let mut start = 0;
        let mut i = 0;
        while i < self.pending.len() {
            match self.pending[i] {
                b'\n' => {
                    lines.push(self.pending[start..i].to_vec());
                    i += 1;
                    start = i;
                }
                b'\r' => {
                    // A `\r` last in the chunk may be the first half of `\r\n`: wait for the next byte.
                    let Some(&next) = self.pending.get(i + 1) else {
                        break;
                    };
                    lines.push(self.pending[start..i].to_vec());
                    i += if next == b'\n' { 2 } else { 1 };
                    start = i;
                }
                _ => i += 1,
            }
        }
        self.pending.drain(..start);
        let mut out = Vec::new();
        for line in lines {
            self.line(&line, &mut out);
        }
        out
    }

    /// The stream ended: what its last lines still hold (a last event without the blank line after it).
    pub(crate) fn finish(&mut self) -> Vec<SseEvent> {
        let mut out = Vec::new();
        let rest = std::mem::take(&mut self.pending);
        let rest = rest.strip_suffix(b"\r").unwrap_or(&rest);
        if !rest.is_empty() {
            self.line(rest, &mut out);
        }
        self.dispatch(&mut out);
        out
    }

    fn line(&mut self, line: &[u8], out: &mut Vec<SseEvent>) {
        if line.is_empty() {
            self.dispatch(out);
            return;
        }
        if line[0] == b':' {
            return;
        }
        let text = String::from_utf8_lossy(line);
        let (field, value) = match text.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (text.as_ref(), ""),
        };
        match field {
            "event" => value.clone_into(&mut self.event),
            "data" => {
                if self.has_data {
                    self.data.push('\n');
                }
                self.data.push_str(value);
                self.has_data = true;
            }
            _ => {}
        }
    }

    fn dispatch(&mut self, out: &mut Vec<SseEvent>) {
        if self.has_data {
            out.push(SseEvent {
                event: std::mem::take(&mut self.event),
                data: std::mem::take(&mut self.data),
            });
        }
        self.event.clear();
        self.data.clear();
        self.has_data = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut p = SseParser::default();
        let mut out = Vec::new();
        for c in chunks {
            out.extend(p.push(c));
        }
        out.extend(p.finish());
        out
    }

    fn ev(event: &str, data: &str) -> SseEvent {
        SseEvent {
            event: event.into(),
            data: data.into(),
        }
    }

    #[test]
    fn named_events_data_lines_comments_and_line_ends() {
        let text = "event: message_start\ndata: {\"a\":1}\n\n: keep-alive\n\n: OPENROUTER PROCESSING\r\n\r\ndata: x\r\ndata:y\r\rdata: [DONE]\n\n";
        assert_eq!(
            all(&[text.as_bytes()]),
            vec![
                ev("message_start", "{\"a\":1}"),
                ev("", "x\ny"),
                ev("", "[DONE]")
            ]
        );
        // An event without data is not one; a last event without its blank line still is.
        assert_eq!(
            all(&[b"event: ping\n\nevent: e\ndata: last"]),
            vec![ev("e", "last")]
        );
    }

    #[test]
    fn chunks_cut_anywhere_give_the_same_events() {
        let text = "event: a\r\ndata: çizim ğüşıöç\r\n\r\ndata: {\"b\":2}\n\n";
        let whole = all(&[text.as_bytes()]);
        assert_eq!(whole, vec![ev("a", "çizim ğüşıöç"), ev("", "{\"b\":2}")]);
        let bytes = text.as_bytes();
        for cut in 1..bytes.len() {
            assert_eq!(all(&[&bytes[..cut], &bytes[cut..]]), whole, "cut at {cut}");
        }
        let singles: Vec<&[u8]> = bytes.chunks(1).collect();
        assert_eq!(all(&singles), whole);
    }
}
