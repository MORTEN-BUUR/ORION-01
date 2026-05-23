//! Fundamental rights impact assessment per EU AI Act Article 29.

use serde::{Deserialize, Serialize};

/// Fundamental rights that may be affected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffectedRight {
    NonDiscrimination,
    Privacy,
    DataProtection,
    FreedomOfExpression,
    RightToEducation,
    RightToWork,
    AccessToServices,
    FairTrial,
    Other(String),
}

/// Severity of impact on a fundamental right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactSeverity {
    Negligible,
    Limited,
    Significant,
    Serious,
}

/// Single fundamental rights impact entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundamentalRightsImpact {
    pub right: AffectedRight,
    pub severity: ImpactSeverity,
    pub description: String,
    pub mitigation: Option<String>,
}

/// Full impact assessment (Article 29).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub system_id: String,
    pub version: String,
    pub impacts: Vec<FundamentalRightsImpact>,
    pub overall_risk: ImpactSeverity,
    pub mitigation_summary: Option<String>,
}

impl ImpactAssessment {
    pub fn has_serious_impact(&self) -> bool {
        self.impacts
            .iter()
            .any(|i| i.severity == ImpactSeverity::Serious)
    }

    pub fn required_for_high_risk(&self) -> bool {
        self.overall_risk == ImpactSeverity::Significant
            || self.overall_risk == ImpactSeverity::Serious
    }
}
