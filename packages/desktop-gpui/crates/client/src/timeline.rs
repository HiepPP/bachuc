//! Timeline state of one viewed agent, kept in sync per `docs/timeline-sync.md`.
//!
//! Entries are projected timeline entries (`item`, `seqStart`, `seqEnd`, `sourceSeqRanges`,
//! `collapsed`, ...) ordered by `seqStart`. Live `agent_stream` rows merge into them with the
//! daemon's projection rules (`packages/server/src/server/agent/timeline-projection.ts`), so a
//! synced state equals a fresh fetch of the same range.

use serde_json::{Map, Value, json};

/// What the timeline view reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineState {
    /// Projected timeline entries in order.
    pub entries: Vec<Value>,
    /// Older entries exist before the first one; `Client::load_older` fetches them.
    pub has_older: bool,
    /// Grows on every change.
    pub version: u64,
    /// Open permission requests (`AgentPermissionRequestPayloadSchema` objects).
    pub pending_permissions: Vec<Value>,
}

/// A timeline fetch the sync wants sent. Cursors are `(epoch, seq)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchPlan {
    /// The latest page; it replaces the state.
    Tail,
    /// The next page after the end cursor, while closing a gap.
    After { epoch: String, seq: u64 },
    /// The first page after a reconnect. If more pages follow, the sync takes a tail instead.
    Resume { epoch: String, seq: u64 },
    /// The page before the first cursor.
    Before { epoch: String, seq: u64 },
}

impl FetchPlan {
    /// The `fetch_agent_timeline_request` fields for this plan.
    pub fn request(&self, agent_id: &str) -> Value {
        let cursor = |epoch: &str, seq: u64| json!({"epoch": epoch, "seq": seq});
        let (direction, cursor, limit) = match self {
            FetchPlan::Tail => ("tail", None, TAIL_LIMIT),
            FetchPlan::After { epoch, seq } | FetchPlan::Resume { epoch, seq } => {
                ("after", Some(cursor(epoch, *seq)), PAGE_LIMIT)
            }
            FetchPlan::Before { epoch, seq } => ("before", Some(cursor(epoch, *seq)), PAGE_LIMIT),
        };
        let mut request = json!({
            "type": "fetch_agent_timeline_request",
            "agentId": agent_id,
            "direction": direction,
            "limit": limit,
            "projection": "projected",
        });
        if let Some(cursor) = cursor {
            request["cursor"] = cursor;
        }
        request
    }
}

const TAIL_LIMIT: u64 = 200;
const PAGE_LIMIT: u64 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// A tail page is on its way; live rows wait in the buffer.
    Loading,
    /// `after` pages are on their way; live rows wait in the buffer.
    CatchingUp,
    /// Live rows apply as they arrive.
    Live,
    /// A fetch failed; a reconnect or a new view retries.
    Failed,
}

/// One live timeline row from `agent_stream`.
#[derive(Clone, Debug)]
pub struct LiveRow {
    pub seq: u64,
    pub epoch: String,
    pub entry: Value,
}

impl LiveRow {
    /// Reads a timeline row from an `agent_stream` payload. Other events give `None`.
    pub fn from_stream(payload: &Value) -> Option<Self> {
        let event = &payload["event"];
        if event["type"] != "timeline" {
            return None;
        }
        let seq = payload["seq"].as_u64()?;
        let epoch = payload["epoch"].as_str()?.to_string();
        let mut entry = json!({
            "provider": event["provider"],
            "item": event["item"],
            "timestamp": payload["timestamp"],
            "seqStart": seq,
            "seqEnd": seq,
            "sourceSeqRanges": [{"startSeq": seq, "endSeq": seq}],
            "collapsed": [],
        });
        if let Some(turn_id) = event.get("turnId").filter(|v| v.is_string()) {
            entry["turnId"] = turn_id.clone();
        }
        Some(Self { seq, epoch, entry })
    }
}

/// The sync state of one viewed agent.
#[derive(Debug)]
pub struct TimelineSync {
    pub state: TimelineState,
    epoch: Option<String>,
    start_seq: Option<u64>,
    end_seq: Option<u64>,
    mode: Mode,
    buffer: Vec<LiveRow>,
    generation: u64,
    older_pending: bool,
}

impl Default for TimelineSync {
    fn default() -> Self {
        Self::new()
    }
}

impl TimelineSync {
    pub fn new() -> Self {
        Self {
            state: TimelineState::default(),
            epoch: None,
            start_seq: None,
            end_seq: None,
            mode: Mode::Loading,
            buffer: Vec::new(),
            generation: 0,
            older_pending: false,
        }
    }

    /// Responses carry the generation of their request; older ones are stale and dropped.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The epoch of the current timeline window; `None` before the first page.
    pub fn epoch(&self) -> Option<&str> {
        self.epoch.as_deref()
    }

    /// The last applied source sequence.
    pub fn end_seq(&self) -> Option<u64> {
        self.end_seq
    }

    /// Starts over with a fresh tail page.
    pub fn plan_tail(&mut self) -> FetchPlan {
        self.generation += 1;
        self.mode = Mode::Loading;
        self.older_pending = false;
        FetchPlan::Tail
    }

    /// After a reconnect: catch up from the end cursor, or load a tail if nothing is synced.
    pub fn resume(&mut self) -> FetchPlan {
        self.buffer.clear();
        match (&self.epoch, self.end_seq) {
            (Some(epoch), Some(seq)) => {
                let epoch = epoch.clone();
                self.generation += 1;
                self.mode = Mode::CatchingUp;
                self.older_pending = false;
                FetchPlan::Resume { epoch, seq }
            }
            _ => self.plan_tail(),
        }
    }

    /// Drops what waits for the old socket; `resume` follows on the next one.
    pub fn disconnected(&mut self) {
        self.buffer.clear();
        self.older_pending = false;
        self.generation += 1;
    }

    /// Takes one live row. Returns a fetch to send when the row shows a gap or a new epoch.
    pub fn on_live(&mut self, row: LiveRow) -> Option<FetchPlan> {
        match self.mode {
            Mode::Live => self.apply_live(row),
            Mode::Loading | Mode::CatchingUp => {
                self.buffer.push(row);
                None
            }
            Mode::Failed => None,
        }
    }

    /// Asks for the page before the first entry.
    pub fn load_older(&mut self) -> Option<FetchPlan> {
        if !self.state.has_older || self.older_pending || self.mode == Mode::Failed {
            return None;
        }
        let epoch = self.epoch.clone()?;
        let seq = self.start_seq?;
        self.older_pending = true;
        Some(FetchPlan::Before { epoch, seq })
    }

    /// Applies a `fetch_agent_timeline_response` payload (or `{"error": ...}`).
    pub fn on_page(
        &mut self,
        plan: &FetchPlan,
        generation: u64,
        page: &Value,
    ) -> Option<FetchPlan> {
        if generation != self.generation {
            return None;
        }
        if let Some(error) = page["error"].as_str() {
            log::warn!("timeline fetch failed: {error}");
            if matches!(plan, FetchPlan::Before { .. }) {
                self.older_pending = false;
                return None;
            }
            // Retrying at once could loop on a daemon-side refusal; a reconnect or a new view
            // retries instead.
            self.mode = Mode::Failed;
            self.buffer.clear();
            return None;
        }
        if matches!(plan, FetchPlan::Before { .. }) {
            self.apply_older(page);
            return None;
        }
        if matches!(plan, FetchPlan::Tail) || page["reset"] == true {
            self.replace(page);
            return self.replay();
        }
        // An `after` page: apply what is newer than the end cursor.
        let Some(epoch) = self.epoch.clone() else {
            return Some(self.plan_tail());
        };
        if page["epoch"].as_str() != Some(epoch.as_str()) {
            return Some(self.plan_tail());
        }
        let before_end = self.end_seq.unwrap_or(0);
        let entries = page["entries"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        for entry in entries {
            apply_page_entry(&mut self.state.entries, entry.clone());
        }
        let page_end = page["endCursor"]["seq"].as_u64();
        if let Some(end) = page_end {
            self.end_seq = Some(before_end.max(end));
        }
        if !entries.is_empty() {
            self.state.version += 1;
        }
        if page["hasNewer"] == true {
            // A resume that misses more than one page loads one latest tail instead.
            let stalled = page_end.is_none_or(|end| end <= before_end);
            if matches!(plan, FetchPlan::Resume { .. }) || stalled {
                return Some(self.plan_tail());
            }
            self.mode = Mode::CatchingUp;
            return Some(FetchPlan::After {
                epoch,
                seq: self.end_seq.unwrap_or(0),
            });
        }
        self.replay()
    }

    fn replace(&mut self, page: &Value) {
        self.epoch = page["epoch"].as_str().map(str::to_string);
        self.state.entries = page["entries"].as_array().cloned().unwrap_or_default();
        self.start_seq = page["startCursor"]["seq"]
            .as_u64()
            .or_else(|| self.state.entries.first().and_then(seq_start));
        self.end_seq = page["endCursor"]["seq"]
            .as_u64()
            .or_else(|| page["window"]["maxSeq"].as_u64());
        self.state.has_older = page["hasOlder"] == true;
        self.state.version += 1;
        // Pages asked for before the replacement describe the old range.
        self.generation += 1;
        self.older_pending = false;
        let epoch = self.epoch.clone();
        self.buffer.retain(|row| Some(&row.epoch) == epoch.as_ref());
    }

    fn apply_older(&mut self, page: &Value) {
        self.older_pending = false;
        if page["epoch"].as_str() != self.epoch.as_deref() {
            return;
        }
        let first = self.state.entries.first().and_then(seq_start);
        let older: Vec<Value> = page["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|entry| match (seq_start(entry), first) {
                (Some(start), Some(first)) => start < first,
                (Some(_), None) => true,
                _ => false,
            })
            .cloned()
            .collect();
        if let Some(start) = page["startCursor"]["seq"].as_u64() {
            self.start_seq = Some(start);
        }
        self.state.has_older = page["hasOlder"] == true;
        self.state.entries.splice(0..0, older);
        self.state.version += 1;
    }

    /// Goes live and applies the buffered rows in order.
    fn replay(&mut self) -> Option<FetchPlan> {
        self.mode = Mode::Live;
        let mut rows = std::mem::take(&mut self.buffer).into_iter();
        while let Some(row) = rows.next() {
            if let Some(plan) = self.apply_live(row) {
                self.buffer.extend(rows);
                return Some(plan);
            }
        }
        None
    }

    fn apply_live(&mut self, row: LiveRow) -> Option<FetchPlan> {
        if self.epoch.as_deref() != Some(row.epoch.as_str()) {
            self.buffer.push(row);
            return Some(self.plan_tail());
        }
        let end = self.end_seq.unwrap_or(0);
        if row.seq <= end {
            return None;
        }
        if row.seq > end + 1 {
            let epoch = row.epoch.clone();
            self.buffer.push(row);
            self.mode = Mode::CatchingUp;
            return Some(FetchPlan::After { epoch, seq: end });
        }
        merge_entry(&mut self.state.entries, row.entry);
        self.end_seq = Some(row.seq);
        self.state.version += 1;
        None
    }
}

fn seq_start(entry: &Value) -> Option<u64> {
    entry["seqStart"].as_u64()
}

/// Puts a fetched entry at its `seqStart`, replacing the entry already there.
fn apply_page_entry(entries: &mut Vec<Value>, entry: Value) {
    let Some(start) = seq_start(&entry) else {
        return;
    };
    match entries.binary_search_by_key(&start, |e| seq_start(e).unwrap_or(0)) {
        Ok(index) => entries[index] = entry,
        Err(index) => entries.insert(index, entry),
    }
}

/// The identity that merges a tool call or plugin item across its lifecycle rows.
fn identity(item: &Value) -> Option<String> {
    match item["type"].as_str()? {
        "tool_call" => item["callId"].as_str().map(str::to_string),
        "plugin" => Some(format!(
            "{}/{}",
            item["pluginId"].as_str()?,
            item["id"].as_str()?
        )),
        _ => None,
    }
}

/// Merges one live row into projected entries, as `TimelineProjection.append` does.
pub fn merge_entry(entries: &mut Vec<Value>, entry: Value) {
    if let Some(key) = identity(&entry["item"])
        && let Some(index) = entries
            .iter()
            .rposition(|e| identity(&e["item"]).as_deref() == Some(key.as_str()))
        && let Some(merged) = merge_identity(&entries[index], &entry)
    {
        entries[index] = merged;
        return;
    }
    if let Some(last) = entries.last_mut()
        && let Some(merged) = merge_adjacent(last, &entry)
    {
        *last = merged;
        return;
    }
    entries.push(entry);
}

fn item_type(entry: &Value) -> &str {
    entry["item"]["type"].as_str().unwrap_or_default()
}

fn append_seq(ranges: &mut Vec<Value>, seq: u64) {
    if let Some(last) = ranges.last_mut() {
        let end = last["endSeq"].as_u64().unwrap_or(0);
        if seq <= end + 1 {
            last["endSeq"] = json!(end.max(seq));
            return;
        }
    }
    ranges.push(json!({"startSeq": seq, "endSeq": seq}));
}

fn merge_ranges(existing: &Value, incoming: &Value) -> Value {
    let mut ranges = existing.as_array().cloned().unwrap_or_default();
    for range in incoming.as_array().into_iter().flatten() {
        let (Some(start), Some(end)) = (range["startSeq"].as_u64(), range["endSeq"].as_u64())
        else {
            continue;
        };
        for seq in start..=end {
            append_seq(&mut ranges, seq);
        }
    }
    Value::Array(ranges)
}

fn with_collapsed(existing: &Value, extra: &Value, kind: &str) -> Value {
    let mut kinds: Vec<Value> = Vec::new();
    for value in existing
        .as_array()
        .into_iter()
        .flatten()
        .chain(extra.as_array().into_iter().flatten())
        .cloned()
        .chain(std::iter::once(json!(kind)))
    {
        if !kinds.contains(&value) {
            kinds.push(value);
        }
    }
    Value::Array(kinds)
}

fn merge_identity(existing: &Value, entry: &Value) -> Option<Value> {
    match item_type(entry) {
        "tool_call" => {
            if item_type(existing) != "tool_call" || existing.get("turnId") != entry.get("turnId") {
                return None;
            }
            let mut merged = existing.clone();
            merged["item"] = merge_tool_call(&existing["item"], &entry["item"]);
            merged["timestamp"] = entry["timestamp"].clone();
            let end = existing["seqEnd"].as_u64().max(entry["seqEnd"].as_u64());
            merged["seqEnd"] = json!(end);
            merged["sourceSeqRanges"] =
                merge_ranges(&existing["sourceSeqRanges"], &entry["sourceSeqRanges"]);
            merged["collapsed"] =
                with_collapsed(&existing["collapsed"], &json!([]), "tool_lifecycle");
            Some(merged)
        }
        "plugin" => {
            if item_type(existing) != "plugin" {
                return None;
            }
            let mut merged = entry.clone();
            merged["seqStart"] = existing["seqStart"].clone();
            merged["sourceSeqRanges"] =
                merge_ranges(&existing["sourceSeqRanges"], &entry["sourceSeqRanges"]);
            merged["collapsed"] = with_collapsed(&existing["collapsed"], &json!([]), "identity");
            Some(merged)
        }
        _ => None,
    }
}

fn merge_tool_call(existing: &Value, incoming: &Value) -> Value {
    let mut merged: Map<String, Value> = existing.as_object().cloned().unwrap_or_default();
    for (key, value) in incoming.as_object().into_iter().flatten() {
        merged.insert(key.clone(), value.clone());
    }
    let unknown = |detail: &Value| detail["type"] == "unknown";
    let (old_detail, new_detail) = (&existing["detail"], &incoming["detail"]);
    let detail = if unknown(new_detail) && !old_detail.is_null() && !unknown(old_detail) {
        old_detail.clone()
    } else {
        new_detail.clone()
    };
    if !detail.is_null() {
        merged.insert("detail".into(), detail);
    }
    let (old_meta, new_meta) = (existing.get("metadata"), incoming.get("metadata"));
    if old_meta.is_some() || new_meta.is_some() {
        let mut metadata = old_meta
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (key, value) in new_meta.and_then(Value::as_object).into_iter().flatten() {
            metadata.insert(key.clone(), value.clone());
        }
        merged.insert("metadata".into(), Value::Object(metadata));
    }
    match incoming["status"].as_str() {
        Some("failed") => match incoming.get("error") {
            Some(error) => {
                merged.insert("error".into(), error.clone());
            }
            None => {
                merged.remove("error");
            }
        },
        Some("completed") | Some("canceled") => {
            merged.insert("error".into(), Value::Null);
        }
        _ => {}
    }
    Value::Object(merged)
}

fn merge_adjacent(previous: &Value, entry: &Value) -> Option<Value> {
    let kind = item_type(entry);
    if kind != item_type(previous) || !matches!(kind, "assistant_message" | "reasoning") {
        return None;
    }
    let previous_end = previous["seqEnd"].as_u64()?;
    if entry["seqStart"].as_u64()? != previous_end + 1
        || previous.get("turnId") != entry.get("turnId")
    {
        return None;
    }
    let text = format!(
        "{}{}",
        previous["item"]["text"].as_str().unwrap_or_default(),
        entry["item"]["text"].as_str().unwrap_or_default()
    );
    let (item, collapse) = if kind == "assistant_message" {
        let message_id = previous["item"]["messageId"]
            .as_str()
            .filter(|id| !id.is_empty());
        if let Some(incoming) = entry["item"].get("messageId")
            && incoming.as_str() != previous["item"]["messageId"].as_str()
        {
            return None;
        }
        let mut item = json!({"type": "assistant_message", "text": text});
        if let Some(id) = message_id {
            item["messageId"] = json!(id);
        }
        (item, "assistant_merge")
    } else {
        (
            json!({"type": "reasoning", "text": text}),
            "reasoning_merge",
        )
    };
    let mut merged = previous.clone();
    merged["item"] = item;
    merged["timestamp"] = entry["timestamp"].clone();
    merged["seqEnd"] = entry["seqEnd"].clone();
    merged["sourceSeqRanges"] =
        merge_ranges(&previous["sourceSeqRanges"], &entry["sourceSeqRanges"]);
    merged["collapsed"] = with_collapsed(&previous["collapsed"], &entry["collapsed"], collapse);
    Some(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(seq: u64, item: Value, turn: &str) -> Value {
        json!({
            "provider": "mock",
            "item": item,
            "timestamp": format!("t{seq}"),
            "turnId": turn,
            "seqStart": seq,
            "seqEnd": seq,
            "sourceSeqRanges": [{"startSeq": seq, "endSeq": seq}],
            "collapsed": [],
        })
    }

    #[test]
    fn assistant_chunks_merge_like_the_daemon() {
        let mut entries = Vec::new();
        let chunk =
            |text: &str| json!({"type": "assistant_message", "text": text, "messageId": "m1"});
        merge_entry(
            &mut entries,
            row(1, json!({"type": "user_message", "text": "hi"}), "t"),
        );
        merge_entry(&mut entries, row(2, chunk("Hel"), "t"));
        merge_entry(&mut entries, row(3, chunk("lo"), "t"));
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[1]["item"],
            json!({"type": "assistant_message", "text": "Hello", "messageId": "m1"})
        );
        assert_eq!(entries[1]["seqStart"], 2);
        assert_eq!(entries[1]["seqEnd"], 3);
        assert_eq!(
            entries[1]["sourceSeqRanges"],
            json!([{"startSeq": 2, "endSeq": 3}])
        );
        assert_eq!(entries[1]["collapsed"], json!(["assistant_merge"]));
        assert_eq!(entries[1]["timestamp"], "t3");
        // A new message id starts a new entry, and so does a new turn.
        merge_entry(
            &mut entries,
            row(
                4,
                json!({"type": "assistant_message", "text": "x", "messageId": "m2"}),
                "t",
            ),
        );
        merge_entry(
            &mut entries,
            row(5, json!({"type": "reasoning", "text": "a"}), "t"),
        );
        merge_entry(
            &mut entries,
            row(6, json!({"type": "reasoning", "text": "b"}), "t"),
        );
        merge_entry(
            &mut entries,
            row(7, json!({"type": "reasoning", "text": "c"}), "u"),
        );
        assert_eq!(entries.len(), 5);
        assert_eq!(
            entries[3]["item"],
            json!({"type": "reasoning", "text": "ab"})
        );
        assert_eq!(entries[3]["collapsed"], json!(["reasoning_merge"]));
    }

    #[test]
    fn tool_call_lifecycle_merges_in_place() {
        let mut entries = Vec::new();
        let call = |status: &str, detail: Value| {
            json!({"type": "tool_call", "callId": "c1", "name": "read", "status": status,
                   "detail": detail, "error": null})
        };
        merge_entry(
            &mut entries,
            row(
                1,
                call("running", json!({"type": "read", "filePath": "a"})),
                "t",
            ),
        );
        merge_entry(
            &mut entries,
            row(2, json!({"type": "reasoning", "text": "r"}), "t"),
        );
        merge_entry(
            &mut entries,
            row(3, call("completed", json!({"type": "unknown"})), "t"),
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["item"]["status"], "completed");
        assert_eq!(entries[0]["item"]["detail"]["filePath"], "a");
        assert_eq!(entries[0]["seqStart"], 1);
        assert_eq!(entries[0]["seqEnd"], 3);
        assert_eq!(
            entries[0]["sourceSeqRanges"],
            json!([{"startSeq": 1, "endSeq": 1}, {"startSeq": 3, "endSeq": 3}])
        );
        assert_eq!(entries[0]["collapsed"], json!(["tool_lifecycle"]));
        // A failed status keeps the incoming error.
        merge_entry(
            &mut entries,
            row(
                4,
                json!({"type": "tool_call", "callId": "c1", "status": "failed", "error": {"message": "x"}}),
                "t",
            ),
        );
        assert_eq!(entries[0]["item"]["error"], json!({"message": "x"}));
    }

    #[test]
    fn page_entries_replace_by_seq_start() {
        let mut entries = vec![row(1, json!({"type": "user_message", "text": "a"}), "t")];
        let mut partial = row(2, json!({"type": "assistant_message", "text": "He"}), "t");
        partial["seqEnd"] = json!(3);
        entries.push(partial);
        let mut full = row(
            2,
            json!({"type": "assistant_message", "text": "Hello"}),
            "t",
        );
        full["seqEnd"] = json!(5);
        apply_page_entry(&mut entries, full.clone());
        apply_page_entry(
            &mut entries,
            row(6, json!({"type": "user_message", "text": "b"}), "t"),
        );
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1], full);
    }
}
