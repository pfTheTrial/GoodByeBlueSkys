use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::contracts::{CapabilityManifest, RuntimeMode, RuntimePolicy, TransportKind};
use crate::policy::{DefaultManifestScoringPolicy, ManifestScoringPolicy};

#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    manifests_by_id: HashMap<String, CapabilityManifest>,
    scoring_policy: DefaultManifestScoringPolicy,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, manifest: CapabilityManifest) -> Result<(), RegistryError> {
        if manifest.id.trim().is_empty() {
            return Err(RegistryError::InvalidId);
        }

        if self.manifests_by_id.contains_key(&manifest.id) {
            return Err(RegistryError::AlreadyRegistered(manifest.id));
        }

        self.manifests_by_id.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    pub fn list(&self) -> Vec<CapabilityManifest> {
        self.manifests_by_id.values().cloned().collect()
    }

    pub fn choose_backend(
        &self,
        policy: RuntimePolicy,
        require_screen_reasoning: bool,
    ) -> Result<CapabilityManifest, RegistryError> {
        self.choose_backend_with_filter(require_screen_reasoning, |manifest| {
            transport_allowed_by_mode(policy.mode, manifest.transport)
                && manifest.privacy_level <= policy.minimum_privacy_level
        })
    }

    pub fn choose_backend_with_filter<F>(
        &self,
        require_screen_reasoning: bool,
        filter: F,
    ) -> Result<CapabilityManifest, RegistryError>
    where
        F: Fn(&CapabilityManifest) -> bool,
    {
        self.manifests_by_id
            .values()
            .filter(|manifest| filter(manifest))
            .filter(|manifest| !require_screen_reasoning || manifest.supports.screen_reasoning)
            .cloned()
            .max_by_key(|manifest| self.scoring_policy.score_manifest(manifest))
            .ok_or(RegistryError::NoBackendAvailable)
    }
}

fn transport_allowed_by_mode(mode: RuntimeMode, transport: TransportKind) -> bool {
    match mode {
        RuntimeMode::Local => transport != TransportKind::Api,
        RuntimeMode::Cloud => transport == TransportKind::Api,
        RuntimeMode::Hybrid => true,
    }
}

#[derive(Debug)]
pub enum RegistryError {
    InvalidId,
    AlreadyRegistered(String),
    NoBackendAvailable,
}

impl Display for RegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::InvalidId => formatter.write_str("manifest id is invalid"),
            RegistryError::AlreadyRegistered(id) => {
                write!(formatter, "manifest id already registered: {id}")
            }
            RegistryError::NoBackendAvailable => {
                formatter.write_str("no backend available for current policy")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{PrivacyLevel, RuntimeMode, StartupMode, SupportedCapabilities};

    #[test]
    fn selects_backend_for_local_mode() {
        let mut registry = CapabilityRegistry::new();

        registry
            .register(CapabilityManifest {
                id: "local-cli".to_string(),
                transport: TransportKind::Cli,
                supports: SupportedCapabilities {
                    chat: true,
                    streaming: true,
                    screen_reasoning: true,
                    tool_use: true,
                    ..Default::default()
                },
                privacy_level: PrivacyLevel::LocalFirst,
                startup_mode: StartupMode::Warm,
            })
            .expect("manifest should register");
        registry
            .register(CapabilityManifest {
                id: "cloud-api".to_string(),
                transport: TransportKind::Api,
                supports: SupportedCapabilities {
                    chat: true,
                    streaming: true,
                    ..Default::default()
                },
                privacy_level: PrivacyLevel::Cloud,
                startup_mode: StartupMode::Cold,
            })
            .expect("manifest should register");

        let chosen = registry
            .choose_backend(
                RuntimePolicy {
                    mode: RuntimeMode::Local,
                    minimum_privacy_level: PrivacyLevel::LocalFirst,
                },
                true,
            )
            .expect("backend should be available");

        assert_eq!(chosen.id, "local-cli");
    }

    #[test]
    fn selects_backend_for_cloud_mode() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(CapabilityManifest {
                id: "local-cli".to_string(),
                transport: TransportKind::Cli,
                supports: SupportedCapabilities {
                    chat: true,
                    streaming: true,
                    ..Default::default()
                },
                privacy_level: PrivacyLevel::LocalFirst,
                startup_mode: StartupMode::Warm,
            })
            .expect("manifest should register");
        registry
            .register(CapabilityManifest {
                id: "cloud-api".to_string(),
                transport: TransportKind::Api,
                supports: SupportedCapabilities {
                    chat: true,
                    streaming: true,
                    vision: true,
                    ..Default::default()
                },
                privacy_level: PrivacyLevel::Cloud,
                startup_mode: StartupMode::Cold,
            })
            .expect("manifest should register");

        let chosen = registry
            .choose_backend(
                RuntimePolicy {
                    mode: RuntimeMode::Cloud,
                    minimum_privacy_level: PrivacyLevel::Cloud,
                },
                false,
            )
            .expect("backend should be available");

        assert_eq!(chosen.id, "cloud-api");
    }

    #[test]
    fn selects_backend_for_hybrid_mode() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(CapabilityManifest {
                id: "local-cli".to_string(),
                transport: TransportKind::Cli,
                supports: SupportedCapabilities {
                    chat: true,
                    streaming: true,
                    screen_reasoning: true,
                    tool_use: true,
                    ..Default::default()
                },
                privacy_level: PrivacyLevel::LocalFirst,
                startup_mode: StartupMode::Warm,
            })
            .expect("manifest should register");
        registry
            .register(CapabilityManifest {
                id: "cloud-api".to_string(),
                transport: TransportKind::Api,
                supports: SupportedCapabilities {
                    chat: true,
                    streaming: true,
                    vision: true,
                    ..Default::default()
                },
                privacy_level: PrivacyLevel::Cloud,
                startup_mode: StartupMode::Cold,
            })
            .expect("manifest should register");

        let chosen = registry
            .choose_backend(
                RuntimePolicy {
                    mode: RuntimeMode::Hybrid,
                    minimum_privacy_level: PrivacyLevel::Cloud,
                },
                true,
            )
            .expect("backend should be available");

        assert_eq!(chosen.id, "local-cli");
    }
}
