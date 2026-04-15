use crate::contracts::{CapabilityManifest, TransportKind};

pub trait ManifestScoringPolicy {
    fn score_manifest(&self, manifest: &CapabilityManifest) -> i32;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultManifestScoringPolicy;

impl ManifestScoringPolicy for DefaultManifestScoringPolicy {
    fn score_manifest(&self, manifest: &CapabilityManifest) -> i32 {
        let transport_score = match manifest.transport {
            TransportKind::Local => 50,
            TransportKind::Cli => 40,
            TransportKind::Mcp => 30,
            TransportKind::Bridge => 20,
            TransportKind::Api => 10,
        };

        let capability_score = [
            manifest.supports.chat,
            manifest.supports.streaming,
            manifest.supports.screen_reasoning,
            manifest.supports.tool_use,
        ]
        .into_iter()
        .filter(|flag| *flag)
        .count() as i32
            * 5;

        transport_score + capability_score
    }
}

