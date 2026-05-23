//! EU AI Act risk classification (Article 6, Annex III).
//! Risk tiers: Unacceptable (prohibited), High, Limited, Minimal.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Risk level per EU AI Act Article 5-6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Article 5: Prohibited AI practices. Must not be deployed.
    Unacceptable,

    /// Annex III: High-risk systems. Strict compliance required by Aug 2026/2027.
    High,

    /// Limited transparency obligations (e.g. chatbots, deepfakes).
    Limited,

    /// Minimal or no risk. No specific obligations.
    Minimal,
}

impl RiskLevel {
    /// Whether human oversight is mandatory.
    pub fn requires_human_oversight(&self) -> bool {
        matches!(self, RiskLevel::High)
    }

    /// Whether technical documentation (Annex IV) is required.
    pub fn requires_technical_docs(&self) -> bool {
        matches!(self, RiskLevel::High)
    }

    /// Whether fundamental rights impact assessment is required.
    pub fn requires_fundamental_rights_assessment(&self) -> bool {
        matches!(self, RiskLevel::High)
    }

    /// Numeric severity for ordering (higher = more restrictive).
    pub fn severity(&self) -> u8 {
        match self {
            RiskLevel::Unacceptable => 4,
            RiskLevel::High => 3,
            RiskLevel::Limited => 2,
            RiskLevel::Minimal => 1,
        }
    }
}

/// Annex III high-risk categories (simplified).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnexIIICategory {
    BiometricIdentification,
    BiometricCategorization,
    EmotionRecognition,
    CriticalInfrastructure,
    Education,
    Employment,
    AccessToServices,
    LawEnforcement,
    Migration,
}

/// Classifier that maps use case attributes to risk level.
#[derive(Debug, Clone, Default)]
pub struct RiskClassifier {
    high_risk_categories: HashSet<AnnexIIICategory>,
}

impl RiskClassifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_high_risk(categories: impl IntoIterator<Item = AnnexIIICategory>) -> Self {
        Self {
            high_risk_categories: categories.into_iter().collect(),
        }
    }

    /// Classify risk from declared category and optional flags.
    pub fn classify(
        &self,
        category: Option<AnnexIIICategory>,
        prohibited_flags: &[String],
    ) -> RiskLevel {
        if prohibited_flags
            .iter()
            .any(|f| f.eq_ignore_ascii_case("unacceptable") || f.eq_ignore_ascii_case("prohibited"))
        {
            return RiskLevel::Unacceptable;
        }
        if let Some(cat) = category {
            if self.high_risk_categories.contains(&cat) {
                return RiskLevel::High;
            }
        }
        if prohibited_flags
            .iter()
            .any(|f| f.eq_ignore_ascii_case("limited_transparency"))
        {
            return RiskLevel::Limited;
        }
        RiskLevel::Minimal
    }
}

/// Convenience: classify from governance flags.
pub fn classify_risk(
    governance_flags: &[String],
    category: Option<AnnexIIICategory>,
) -> RiskLevel {
    let classifier = RiskClassifier::with_high_risk([
        AnnexIIICategory::BiometricIdentification,
        AnnexIIICategory::BiometricCategorization,
        AnnexIIICategory::EmotionRecognition,
        AnnexIIICategory::CriticalInfrastructure,
        AnnexIIICategory::Education,
        AnnexIIICategory::Employment,
        AnnexIIICategory::AccessToServices,
        AnnexIIICategory::LawEnforcement,
        AnnexIIICategory::Migration,
    ]);
    classifier.classify(category, governance_flags)
}

#[derive(Debug, Error)]
pub enum ClassificationError {
    #[error("deployment prohibited for risk level: {0:?}")]
    Prohibited(RiskLevel),
}
