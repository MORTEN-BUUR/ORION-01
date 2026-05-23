//! Technical documentation per EU AI Act Annex IV (Article 11).

use serde::{Deserialize, Serialize};

/// Annex IV technical documentation structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalDocumentation {
    pub general_description: GeneralDescription,
    pub development_process: DevelopmentProcess,
    pub risk_management: RiskManagementSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralDescription {
    pub intended_purpose: String,
    pub provider_name: String,
    pub system_version: String,
    pub hardware_requirements: Option<String>,
    pub deployment_forms: Vec<String>,
    pub usage_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentProcess {
    pub design_specifications: String,
    pub system_architecture: String,
    pub data_requirements: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskManagementSummary {
    pub residual_risks: Vec<String>,
    pub mitigation_measures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnexIVSection {
    GeneralDescription,
    DevelopmentProcess,
    RiskManagement,
    HumanOversight,
    AccuracyRobustness,
}

impl TechnicalDocumentation {
    pub fn required_sections_for_high_risk() -> &'static [AnnexIVSection] {
        &[
            AnnexIVSection::GeneralDescription,
            AnnexIVSection::DevelopmentProcess,
            AnnexIVSection::RiskManagement,
            AnnexIVSection::HumanOversight,
            AnnexIVSection::AccuracyRobustness,
        ]
    }

    pub fn validate_completeness(&self) -> Vec<AnnexIVSection> {
        let mut missing = Vec::new();
        if self.general_description.intended_purpose.is_empty() {
            missing.push(AnnexIVSection::GeneralDescription);
        }
        if self.development_process.design_specifications.is_empty() {
            missing.push(AnnexIVSection::DevelopmentProcess);
        }
        if self.risk_management.residual_risks.is_empty() && self.risk_management.mitigation_measures.is_empty() {
            missing.push(AnnexIVSection::RiskManagement);
        }
        missing
    }
}
