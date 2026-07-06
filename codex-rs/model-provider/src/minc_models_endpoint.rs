use codex_login::default_client::build_reqwest_client;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::manager::ModelsEndpointClient;
use codex_models_manager::manager::ModelsEndpointFuture;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::error::CodexErr;
use codex_protocol::error::ConnectionFailedError;
use codex_protocol::error::Result as CoreResult;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::TruncationPolicyConfig;
use codex_protocol::openai_models::WebSearchToolType;
use reqwest::Url;
use serde::Deserialize;

#[derive(Debug)]
pub(crate) struct MincModelsEndpoint {
    provider_info: ModelProviderInfo,
}

impl MincModelsEndpoint {
    pub(crate) fn new(provider_info: ModelProviderInfo) -> Self {
        Self { provider_info }
    }

    async fn list_models(
        &self,
        _client_version: &str,
    ) -> CoreResult<(Vec<ModelInfo>, Option<String>)> {
        let base_url = self
            .provider_info
            .base_url
            .clone()
            .ok_or_else(|| CodexErr::Fatal("Minc provider missing base_url".to_string()))?;
        let url = Url::parse(&base_url)
            .map_err(|err| CodexErr::Fatal(format!("invalid Minc base_url: {err}")))?
            .join("/api/v1/models")
            .map_err(|err| CodexErr::Fatal(format!("invalid Minc models url: {err}")))?;

        let response = build_reqwest_client()
            .get(url)
            .send()
            .await
            .map_err(|source| CodexErr::ConnectionFailed(ConnectionFailedError { source }))?
            .error_for_status()
            .map_err(|source| CodexErr::ConnectionFailed(ConnectionFailedError { source }))?;
        let payload: MincModelsResponse = response
            .json()
            .await
            .map_err(|source| CodexErr::ConnectionFailed(ConnectionFailedError { source }))?;
        let models = payload
            .data
            .into_iter()
            .map(|model| minc_model_info(model.id.as_str()))
            .collect();
        Ok((models, None))
    }
}

impl ModelsEndpointClient for MincModelsEndpoint {
    fn has_command_auth(&self) -> bool {
        false
    }

    fn uses_codex_backend(&self) -> ModelsEndpointFuture<'_, bool> {
        // Reuse the shared refresh path so the models manager will fetch the
        // current Minc mode list from `/api/v1/models`.
        Box::pin(async { true })
    }

    fn replace_remote_models(&self) -> bool {
        true
    }

    fn list_models<'a>(
        &'a self,
        client_version: &'a str,
    ) -> ModelsEndpointFuture<'a, CoreResult<(Vec<ModelInfo>, Option<String>)>> {
        Box::pin(MincModelsEndpoint::list_models(self, client_version))
    }
}

#[derive(Debug, Deserialize)]
struct MincModelsResponse {
    data: Vec<MincModelRecord>,
}

#[derive(Debug, Deserialize)]
struct MincModelRecord {
    id: String,
}

fn minc_model_info(slug: &str) -> ModelInfo {
    let priority = match slug {
        "Auto" => 0,
        "Instant" => 1,
        "Low Reasoning" => 2,
        "High Reasoning" => 3,
        _ => 10,
    };
    let description = match slug {
        "Auto" => "Lets Minc choose the best mode for the turn.",
        "Instant" => "Fastest Minc mode for lightweight requests.",
        "Low Reasoning" => "Adds some extra reasoning without slowing to a crawl.",
        "High Reasoning" => "Most deliberate Minc mode for trickier tasks.",
        _ => "Minc mode",
    };

    ModelInfo {
        slug: slug.to_string(),
        display_name: slug.to_string(),
        description: Some(description.to_string()),
        default_reasoning_level: None,
        supported_reasoning_levels: Vec::new(),
        shell_type: ConfigShellToolType::ShellCommand,
        visibility: ModelVisibility::List,
        supported_in_api: true,
        priority,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        availability_nux: None,
        upgrade: None,
        base_instructions: "You are Minc Agent, a CLI coding assistant that can either answer directly or request a single local tool action using the documented directive grammar.".to_string(),
        model_messages: None,
        include_skills_usage_instructions: true,
        supports_reasoning_summaries: false,
        default_reasoning_summary: ReasoningSummary::None,
        support_verbosity: false,
        default_verbosity: None,
        apply_patch_tool_type: Some(ApplyPatchToolType::Freeform),
        web_search_tool_type: WebSearchToolType::Text,
        truncation_policy: TruncationPolicyConfig::bytes(32_000),
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: Some(128_000),
        max_context_window: Some(128_000),
        auto_compact_token_limit: None,
        comp_hash: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
        input_modalities: vec![InputModality::Text, InputModality::Image],
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        use_responses_lite: false,
        auto_review_model_override: None,
        tool_mode: None,
        multi_agent_version: None,
    }
}
