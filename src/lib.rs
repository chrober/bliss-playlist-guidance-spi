//! Provider-neutral SPI shared by `bliss-playlist-optimizer` and guidance addons.
//!
//! The wire format is newline-delimited JSON (JSONL).  An optimizer starts an
//! addon, sends a `describe` request, then a job-scoped `prepare` request and
//! zero or more batched `score` requests.  Addons never decide hard
//! eligibility: they only return bounded guidance signals for candidates that
//! the optimizer has already admitted to a scoring batch.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SPI_VERSION: u16 = 1;
pub const PROTOCOL_NAME: &str = "bliss-playlist-optimizer-guidance-jsonl";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GuidanceRequest {
    Describe {
        spi_version: u16,
    },
    Prepare {
        spi_version: u16,
        job_id: String,
        options: Value,
        candidates: Vec<Candidate>,
        anchors: Vec<Anchor>,
    },
    Score {
        spi_version: u16,
        request_id: String,
        context: ScoreContext,
        candidates: Vec<Candidate>,
    },
    Close {
        spi_version: u16,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GuidanceResponse {
    Manifest(Manifest),
    Prepared {
        provider_id: String,
        snapshot_id: Option<String>,
        diagnostics: Diagnostics,
    },
    Scores {
        provider_id: String,
        request_id: String,
        signals: Vec<GuidanceSignal>,
        diagnostics: Diagnostics,
    },
    Closed {
        provider_id: String,
    },
    Error {
        provider_id: Option<String>,
        code: String,
        message: String,
        retryable: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub spi_version: u16,
    pub provider_id: String,
    pub provider_version: String,
    pub protocol: String,
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub required_context: Vec<String>,
    #[serde(default)]
    pub configuration_schema: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    GlobalCandidateGuidance,
    EdgeCandidateGuidance,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub candidate_id: String,
    #[serde(default)]
    pub database_file: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub recording_mbid: Option<String>,
    #[serde(default)]
    pub artist_mbids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Anchor {
    pub anchor_id: String,
    #[serde(flatten)]
    pub track: Candidate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScoreContext {
    pub scope: GuidanceScope,
    #[serde(default)]
    pub left_anchor_id: Option<String>,
    #[serde(default)]
    pub right_anchor_id: Option<String>,
    #[serde(default)]
    pub context_track_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceScope {
    Global,
    Edge,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuidanceSignal {
    pub candidate_id: String,
    pub scope: GuidanceScope,
    /// Normalized contribution in [-1, 1]. Positive values support a
    /// candidate, negative values penalize it, and zero is neutral.
    pub score: f64,
    /// Confidence in the provider's guidance, also normalized to [0, 1].
    pub confidence: f64,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub observed_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Diagnostics {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub request_count: u64,
    #[serde(default)]
    pub failure_count: u64,
    #[serde(default)]
    pub details: Option<Value>,
}

impl GuidanceSignal {
    pub fn bounded(mut self) -> Self {
        self.score = self.score.clamp(-1.0, 1.0);
        self.confidence = self.confidence.clamp(0.0, 1.0);
        self
    }
}

pub fn encode<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

pub fn decode_request(line: &str) -> Result<GuidanceRequest, serde_json::Error> {
    serde_json::from_str(line)
}

pub fn decode_response(line: &str) -> Result<GuidanceResponse, serde_json::Error> {
    serde_json::from_str(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_request_round_trips_as_jsonl() {
        let request = GuidanceRequest::Score {
            spi_version: SPI_VERSION,
            request_id: "batch-1".into(),
            context: ScoreContext {
                scope: GuidanceScope::Edge,
                left_anchor_id: Some("source-a".into()),
                right_anchor_id: Some("source-b".into()),
                context_track_ids: vec!["source-a".into()],
            },
            candidates: vec![Candidate {
                candidate_id: "bliss-row-42".into(),
                database_file: Some("/music/a.mp3".into()),
                title: Some("A song".into()),
                artist: Some("An artist".into()),
                album: None,
                recording_mbid: None,
                artist_mbids: vec![],
            }],
        };
        let encoded = encode(&request).unwrap();
        assert!(!encoded.contains('\n'));
        assert_eq!(decode_request(&encoded).unwrap(), request);
    }

    #[test]
    fn signals_are_bounded() {
        let signal = GuidanceSignal {
            candidate_id: "c".into(),
            scope: GuidanceScope::Global,
            score: 4.0,
            confidence: -2.0,
            rationale: None,
            observed_at: None,
        }
        .bounded();
        assert_eq!(signal.score, 1.0);
        assert_eq!(signal.confidence, 0.0);
    }
}
