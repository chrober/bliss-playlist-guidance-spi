//! Provider-neutral SPI shared by `bliss-playlist-optimizer` and guidance addons.
//!
//! The wire format is newline-delimited JSON (JSONL). A host starts an addon,
//! sends a `describe` request, then a job-scoped `prepare` request and
//! zero or more batched `score` requests.  Addons never decide hard
//! eligibility: they only return bounded guidance signals for candidates that
//! the host has already admitted to a scoring batch.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod policy;

pub const SPI_VERSION: u16 = 2;
pub const PROTOCOL_NAME: &str = "bliss-guidance-jsonl-v2";

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
        artifacts: Vec<ArtifactDescriptor>,
        resources: Vec<ResourceDescriptor>,
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
    pub channels: Vec<ChannelDescriptor>,
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

/// A provider-local, stable signal channel and the guidance scopes in which
/// the provider can emit it. Hosts identify a policy by the pair of provider
/// ID and this channel name, never by a globally reserved channel string.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelDescriptor {
    pub channel: String,
    pub scopes: Vec<GuidanceScope>,
    /// Host policies this provider channel can safely support. An empty list
    /// means that the provider made no capability claim.
    #[serde(default)]
    pub supported_host_policies: Vec<policy::HostPolicyKind>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub candidate_id: String,
    #[serde(default)]
    pub lms_urlmd5: Option<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactDescriptor {
    pub kind: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceDescriptor {
    pub kind: String,
    pub path: String,
    pub access: ResourceAccess,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAccess {
    ReadOnly,
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
    /// Stable provider-defined channel, for example `lastfm_track`,
    /// `lastfm_artist`, or `playcount`.
    pub channel: String,
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
    /// Optional structured raw observation.  Providers may expose bounded
    /// evidence such as a play count or timestamp so a host can retain an
    /// existing presentation without rereading the provider's data source.
    #[serde(default)]
    pub observation: Option<Value>,
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

/// Host-to-host description of one bounded native guidance request.
///
/// This envelope is deliberately separate from the provider JSONL messages.
/// A Lyrion host constructs it from trusted provider configuration and sends
/// it to a native Bliss host; the native host then owns the provider process
/// lifecycle.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuidanceHostRequestV1 {
    pub request_version: String,
    pub job_id: String,
    pub request_id: String,
    pub deadline_ms: u64,
    pub provider: GuidanceProviderConfig,
    /// Host-owned policy used solely to explain signed provider contributions.
    #[serde(default)]
    pub policy: Value,
    pub context: ScoreContext,
    pub candidates: Vec<Candidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuidanceProviderConfig {
    pub provider_id: String,
    pub program: String,
    #[serde(default)]
    pub argv: Vec<String>,
    #[serde(default)]
    pub options: Value,
    #[serde(default)]
    pub artifacts: Vec<ArtifactDescriptor>,
    #[serde(default)]
    pub resources: Vec<ResourceDescriptor>,
}

/// Structured data behind one candidate's guidance explanation. It never
/// contains rendered LMS log text; the Perl host retains that responsibility.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectionTraceV1 {
    pub trace_version: String,
    pub provider_id: String,
    pub host: String,
    #[serde(default)]
    pub policy: Value,
    pub candidates: Vec<SelectionTraceCandidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectionTraceCandidate {
    pub candidate_id: String,
    #[serde(default)]
    pub bliss_similarity: Option<f64>,
    #[serde(default)]
    pub guidance: Vec<SelectionTraceContribution>,
    /// Reserved for a future native final-selection host. This first slice
    /// leaves it absent so Lab can retain its established selection formatter.
    #[serde(default)]
    pub final_score: Option<f64>,
    #[serde(default)]
    pub dominant_boost: Option<String>,
    #[serde(default)]
    pub stochastic_key: Option<f64>,
    #[serde(default)]
    pub cutoff: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectionTraceContribution {
    pub channel: String,
    pub score: f64,
    pub confidence: f64,
    pub contribution: f64,
    #[serde(default)]
    pub observation: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuidanceHostResponseV1 {
    pub response_version: String,
    pub valid: bool,
    #[serde(default)]
    pub signals: Vec<GuidanceSignal>,
    #[serde(default)]
    pub diagnostics: Diagnostics,
    #[serde(default)]
    pub selection_trace: Option<SelectionTraceV1>,
    #[serde(default)]
    pub diagnostic: String,
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
    use crate::policy::{
        bounded_multiplier, saturating_time_signal, target_share_multiplier,
        AppliedGuidanceContribution, GuidancePolicyEntry, HostPolicyKind,
    };

    #[test]
    fn shared_policy_clamps_orders_and_reports_bounded_contributions() {
        let entries = vec![
            GuidancePolicyEntry {
                provider_id: "lastfm-guidance".into(),
                channel: "lastfm_artist".into(),
                weight: 4.0,
                target_percent: None,
            },
            GuidancePolicyEntry {
                provider_id: "library-signals".into(),
                channel: "playcount".into(),
                weight: -4.0,
                target_percent: None,
            },
        ];

        let contributions = AppliedGuidanceContribution::from_policy_entries(&entries, 0.5, 0.8);
        assert_eq!(contributions.len(), 2);
        assert_eq!(contributions[0].provider_id, "lastfm-guidance");
        assert_eq!(contributions[1].provider_id, "library-signals");
        assert!((contributions[0].multiplier - bounded_multiplier(1.0, 0.4)).abs() < 1e-12);
        assert!((contributions[1].multiplier - bounded_multiplier(-1.0, 0.4)).abs() < 1e-12);
    }

    #[test]
    fn shared_policy_uses_lab_exponential_time_signal() {
        let horizon = 10 * 86_400;
        assert_eq!(
            saturating_time_signal(None, 1_000_000, horizon, false),
            None
        );
        assert_eq!(
            saturating_time_signal(Some(0), 1_000_000, horizon, true),
            Some(-1.0)
        );
        assert_eq!(
            saturating_time_signal(Some(1_000_001), 1_000_000, horizon, false),
            Some(1.0)
        );

        let at_horizon =
            saturating_time_signal(Some(1_000_000 - horizon), 1_000_000, horizon, false).unwrap();
        assert!((at_horizon - ((2.0 * (-1.0_f64).exp()) - 1.0)).abs() < 1e-12);
    }

    #[test]
    fn shared_policy_calculates_sparse_target_share_multiplier() {
        let multiplier = target_share_multiplier(75, 0.2, 1.8);
        assert!(multiplier > 1.0);
        assert_eq!(target_share_multiplier(0, 0.2, 1.8), 1.0);
        assert_eq!(target_share_multiplier(75, 0.0, 1.8), 1.0);
    }

    #[test]
    fn manifest_declares_host_policy_capability_per_channel() {
        let channel = ChannelDescriptor {
            channel: "lastfm_artist".into(),
            scopes: vec![GuidanceScope::Global, GuidanceScope::Edge],
            supported_host_policies: vec![
                HostPolicyKind::BoundedInfluence,
                HostPolicyKind::TargetShare,
            ],
        };

        let decoded: ChannelDescriptor = serde_json::from_str(&encode(&channel).unwrap()).unwrap();
        assert_eq!(
            decoded.supported_host_policies,
            channel.supported_host_policies
        );
    }

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
                lms_urlmd5: None,
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
            channel: "playcount".into(),
            scope: GuidanceScope::Global,
            score: 4.0,
            confidence: -2.0,
            rationale: None,
            observed_at: None,
            observation: None,
        }
        .bounded();
        assert_eq!(signal.score, 1.0);
        assert_eq!(signal.confidence, 0.0);
    }

    #[test]
    fn guidance_signal_round_trips_a_stable_channel() {
        let signal = GuidanceSignal {
            candidate_id: "bliss-row-42".into(),
            channel: "lastfm_track".into(),
            scope: GuidanceScope::Edge,
            score: 0.75,
            confidence: 0.9,
            rationale: Some("similar recording".into()),
            observed_at: None,
            observation: Some(serde_json::json!({"recording_mbid": "fixture"})),
        };

        let decoded: GuidanceSignal = serde_json::from_str(&encode(&signal).unwrap()).unwrap();
        assert_eq!(decoded.channel, "lastfm_track");
        assert_eq!(decoded.observation, signal.observation);
    }

    #[test]
    fn prepare_round_trips_artifacts_and_resources_without_candidate_inventory() {
        let request = GuidanceRequest::Prepare {
            spi_version: 2,
            job_id: "preview-42".into(),
            options: serde_json::json!({"preference_percent": -40}),
            artifacts: vec![ArtifactDescriptor {
                kind: "resolved-lastfm-evidence-v1".into(),
                path: "/private/job/semantic-evidence.json".into(),
                sha256: "a".repeat(64),
            }],
            resources: vec![ResourceDescriptor {
                kind: "lms-persist-sqlite-v1".into(),
                path: "/private/lms/persist.db".into(),
                access: ResourceAccess::ReadOnly,
            }],
            anchors: vec![],
        };

        let encoded = encode(&request).unwrap();
        assert!(!encoded.contains("candidates"));
        assert_eq!(decode_request(&encoded).unwrap(), request);
    }

    #[test]
    fn score_candidate_preserves_lms_urlmd5() {
        let request = GuidanceRequest::Score {
            spi_version: 2,
            request_id: "batch-1".into(),
            context: ScoreContext {
                scope: GuidanceScope::Global,
                left_anchor_id: None,
                right_anchor_id: None,
                context_track_ids: vec![],
            },
            candidates: vec![Candidate {
                candidate_id: "bliss-row-42".into(),
                lms_urlmd5: Some("aabbcc".into()),
                database_file: None,
                title: None,
                artist: None,
                album: None,
                recording_mbid: None,
                artist_mbids: vec![],
            }],
        };

        let decoded = decode_request(&encode(&request).unwrap()).unwrap();
        let GuidanceRequest::Score { candidates, .. } = decoded else {
            panic!("expected a score request");
        };
        assert_eq!(candidates[0].lms_urlmd5.as_deref(), Some("aabbcc"));
    }

    #[test]
    fn v2_schema_declares_candidate_free_prepare() {
        let schema: Value =
            serde_json::from_str(include_str!("../schemas/guidance-addon-spi-v2.schema.json"))
                .unwrap();
        let prepare = &schema["$defs"]["prepare"];
        assert!(prepare["required"]
            .as_array()
            .unwrap()
            .contains(&Value::String("artifacts".into())));
        assert!(prepare["required"]
            .as_array()
            .unwrap()
            .contains(&Value::String("resources".into())));
        assert!(prepare["properties"].get("candidates").is_none());
    }

    #[test]
    fn v2_protocol_name_is_host_neutral() {
        assert_eq!(PROTOCOL_NAME, "bliss-guidance-jsonl-v2");
        let schema: Value =
            serde_json::from_str(include_str!("../schemas/guidance-addon-spi-v2.schema.json"))
                .unwrap();
        assert_eq!(
            schema["$defs"]["manifest"]["properties"]["protocol"]["const"],
            Value::String(PROTOCOL_NAME.into()),
        );
    }

    #[test]
    fn manifest_round_trips_provider_local_channel_scopes() {
        let manifest = Manifest {
            spi_version: SPI_VERSION,
            provider_id: "fixture-guidance".into(),
            provider_version: "1.0.0".into(),
            protocol: PROTOCOL_NAME.into(),
            capabilities: vec![Capability::GlobalCandidateGuidance],
            channels: vec![ChannelDescriptor {
                channel: "preference".into(),
                scopes: vec![GuidanceScope::Global],
                supported_host_policies: vec![policy::HostPolicyKind::BoundedInfluence],
            }],
            required_context: vec![],
            configuration_schema: None,
        };

        let decoded: Manifest = serde_json::from_str(&encode(&manifest).unwrap()).unwrap();
        assert_eq!(decoded.channels, manifest.channels);
    }

    #[test]
    fn library_signals_selection_trace_fixture_round_trips_without_rendered_log_text() {
        let fixture: SelectionTraceV1 = serde_json::from_str(include_str!(
            "../fixtures/selection-trace-v1-library-signals.json"
        ))
        .expect("Library Signals selection trace fixture must be valid");

        assert_eq!(fixture.trace_version, "selection_trace_v1");
        assert_eq!(fixture.provider_id, "library-signals-guidance");
        assert_eq!(fixture.candidates.len(), 1);
        assert_eq!(fixture.candidates[0].candidate_id, "file:///music/example.flac");
        assert_eq!(fixture.candidates[0].guidance.len(), 3);
        assert!(!encode(&fixture)
            .expect("trace encodes")
            .contains("Candidate selection:"));
    }

    #[test]
    fn library_signals_host_request_fixture_round_trips() {
        let fixture: GuidanceHostRequestV1 = serde_json::from_str(include_str!(
            "../fixtures/guidance-host-request-v1-library-signals.json"
        ))
        .expect("Library Signals host request fixture must be valid");
        assert_eq!(fixture.request_version, "guidance_host_request_v1");
        assert_eq!(fixture.provider.provider_id, "library-signals-guidance");
        assert_eq!(fixture.candidates[0].lms_urlmd5.as_deref(), Some("aabbcc"));
    }
}
