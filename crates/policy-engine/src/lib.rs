use runtime_core::{CapabilityManifest, RuntimeMode, RuntimePolicy, TransportKind};

pub struct BackendPolicyEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadDataType {
    Screen,
    Audio,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspacePolicy {
    pub no_screen_upload: bool,
    pub no_audio_upload: bool,
    pub local_only: bool,
}

impl Default for WorkspacePolicy {
    fn default() -> Self {
        Self {
            no_screen_upload: false,
            no_audio_upload: false,
            local_only: false,
        }
    }
}

impl BackendPolicyEngine {
    pub fn manifest_is_allowed(
        manifest: &CapabilityManifest,
        runtime_policy: RuntimePolicy,
    ) -> bool {
        transport_allowed_by_mode(runtime_policy.mode, manifest.transport)
            && manifest.privacy_level <= runtime_policy.minimum_privacy_level
    }

    pub fn upload_is_allowed(
        upload_data_type: UploadDataType,
        workspace_policy: WorkspacePolicy,
        runtime_policy: RuntimePolicy,
    ) -> bool {
        if workspace_policy.local_only && runtime_policy.mode == RuntimeMode::Cloud {
            return false;
        }

        match upload_data_type {
            UploadDataType::Screen => !workspace_policy.no_screen_upload,
            UploadDataType::Audio => !workspace_policy.no_audio_upload,
            UploadDataType::Text => true,
        }
    }
}

fn transport_allowed_by_mode(runtime_mode: RuntimeMode, transport_kind: TransportKind) -> bool {
    match runtime_mode {
        RuntimeMode::Local => transport_kind != TransportKind::Api,
        RuntimeMode::Cloud => transport_kind == TransportKind::Api,
        RuntimeMode::Hybrid => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime_core::{PrivacyLevel, StartupMode, SupportedCapabilities};

    #[test]
    fn blocks_api_transport_in_local_mode() {
        let manifest = CapabilityManifest {
            id: "cloud-api".to_string(),
            transport: TransportKind::Api,
            supports: SupportedCapabilities::default(),
            privacy_level: PrivacyLevel::Cloud,
            startup_mode: StartupMode::Cold,
        };

        let allowed = BackendPolicyEngine::manifest_is_allowed(
            &manifest,
            RuntimePolicy {
                mode: RuntimeMode::Local,
                minimum_privacy_level: PrivacyLevel::Cloud,
            },
        );

        assert!(!allowed);
    }

    #[test]
    fn allows_cloud_api_in_cloud_mode_with_cloud_policy() {
        let manifest = CapabilityManifest {
            id: "cloud-api".to_string(),
            transport: TransportKind::Api,
            supports: SupportedCapabilities::default(),
            privacy_level: PrivacyLevel::Cloud,
            startup_mode: StartupMode::Cold,
        };

        let allowed = BackendPolicyEngine::manifest_is_allowed(
            &manifest,
            RuntimePolicy {
                mode: RuntimeMode::Cloud,
                minimum_privacy_level: PrivacyLevel::Cloud,
            },
        );

        assert!(allowed);
    }

    #[test]
    fn blocks_screen_upload_when_workspace_disables_screen_upload() {
        let allowed = BackendPolicyEngine::upload_is_allowed(
            UploadDataType::Screen,
            WorkspacePolicy {
                no_screen_upload: true,
                ..Default::default()
            },
            RuntimePolicy {
                mode: RuntimeMode::Hybrid,
                minimum_privacy_level: PrivacyLevel::Cloud,
            },
        );

        assert!(!allowed);
    }

    #[test]
    fn blocks_any_upload_when_workspace_is_local_only_and_mode_is_cloud() {
        let allowed = BackendPolicyEngine::upload_is_allowed(
            UploadDataType::Text,
            WorkspacePolicy {
                local_only: true,
                ..Default::default()
            },
            RuntimePolicy {
                mode: RuntimeMode::Cloud,
                minimum_privacy_level: PrivacyLevel::Cloud,
            },
        );

        assert!(!allowed);
    }
}
