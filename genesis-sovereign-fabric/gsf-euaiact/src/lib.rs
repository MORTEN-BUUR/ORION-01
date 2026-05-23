//! EU AI Act compliance extensions for GENESIS Sovereign Fabric.
//! Based on Regulation (EU) 2024/1689 and implementing acts.

pub mod risk_classification;
pub mod human_oversight;
pub mod technical_docs;
pub mod transparency;
pub mod fundamental_rights;

pub use risk_classification::{RiskLevel, AnnexIIICategory, classify_risk};
pub use human_oversight::{OversightType, PendingDecision, OversightState};
pub use technical_docs::{TechnicalDocumentation, AnnexIVSection};
pub use transparency::{TransparencyConfig, EndUserDisclosure};
pub use fundamental_rights::{ImpactAssessment, FundamentalRightsImpact};
