use crate::{contracts::*, workspace::error};
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};
pub struct Redactor {
    values: Vec<String>,
}
impl Redactor {
    pub fn new(values: impl IntoIterator<Item = String>) -> Self {
        let mut all = vec![];
        for v in values {
            if v.is_empty() {
                continue;
            }
            all.push(v.clone());
            for lower in [false, true] {
                let encoded = v
                    .as_bytes()
                    .iter()
                    .map(|b| {
                        if b.is_ascii_alphanumeric() || b"-_.~".contains(b) {
                            (*b as char).to_string()
                        } else if lower {
                            format!("%{b:02x}")
                        } else {
                            format!("%{b:02X}")
                        }
                    })
                    .collect::<String>();
                all.push(encoded);
            }
        }
        all.sort_by_key(|s| std::cmp::Reverse(s.len()));
        all.dedup();
        Self { values: all }
    }
    pub fn redact(&self, input: &str) -> String {
        let mut text = input.to_owned();
        for value in &self.values {
            text = text.replace(value, "[REDACTED]");
        }
        let mut scan = 0;
        while let Some(relative) = text[scan..].find("://") {
            let start = scan + relative + 3;
            let end = text[start..]
                .find(|c: char| c.is_whitespace() || ['/', '?', '#'].contains(&c))
                .map_or(text.len(), |i| start + i);
            if let Some(at) = text[start..end].rfind('@') {
                let at = start + at;
                if let Some(colon) = text[start..at].find(':') {
                    let colon = start + colon;
                    text.replace_range(colon + 1..at, "[REDACTED]");
                    scan = colon + 1 + "[REDACTED]".len();
                    continue;
                }
            }
            scan = start;
        }
        // Common key/value log forms. This is deliberately conservative; diagnostics still show a risk notice.
        for key in [
            "password",
            "passwd",
            "pwd",
            "token",
            "secret",
            "authorization",
            "api_key",
            "apikey",
        ] {
            let mut from = 0;
            loop {
                let lower = text.to_ascii_lowercase();
                let Some(pos) = lower[from..].find(key).map(|p| from + p) else {
                    break;
                };
                let end = pos + key.len();
                let boundary = pos == 0 || !text.as_bytes()[pos - 1].is_ascii_alphanumeric();
                let tail = text[end..].trim_start_matches([' ', '\t', '"', '\'']);
                if !boundary || !tail.starts_with(['=', ':']) {
                    from = end;
                    continue;
                }
                let delimiter_at = text.len() - tail.len();
                let start = delimiter_at + 1;
                let value_start = start + text[start..].len() - text[start..].trim_start().len();
                if value_start >= text.len() {
                    break;
                }
                let first = text.as_bytes()[value_start];
                let (begin, finish) = if first == b'\'' || first == b'"' {
                    let begin = value_start + 1;
                    let mut escaped = false;
                    let finish = text[begin..]
                        .char_indices()
                        .find_map(|(offset, ch)| {
                            if escaped {
                                escaped = false;
                                return None;
                            }
                            if ch == '\\' {
                                escaped = true;
                                return None;
                            }
                            (ch == first as char).then_some(begin + offset)
                        })
                        .unwrap_or(text.len());
                    (begin, finish)
                } else {
                    let finish = text[value_start..]
                        .find(|c: char| {
                            (key != "authorization"
                                && (c.is_whitespace() || ['"', '\''].contains(&c)))
                                || [',', ';', '&', '}', '\r', '\n'].contains(&c)
                        })
                        .map_or(text.len(), |n| value_start + n);
                    (value_start, finish)
                };
                if begin == finish {
                    from = finish.saturating_add(1).min(text.len());
                    if from == text.len() {
                        break;
                    }
                    continue;
                }
                text.replace_range(begin..finish, "[REDACTED]");
                from = begin + "[REDACTED]".len();
                if from >= text.len() {
                    break;
                }
            }
        }
        text
    }
}
impl Drop for Redactor {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        for v in &mut self.values {
            v.zeroize();
        }
    }
}

const LINE_JSON_LIMIT: usize = 16 * 1024;
fn escaped_char_bytes(ch: char) -> usize {
    match ch {
        '"' | '\\' | '\n' | '\r' | '\t' | '\u{0008}' | '\u{000c}' => 2,
        '\u{0000}'..='\u{001f}' => 6,
        _ => ch.len_utf8(),
    }
}
fn line_json_size(line: &str) -> usize {
    2 + line.chars().map(escaped_char_bytes).sum::<usize>()
}
fn bound_line(mut line: String) -> String {
    const SUFFIX: &str = " [truncated]";
    let mut size = 2;
    let mut prefix = 0;
    for (offset, ch) in line.char_indices() {
        size += escaped_char_bytes(ch);
        if size + SUFFIX.len() <= LINE_JSON_LIMIT {
            prefix = offset + ch.len_utf8();
        }
        if size > LINE_JSON_LIMIT {
            line.truncate(prefix);
            line.push_str(SUFFIX);
            break;
        }
    }
    line
}
pub struct LogWindow {
    stream_id: String,
    next: u64,
    seq: u64,
    unacked: BTreeMap<u64, (usize, Instant)>,
    queued: VecDeque<(u64, String)>,
    bytes: usize,
    dropped: u64,
    last_send: Option<Instant>,
    expired: bool,
}
impl LogWindow {
    pub fn new(id: String) -> Self {
        Self {
            stream_id: id,
            next: 1,
            seq: 1,
            unacked: BTreeMap::new(),
            queued: VecDeque::new(),
            bytes: 0,
            dropped: 0,
            last_send: None,
            expired: false,
        }
    }
    pub fn push(&mut self, line: String) {
        if self.expired {
            return;
        }
        let line = bound_line(line);
        let seq = self.seq;
        self.seq += 1;
        self.bytes += line_json_size(&line);
        self.queued.push_back((seq, line));
        while self.bytes > 2 * 1024 * 1024 || self.queued.len() > 2000 {
            if let Some((_, line)) = self.queued.pop_front() {
                self.bytes -= line_json_size(&line);
                self.dropped += 1;
            }
        }
    }
    fn expire(&mut self) {
        self.expired = true;
        self.unacked.clear();
        self.queued.clear();
        self.bytes = 0;
    }
    pub fn batch(&mut self, now: Instant) -> Result<Option<LogBatch>, ToolboxError> {
        if self.expired {
            return Err(error("LogStreamExpired"));
        }
        if let Some(oldest) = self.unacked.values().map(|(_, at)| *at).min() {
            let age = now.saturating_duration_since(oldest);
            if age >= Duration::from_secs(30) {
                self.expire();
                return Err(error("LogStreamExpired"));
            }
            if age >= Duration::from_secs(10) {
                return Err(error("AwaitingConsumer"));
            }
        }
        if self.unacked.len() >= 2
            || self.unacked.values().map(|(size, _)| size).sum::<usize>() >= 128 * 1024
            || self.last_send.is_some_and(|last| {
                now.saturating_duration_since(last) < Duration::from_millis(100)
            })
            || self.queued.is_empty()
        {
            return Ok(None);
        }
        let mut batch = LogBatch {
            stream_id: self.stream_id.clone(),
            batch_id: self.next,
            from_seq: self.queued.front().map_or(self.seq, |v| v.0),
            lines: vec![],
            dropped_count: self.dropped,
        };
        while batch.lines.len() < 100 {
            let Some((_, line)) = self.queued.front() else {
                break;
            };
            batch.lines.push(line.clone());
            let size = serde_json::to_vec(&batch)
                .map_err(|_| error("LogSerializationFailed"))?
                .len();
            if size > 64 * 1024 {
                batch.lines.pop();
                break;
            }
            let (_, line) = self
                .queued
                .pop_front()
                .ok_or_else(|| error("LogStreamCorrupt"))?;
            self.bytes -= line_json_size(&line);
        }
        if batch.lines.is_empty() {
            return Err(error("LogLineTooLarge"));
        }
        let size = serde_json::to_vec(&batch)
            .map_err(|_| error("LogSerializationFailed"))?
            .len();
        self.unacked.insert(self.next, (size, now));
        self.next += 1;
        self.last_send = Some(now);
        self.dropped = 0; // Per-delivered-batch delta; the frontend sums these values.
        Ok(Some(batch))
    }
    pub fn ack(&mut self, ack: &LogAck) -> Result<(), ToolboxError> {
        self.ack_at(ack, Instant::now())
    }
    fn ack_at(&mut self, ack: &LogAck, now: Instant) -> Result<(), ToolboxError> {
        if self.expired
            || self
                .unacked
                .values()
                .any(|(_, at)| now.saturating_duration_since(*at) >= Duration::from_secs(30))
        {
            self.expire();
            return Err(error("LogStreamExpired"));
        }
        if ack.stream_id != self.stream_id || self.unacked.remove(&ack.batch_id).is_none() {
            return Err(error("InvalidLogAcknowledgement"));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logs_redact_raw_encoded_and_connection_passwords() {
        let r = Redactor::new(["secret@!".into()]);
        let line = r.redact("secret@! secret%40%21 mysql://user:other@db/service");
        assert!(!line.contains("secret"));
        assert!(!line.contains("other"));
        assert!(line.contains("[REDACTED]"));
    }
    #[test]
    fn ack_window_is_bounded_and_unknown_ack_does_not_give_credit() {
        let mut w = LogWindow::new("stream".into());
        for _ in 0..500 {
            w.push("hello".into());
        }
        let start = Instant::now();
        let a = w.batch(start).unwrap().unwrap();
        let b = w
            .batch(start + Duration::from_millis(101))
            .unwrap()
            .unwrap();
        assert_eq!(a.lines.len(), 100);
        assert!(
            w.batch(start + Duration::from_millis(202))
                .unwrap()
                .is_none()
        );
        assert!(
            w.ack(&LogAck {
                stream_id: "stream".into(),
                batch_id: 999
            })
            .is_err()
        );
        w.ack(&LogAck {
            stream_id: "stream".into(),
            batch_id: a.batch_id,
        })
        .unwrap();
        assert!(
            w.ack(&LogAck {
                stream_id: "stream".into(),
                batch_id: a.batch_id
            })
            .is_err()
        );
        assert!(
            w.batch(start + Duration::from_millis(303))
                .unwrap()
                .is_some()
        );
        assert_ne!(a.batch_id, b.batch_id);
    }
    #[test]
    fn no_ack_times_out_and_ring_cannot_grow_without_bound() {
        let mut w = LogWindow::new("s".into());
        for _ in 0..3000 {
            w.push("x".repeat(2000));
        }
        assert!(w.bytes <= 2 * 1024 * 1024);
        assert!(w.queued.len() <= 2000);
        let at = Instant::now();
        assert!(w.batch(at).unwrap().is_some());
        assert_eq!(
            w.batch(at + Duration::from_secs(11)).unwrap_err().code,
            "AwaitingConsumer"
        );
        assert_eq!(
            w.batch(at + Duration::from_secs(31)).unwrap_err().code,
            "LogStreamExpired"
        );
    }
}
#[cfg(test)]
mod authorization_tests {
    use super::*;
    #[test]
    fn authorization_schemes_never_leave_unknown_credentials_in_logs() {
        let r = Redactor::new([]);
        for line in [
            "Authorization: Bearer unknown-credential",
            "Proxy-Authorization: Basic abcdef==",
            "authorization=Bearer unknown-credential; next=public",
        ] {
            let value = r.redact(line);
            assert!(!value.contains("unknown-credential"));
            assert!(!value.contains("abcdef=="));
            assert!(value.contains("[REDACTED]"));
        }
    }
}

#[cfg(test)]
mod bounded_delivery_tests {
    use super::*;
    #[test]
    fn json_escaped_control_lines_fit_line_and_batch_budgets_and_keep_stream_usable() {
        let mut window = LogWindow::new("stream".into());
        for text in [
            "\0".repeat(16 * 1024),
            "\\\"\n".repeat(16 * 1024),
            "界".repeat(16 * 1024),
        ] {
            window.push(text);
        }
        let at = Instant::now();
        let batch = window.batch(at).unwrap().unwrap();
        assert!(!batch.lines.is_empty());
        assert!(serde_json::to_vec(&batch).unwrap().len() <= 64 * 1024);
        for line in &batch.lines {
            assert!(serde_json::to_vec(line).unwrap().len() <= 16 * 1024);
            assert!(line.ends_with(" [truncated]"));
        }
        window
            .ack(&LogAck {
                stream_id: "stream".into(),
                batch_id: batch.batch_id,
            })
            .unwrap();
        window.push("next normal line".into());
        assert!(
            window
                .batch(at + Duration::from_millis(100))
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn dropped_count_is_a_delta_and_ring_is_bounded() {
        let mut window = LogWindow::new("stream".into());
        for _ in 0..2001 {
            window.push("line".into());
        }
        assert_eq!(window.queued.len(), 2000);
        let at = Instant::now();
        let first = window.batch(at).unwrap().unwrap();
        assert_eq!(first.dropped_count, 1);
        assert_eq!(first.from_seq, 2);
        window
            .ack(&LogAck {
                stream_id: "stream".into(),
                batch_id: first.batch_id,
            })
            .unwrap();
        let next = window
            .batch(at + Duration::from_millis(100))
            .unwrap()
            .unwrap();
        assert_eq!(next.dropped_count, 0);
        for _ in 0..3000 {
            window.push("\0".repeat(16 * 1024));
        }
        assert!(window.bytes <= 2 * 1024 * 1024);
        assert!(window.queued.len() <= 2000);
    }
    #[test]
    fn expiration_is_terminal_and_a_late_ack_cannot_restore_credit() {
        let mut window = LogWindow::new("stream".into());
        window.push("line".into());
        let at = Instant::now();
        let batch = window.batch(at).unwrap().unwrap();
        assert_eq!(
            window.batch(at + Duration::from_secs(10)).unwrap_err().code,
            "AwaitingConsumer"
        );
        assert_eq!(
            window.batch(at + Duration::from_secs(30)).unwrap_err().code,
            "LogStreamExpired"
        );
        assert!(
            window
                .ack(&LogAck {
                    stream_id: "stream".into(),
                    batch_id: batch.batch_id
                })
                .is_err()
        );
        window.push("new line".into());
        assert_eq!(
            window.batch(at + Duration::from_secs(31)).unwrap_err().code,
            "LogStreamExpired"
        );
    }
    #[test]
    fn redaction_handles_json_escaped_quotes_in_unknown_authorization_values() {
        let redactor = Redactor::new([]);
        for text in [
            r#"{"Authorization":"Bearer unknown\"credential","safe":"shown"}"#,
            r#"{'Proxy-Authorization':'Basic unknown\'credential','safe':'shown'}"#,
        ] {
            let result = redactor.redact(text);
            assert!(!result.contains("unknown"));
            assert!(!result.contains("credential"));
            assert!(result.contains("shown"));
        }
    }
}

#[cfg(test)]
mod final_boundaries {
    use super::*;
    #[test]
    fn delivery_never_exceeds_ten_batches_per_second_even_with_immediate_acks() {
        let mut window = LogWindow::new("stream".into());
        let at = Instant::now();
        let mut sent = 0;
        for millis in (0..1000).step_by(10) {
            window.push("line".into());
            let now = at + Duration::from_millis(millis);
            if let Some(batch) = window.batch(now).unwrap() {
                sent += 1;
                assert!(batch.lines.len() <= 100);
                assert!(serde_json::to_vec(&batch).unwrap().len() <= 64 * 1024);
                window
                    .ack_at(
                        &LogAck {
                            stream_id: "stream".into(),
                            batch_id: batch.batch_id,
                        },
                        now,
                    )
                    .unwrap();
            }
        }
        assert_eq!(sent, 10);
    }
    #[test]
    fn two_unacked_batches_stay_within_byte_credit_and_only_valid_ack_resumes() {
        let mut window = LogWindow::new("stream".into());
        for _ in 0..500 {
            window.push("x".repeat(16384));
        }
        let at = Instant::now();
        let first = window.batch(at).unwrap().unwrap();
        let second = window
            .batch(at + Duration::from_millis(100))
            .unwrap()
            .unwrap();
        assert_eq!(window.unacked.len(), 2);
        assert!(
            window
                .unacked
                .values()
                .map(|(bytes, _)| bytes)
                .sum::<usize>()
                <= 128 * 1024
        );
        assert!(
            window
                .batch(at + Duration::from_millis(200))
                .unwrap()
                .is_none()
        );
        assert!(
            window
                .ack_at(
                    &LogAck {
                        stream_id: "other".into(),
                        batch_id: first.batch_id
                    },
                    at + Duration::from_secs(1)
                )
                .is_err()
        );
        assert_eq!(window.unacked.len(), 2);
        window
            .ack_at(
                &LogAck {
                    stream_id: "stream".into(),
                    batch_id: second.batch_id,
                },
                at + Duration::from_secs(1),
            )
            .unwrap();
        assert!(
            window
                .ack_at(
                    &LogAck {
                        stream_id: "stream".into(),
                        batch_id: second.batch_id
                    },
                    at + Duration::from_secs(1)
                )
                .is_err()
        );
        assert_eq!(
            window.batch(at + Duration::from_secs(10)).unwrap_err().code,
            "AwaitingConsumer"
        );
        window
            .ack_at(
                &LogAck {
                    stream_id: "stream".into(),
                    batch_id: first.batch_id,
                },
                at + Duration::from_secs(11),
            )
            .unwrap();
        assert!(
            window
                .batch(at + Duration::from_secs(11))
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn unknown_unquoted_authorization_masks_quotes_inside_the_whole_value() {
        let redactor = Redactor::new([]);
        for text in [
            "Authorization: Bearer unknown\\\"credential; safe=shown",
            "Proxy-Authorization: Basic unknown'credential; safe=shown",
        ] {
            let value = redactor.redact(text);
            assert!(!value.contains("unknown"));
            assert!(!value.contains("credential"));
            assert!(value.contains("shown"));
        }
    }
}
