//! Host-owned, source-neutral guidance policy helpers.
//!
//! Providers report observations. Hosts decide whether and how those
//! observations influence an already Bliss-qualified candidate pool.

use serde::{Deserialize, Serialize};

/// The two host policies shared by current and future Bliss-first hosts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostPolicyKind {
    /// Multiply one candidate's score by a bounded function of its signal.
    BoundedInfluence,
    /// Calibrate supported candidates to a desired share inside a fixed pool.
    TargetShare,
}

/// A host's policy for one provider-local guidance channel.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuidancePolicyEntry {
    pub provider_id: String,
    pub channel: String,
    /// Signed bounded influence in [-1, 1]. Values outside that interval are
    /// normalized at application time so malformed host input stays bounded.
    pub weight: f64,
    /// A desired percentage of supported candidates in the final selection.
    /// This is host policy, never a provider scoring instruction.
    #[serde(default)]
    pub target_percent: Option<u8>,
}

impl GuidancePolicyEntry {
    pub fn key(&self) -> (&str, &str) {
        (&self.provider_id, &self.channel)
    }

    pub fn bounded_weight(&self) -> f64 {
        self.weight.clamp(-1.0, 1.0)
    }

    pub fn policy_kind(&self) -> HostPolicyKind {
        if self.target_percent.unwrap_or(0) > 0 {
            HostPolicyKind::TargetShare
        } else {
            HostPolicyKind::BoundedInfluence
        }
    }
}

/// A deterministic explanation of one host-policy contribution.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AppliedGuidanceContribution {
    pub provider_id: String,
    pub channel: String,
    pub signal_score: f64,
    pub confidence: f64,
    pub policy_weight: f64,
    pub contribution: f64,
    pub multiplier: f64,
}

impl AppliedGuidanceContribution {
    /// Applies bounded entries in deterministic provider/channel order.
    pub fn from_policy_entries(
        entries: &[GuidancePolicyEntry],
        signal_score: f64,
        confidence: f64,
    ) -> Vec<Self> {
        let signal_score = signal_score.clamp(-1.0, 1.0);
        let confidence = confidence.clamp(0.0, 1.0);
        let mut entries = entries.to_vec();
        entries.sort_by(|left, right| left.key().cmp(&right.key()));
        entries
            .into_iter()
            .filter(|entry| entry.policy_kind() == HostPolicyKind::BoundedInfluence)
            .map(|entry| {
                let policy_weight = entry.bounded_weight();
                let contribution = policy_weight * signal_score * confidence;
                Self {
                    provider_id: entry.provider_id,
                    channel: entry.channel,
                    signal_score,
                    confidence,
                    policy_weight,
                    contribution,
                    multiplier: bounded_multiplier(policy_weight, signal_score * confidence),
                }
            })
            .collect()
    }
}

/// Lab-compatible multiplicative bounded influence.
pub fn bounded_multiplier(weight: f64, signal: f64) -> f64 {
    (std::f64::consts::LN_10 * weight.clamp(-1.0, 1.0) * signal.clamp(-1.0, 1.0)).exp()
}

/// Calculates the DSTM-style target-share multiplier for already
/// Bliss-qualified supported candidates.
pub fn target_share_multiplier(
    target_percent: u8,
    supported_base_weight: f64,
    other_base_weight: f64,
) -> f64 {
    if target_percent == 0 || supported_base_weight <= 0.0 || other_base_weight <= 0.0 {
        return 1.0;
    }
    let target = f64::from(target_percent) / 100.0;
    if target >= 1.0 {
        return 1_000_000.0;
    }
    ((target * other_base_weight) / ((1.0 - target) * supported_base_weight)).max(0.000_001)
}

/// Converts a frozen timestamp to BlissMixerLab's time signal.
///
/// A zero last-played value means never played and becomes `-1`. A zero or
/// negative library-added value is unknown and returns `None`. Future values
/// are treated as newest (`1`) rather than gaining extra influence.
pub fn saturating_time_signal(
    timestamp: Option<i64>,
    as_of_unix_seconds: i64,
    horizon_seconds: i64,
    zero_means_never: bool,
) -> Option<f64> {
    let timestamp = timestamp?;
    if timestamp <= 0 {
        return zero_means_never.then_some(-1.0);
    }
    let horizon_seconds = horizon_seconds.max(1);
    let age_seconds = as_of_unix_seconds.saturating_sub(timestamp).max(0) as f64;
    let remaining = (-age_seconds / horizon_seconds as f64).exp();
    Some((2.0 * remaining) - 1.0)
}
