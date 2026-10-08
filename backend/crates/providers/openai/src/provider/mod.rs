//! Codex 的 `gateway-core` Provider adapter

use std::collections::BTreeSet;
use std::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use async_trait::async_trait;
use bytes::Bytes;
use futures::{StreamExt, future::BoxFuture};
use gateway_core::account::{AccountFeedbackStats, ProviderAccount};
use gateway_core::engine::continuation::{ContinuationBinding, NativeContinuationScope};
use gateway_core::engine::middleware::MiddlewareHeader;
use gateway_core::engine::provider::{
    ContinuationRequestObservation, EventStream, Provider, ProviderCallMetadata, ProviderRequest,
    ProviderRequestObservation, ProviderSelectionObservation, ProviderStream,
};
use gateway_core::engine::{AttemptContext, AttemptTransport, ContinuationAttempt};
use gateway_core::error::{
    ClientVisibleUpstreamError, ClientVisibleUpstreamResponse, ContinuationFailure,
    ContinuationRecoveryDisposition, OpaqueUpstreamValue, ProviderConnectionObservation,
    ProviderDiagnostic, ProviderError, ProviderErrorKind, RawUpstreamError,
};
use gateway_core::event::{
    FinishReason, GatewayEvent, ProtocolWireEvent, ProviderEvent, ProviderResponseHeader,
    ProviderResponseMetadata, ProviderResponseObservation, ProviderResponseTimings, ResponseMeta,
    UpstreamHttpVersion, WebSocketPoolKind,
};
use gateway_core::lifecycle::CancellationToken;
use gateway_core::operation::{
    CapabilityRequirements, GenerateRequest, ImageRequest, ImageRequestKind, Operation,
    OperationKind, ProviderSessionState, StandaloneSearchRequest,
};
use gateway_core::provider_ports::ProviderSessionAffinityKey;
use gateway_core::routing::{
    ModelCapabilities, ModelPresentation, ProviderCandidate, ProviderCatalogGeneration,
    ProviderKind, ProviderModelCapabilities, UpstreamModelId,
};
use gateway_core::task::{
    DaemonRestartPolicy, DaemonTask, ScheduledTask, WorkerContribution, WorkerCycleContext,
    WorkerDefinitionError, WorkerId, WorkerKind, WorkerLeaseRequest, WorkerRegistration,
    WorkerRunnable, WorkerSchedule, WorkerTaskError,
};
use gateway_core::upstream::{UpstreamSendState, UpstreamTransport};
use gateway_protocol::openai::events::{
    ParsedRateLimits, parse_rate_limit_headers, rate_limits_to_header_pairs,
};
use reqwest::{Client, header::HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use url::Url;
use uuid::Uuid;

use crate::credential::{
    CodexAccountFailure, CodexCredentialCatalogError, CodexCredentialCatalogService,
    CodexCredentialLease, CodexCredentialQuotaService, CodexCredentialRefreshOutcome,
    CodexCredentialRefreshService, CodexCredentialRepository, CodexCredentialSelector,
    CodexCyberPolicyScope, CodexQuotaRefreshPolicy, CodexSessionAffinity, CredentialSelectionError,
    RuntimeCodexCookie, SelectCodexCredential, SelectCodexProviderEndpointCredential,
    derive_codex_cyber_policy_session_key, derive_codex_endpoint_session_affinity,
    derive_codex_session_affinity, derive_codex_transport_key,
    derive_endpoint_affinity_with_headers, derive_live_session_affinity,
    derive_previous_response_id_hash, derive_turn_alias,
};
use crate::request_identity::{account_session_with_headers, turn_id_with_headers};
use crate::session_transport::CodexSessionTransportRecovery;
use crate::transport::canonical::{
    CodexCanonicalDecoder, CodexCanonicalError, CodexCanonicalOutcome,
};
use crate::transport::catalog::{
    CodexCatalogCapabilityEvidence, CodexCatalogModel, CodexCatalogVisibility,
};
use crate::transport::diagnostics::{
    CodexFailureCategory, CodexUpstreamFailure, CodexUpstreamSendPhase,
};
use crate::transport::input_guard::{InputGuardConfig, InputGuardDecision, decide_input_guard};
use crate::transport::profile::{
    APPCAST_POLL_INTERVAL, CodexDesktopReleaseService, CodexWireProfileState,
};
use crate::transport::protocol::responses::{
    CodexResponsesRequest, PREVIOUS_RESPONSE_NOT_FOUND_CODE, PREVIOUS_RESPONSE_NOT_FOUND_MESSAGE,
    PreviousResponseScope, ResponseEventSignals, TransportRequirement, transport_requirement,
};
use crate::transport::protocol::websocket::websocket_response_create_payload_len;
use crate::transport::request::{
    CodexRequestEncodeError, RequestAccountScope, align_structured_location_fields,
    clear_request_turn_state, encode_generate_request, scope_request_to_account,
};
use crate::transport::review_override::{self, ReviewOverride};
use crate::transport::session::CodexSessionIdentity;
use crate::transport::usage::normalize_service_tier;
use crate::transport::websocket::{
    CodexWebSocketExchangeError, PreviousResponseUnavailableReason, WEBSOCKET_CLOSE_MESSAGE_TOO_BIG,
};
use crate::transport::{
    CODEX_ALPHA_SEARCH_PATH, CODEX_IMAGE_EDITS_PATH, CODEX_IMAGE_GENERATIONS_PATH,
    CODEX_REALTIME_CALLS_PATH, CODEX_RESPONSES_PATH, CodexAccountSelectionTelemetry,
    CodexBackendClient, CodexBackendJsonResponse, CodexBackendStreamingResponse,
    CodexBackendTransport, CodexClientError, CodexRateLimitUpdates, CodexRequestContext,
    CodexResponseMetadata, CodexResponseMetadataUpdates, CodexTransportMetrics,
    CodexUpstreamDiagnostics, CodexWebSocketPool, endpoint_url,
    normalize_selected_codex_downstream_body, normalize_universal_history_cleanup,
};

mod execution;
mod failure;
#[doc(hidden)]
pub use failure::context_length_exceeded_error;
mod live;
mod observation;
mod upstream_adapter;
mod workers;
pub(crate) use live::{CodexLiveGateway, CodexLiveRegistry};
pub(crate) use workers::ClientReleaseServices;

use execution::*;
#[doc(hidden)]
pub use failure::openai_failure_affects_account_score;
use failure::*;
use observation::*;
pub(crate) use workers::worker_contributions;

const PROVIDER_NAME: &str = "openai";
const HTTP_SSE_TRANSPORT: &str = "http_sse";
const HTTP_JSON_TRANSPORT: &str = "http_json";
const WEBSOCKET_TRANSPORT: &str = "websocket";
// 在已观测到的 Codex OAuth 上游 16 MiB 附近消息边界前留出传输 metadata 余量
const WEBSOCKET_HTTP_FALLBACK_THRESHOLD_BYTES: usize = 15 * 1024 * 1024;
const MAX_COOKIE_HEADER_BYTES: usize = 16 * 1024;
/// 提交边界前预取 128 KiB 原始上游 chunk；容纳携带配置回显的前导事件，
/// 超过阈值后结束无感换号窗口（最后一个 chunk 可越过阈值），
/// 但不会把上游数据改写成协议失败
const MAX_STREAM_PREFETCH_BYTES: usize = 128 * 1024;
/// 短暂保留 response.created 等结构事件，让随后到达的明确拒绝可以无感换号；
/// 到期即放行，避免模型长时间思考时让客户端一直收不到首事件
const STREAM_REPLAY_GRACE: Duration = Duration::from_millis(2_500);
// 额度拒绝后的 usage 补查自身执行时长上限；结算等待的随机延迟见 jitter 模块
const QUOTA_FAILURE_REFRESH_TIMEOUT: Duration = Duration::from_secs(5);
pub const OFFICIAL_CODEX_BASE_PATH: &str = "/backend-api";
pub const OFFICIAL_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexProviderTransport {
    HttpOnly,
    PreferWebSocket,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodexProviderConfigError {
    #[error("Codex provider URL is invalid")]
    InvalidBaseUrl,
}

#[derive(Clone)]
pub struct CodexProvider {
    selector: Arc<CodexCredentialSelector>,
    catalog: Arc<CodexCredentialCatalogService>,
    quota: Arc<CodexCredentialQuotaService>,
    account_feedback: Arc<AccountFeedbackStats>,
    client: CodexBackendClient,
    responses_url: Url,
    image_generations_url: Url,
    image_edits_url: Url,
    search_url: Url,
    live_calls_url: Url,
    session_identity: Option<CodexSessionIdentity>,
    session_transport_recovery: CodexSessionTransportRecovery,
    stream_max_retries: u32,
    input_guard: InputGuardConfig,
    live_registry: Arc<CodexLiveRegistry>,
    live_gateway: Option<Arc<CodexLiveGateway>>,
}

struct PreparedGenerateRequest {
    upstream: CodexResponsesRequest,
    previous_session: Option<OpenAiSessionState>,
    continuation_requested: bool,
    session_affinity: Option<CodexSessionAffinity>,
    cyber_policy_session_key: Option<ProviderSessionAffinityKey>,
}

impl PreparedGenerateRequest {
    fn checked_native_continuation_scope(
        &self,
        context: &AttemptContext,
    ) -> Result<PreviousResponseScope, ProviderError> {
        let pin = context.continuation().and_then(ContinuationBinding::pinned);
        let scope = match self
            .previous_session
            .as_ref()
            .map(|state| state.continuation_scope)
        {
            Some(OpenAiContinuationScope::Persisted) => PreviousResponseScope::Persisted,
            Some(OpenAiContinuationScope::ConnectionLocal) => {
                PreviousResponseScope::ConnectionLocal
            }
            Some(OpenAiContinuationScope::ReplayRequired) => PreviousResponseScope::ExternalUnknown,
            None => match pin.map(|pin| pin.scope()) {
                Some(NativeContinuationScope::Persisted) => PreviousResponseScope::Persisted,
                Some(NativeContinuationScope::ConnectionLocal) => {
                    PreviousResponseScope::ConnectionLocal
                }
                None => PreviousResponseScope::ExternalUnknown,
            },
        };
        // 已知不能脱离原连接的重放必须在选号前拒绝，避免失败请求先改写共享绑定
        // Native 仍交给连接池核对原连接；此处不推测连接是否存活或上游状态是否可用
        if pin.is_some()
            && matches!(
                context.continuation_attempt(),
                ContinuationAttempt::ReplayOwner | ContinuationAttempt::ReplayAny
            )
            && scope == PreviousResponseScope::ConnectionLocal
        {
            tracing::warn!(
                request_id = context.request_id().as_str(),
                attempt_index = context.attempt_index().get(),
                continuation_scope = "connection_local",
                continuation_attempt = context.continuation_attempt().as_str(),
                continuation_recovery_disposition = "client_replay_required",
                continuation_recovery_action = "stop_proxy_recovery",
                "OpenAI connection-local continuation replay was rejected before send"
            );
            return Err(continuation_replay_required_error("scope_unavailable"));
        }
        Ok(scope)
    }
}

struct SelectedGenerate {
    lease: CodexCredentialLease,
    account_selection_wait_ms: u64,
    frozen_requirements: CapabilityRequirements,
}

impl CodexProvider {
    fn prepare_generate_request(
        &self,
        generate: &GenerateRequest,
        mut upstream: CodexResponsesRequest,
        context: &AttemptContext,
    ) -> PreparedGenerateRequest {
        let previous_session = decode_openai_session_state(generate);
        let continuation_requested = generate.native_continuation_requested();
        if let Some(conversation_id) = previous_session
            .as_ref()
            .and_then(|state| state.conversation_id.as_ref())
        {
            upstream.local_conversation_id = Some(conversation_id.clone());
        }
        if let Some(identity) = &self.session_identity {
            identity.prepare_local_conversation(&mut upstream);
        }
        if let Some(previous_session) = previous_session.as_ref() {
            if same_client_turn(
                previous_session.client_turn_id.as_deref(),
                upstream.client_turn_id.as_deref(),
            ) {
                upstream.turn_state = upstream
                    .turn_state
                    .take()
                    .or_else(|| previous_session.turn_state.clone());
            } else {
                clear_request_turn_state(&mut upstream);
            }
        }
        let session_affinity =
            derive_codex_session_affinity(&upstream, context.client_api_key_ref());
        let cyber_policy_session_key =
            derive_codex_cyber_policy_session_key(&upstream, context.client_api_key_ref());
        PreparedGenerateRequest {
            upstream,
            previous_session,
            continuation_requested,
            session_affinity,
            cyber_policy_session_key,
        }
    }

    fn client_for_request(
        &self,
        context: &AttemptContext,
    ) -> Result<CodexBackendClient, ProviderError> {
        let Some(profile) = context.request_profile() else {
            return Ok(self.client.clone());
        };
        let profile = serde_json::from_value(Value::Object(profile.expose_to_provider().clone()))
            .map_err(|error: serde_json::Error| {
            let diagnostic = ProviderDiagnostic::new(format!(
                "OpenAI request profile decoding failed: {:?}, line={}, column={}",
                error.classify(),
                error.line(),
                error.column(),
            ))
            .with_classification("prepare", "request_profile_decode_failed");
            provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent)
                .with_diagnostic(diagnostic)
        })?;
        Ok(self.client.clone().with_request_profile(profile))
    }

    // Provider 构造集中装配独立领域服务和透明传输依赖，拆分参数会模糊所有权
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        selector: Arc<CodexCredentialSelector>,
        catalog: Arc<CodexCredentialCatalogService>,
        quota: Arc<CodexCredentialQuotaService>,
        account_feedback: Arc<AccountFeedbackStats>,
        http: Client,
        profile: CodexWireProfileState,
        base_url: String,
        websocket_pool: Arc<CodexWebSocketPool>,
        stream_max_retries: u32,
    ) -> Result<Self, CodexProviderConfigError> {
        let responses_url = Url::parse(&endpoint_url(&base_url, CODEX_RESPONSES_PATH))
            .map_err(|_| CodexProviderConfigError::InvalidBaseUrl)?;
        let image_generations_url =
            Url::parse(&endpoint_url(&base_url, CODEX_IMAGE_GENERATIONS_PATH))
                .map_err(|_| CodexProviderConfigError::InvalidBaseUrl)?;
        let image_edits_url = Url::parse(&endpoint_url(&base_url, CODEX_IMAGE_EDITS_PATH))
            .map_err(|_| CodexProviderConfigError::InvalidBaseUrl)?;
        let search_url = Url::parse(&endpoint_url(&base_url, CODEX_ALPHA_SEARCH_PATH))
            .map_err(|_| CodexProviderConfigError::InvalidBaseUrl)?;
        let live_calls_url = Url::parse(&endpoint_url(&base_url, CODEX_REALTIME_CALLS_PATH))
            .map_err(|_| CodexProviderConfigError::InvalidBaseUrl)?;
        let client =
            CodexBackendClient::new(http, base_url, profile).with_websocket_pool(websocket_pool);
        Ok(Self {
            selector,
            catalog,
            quota,
            account_feedback,
            client,
            responses_url,
            image_generations_url,
            image_edits_url,
            search_url,
            live_calls_url,
            session_identity: None,
            session_transport_recovery: CodexSessionTransportRecovery::default(),
            stream_max_retries,
            input_guard: InputGuardConfig::default(),
            live_registry: Arc::new(CodexLiveRegistry::default()),
            live_gateway: None,
        })
    }

    #[must_use]
    pub(crate) fn with_timezone(
        mut self,
        timezone: gateway_core::time::DeploymentTimeZone,
    ) -> Self {
        self.client = self.client.with_timezone(timezone);
        self
    }

    pub(crate) fn with_session_identity(mut self, identity: CodexSessionIdentity) -> Self {
        self.session_identity = Some(identity);
        self
    }

    /// 设置 GPT 长上下文输入守卫；默认关闭，由运行设置显式打开
    #[must_use]
    pub fn with_input_guard(mut self, config: InputGuardConfig) -> Self {
        self.input_guard = config;
        self
    }

    /// 绑定 Live 语音支持：call 注册表 + 账号级 sideband 网关
    /// 需要凭据仓库句柄；未绑定时不暴露 sideband 与 hangup 能力
    pub(crate) fn with_live_support(mut self, repository: CodexCredentialRepository) -> Self {
        let gateway = Arc::new(CodexLiveGateway::new(
            Arc::clone(&self.live_registry),
            repository,
            self.client.clone(),
        ));
        self.live_gateway = Some(gateway);
        self
    }
}

impl fmt::Debug for CodexProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodexProvider")
            .field("selector", &"<account-selector>")
            .field("catalog", &"<ttl-catalog>")
            .finish()
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn resolve_request_profile(
        &self,
        configuration: &gateway_core::account::OpaqueProviderData,
    ) -> Result<gateway_core::account::OpaqueProviderData, ProviderError> {
        let selection =
            crate::transport::profile::identity::RequestProfileSelection::parse(configuration)
                .map_err(|error| {
                    provider_error(
                        ProviderErrorKind::InvalidRequest,
                        UpstreamSendState::NotSent,
                    )
                    .with_diagnostic(profile_diagnostic(&error))
                })?;
        let profile = selection
            .resolve(self.client.profile_state())
            .map_err(|error| {
                provider_error(ProviderErrorKind::Unavailable, UpstreamSendState::NotSent)
                    .with_diagnostic(profile_diagnostic(&error))
            })?;
        crate::transport::profile::selection::object(&profile).map_err(|error| {
            provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent)
                .with_diagnostic(profile_diagnostic(&error))
        })
    }

    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn live_gateway(&self) -> Option<Arc<dyn gateway_core::live::LiveGateway>> {
        self.live_gateway
            .clone()
            .map(|gateway| gateway as Arc<dyn gateway_core::live::LiveGateway>)
    }

    fn catalog_generation(&self) -> ProviderCatalogGeneration {
        self.catalog.catalog_generation()
    }

    fn request_observation(
        &self,
        operation: &Operation,
        client_api_key_id: &gateway_core::policy::ClientApiKeyId,
    ) -> ProviderRequestObservation {
        let Operation::Generate(request) = operation else {
            let affinity = match operation {
                Operation::Search(request) => derive_codex_endpoint_session_affinity(
                    request.payload(),
                    client_api_key_id,
                    "id",
                ),
                Operation::GenerateImage(request) => derive_codex_endpoint_session_affinity(
                    request.payload(),
                    client_api_key_id,
                    "session_id",
                ),
                _ => None,
            };
            return ProviderRequestObservation {
                requested_model: match operation {
                    Operation::Search(request) => {
                        observation::endpoint_requested_model(request.payload())
                    }
                    Operation::GenerateImage(request) => {
                        observation::endpoint_requested_model(request.payload())
                    }
                    _ => None,
                },
                continuation: ContinuationRequestObservation {
                    affinity_hash: affinity.map(|affinity| affinity.persistence_hash().to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            };
        };
        let Ok(encoded) = encode_generate_request(request, "observability", None) else {
            return ProviderRequestObservation::default();
        };
        let semantics = encoded.semantics();
        let reasoning_effort = semantics.reasoning_effort.clone();
        let previous_response_id = encoded.previous_response_id();
        let continuation = ContinuationRequestObservation {
            affinity_hash: derive_codex_session_affinity(&encoded, client_api_key_id)
                .map(|affinity| affinity.persistence_hash().to_owned()),
            previous_response_id_hash: previous_response_id.map(|response_id| {
                derive_previous_response_id_hash(response_id, client_api_key_id)
            }),
            requested: previous_response_id.is_some(),
        };
        ProviderRequestObservation {
            requested_model: None,
            reasoning_effort,
            reasoning_preset: semantics.reasoning_preset.map(str::to_owned),
            request_kind: semantics.request_kind,
            subagent_kind: semantics.subagent_kind,
            compact: semantics.compact,
            continuation,
        }
    }

    fn model_catalog_is_exhaustive(&self) -> bool {
        false
    }

    async fn query_model_capabilities(
        &self,
    ) -> Result<Vec<ProviderModelCapabilities>, ProviderError> {
        let snapshot = self.catalog.synchronize().await.map_err(|_| {
            provider_error(ProviderErrorKind::Unavailable, UpstreamSendState::NotSent)
        })?;
        let mut accounts = snapshot.model_catalog_accounts();
        Ok(snapshot
            .models()
            .iter()
            .map(|model| {
                compile_model_capabilities(model).with_catalog_accounts(
                    accounts
                        .remove(model.request_model().as_str())
                        .unwrap_or_default(),
                )
            })
            .collect())
    }

    async fn query_client_model_catalog(
        &self,
        scope: &gateway_core::account::scope::FrozenAccountScope,
        protocol: &str,
        client_version: &str,
    ) -> Result<
        Option<Vec<gateway_core::routing::ProviderModelDescriptor>>,
        gateway_core::routing::ProviderCatalogUnavailable,
    > {
        if protocol != "codex" {
            return Ok(None);
        }
        self.catalog
            .client_model_catalog(scope, client_version)
            .await
            .map(Some)
            .map_err(|_| gateway_core::routing::ProviderCatalogUnavailable)
    }

    async fn execute(
        self: Arc<Self>,
        request: ProviderRequest,
        context: AttemptContext,
    ) -> Result<ProviderStream, ProviderError> {
        if request.candidate().provider().as_str() != PROVIDER_NAME {
            return Err(provider_error(
                ProviderErrorKind::InvalidRequest,
                UpstreamSendState::NotSent,
            ));
        }
        let candidate = request.candidate();
        if context.cancellation().is_cancelled() {
            return Err(provider_error(
                ProviderErrorKind::Cancelled,
                UpstreamSendState::NotSent,
            ));
        }
        if context.deadline().is_elapsed() {
            return Err(provider_error(
                ProviderErrorKind::Timeout,
                UpstreamSendState::NotSent,
            ));
        }
        if let Operation::GenerateImage(image) = request.operation() {
            return self.execute_image(image, candidate, context).await;
        }
        if let Operation::Search(search) = request.operation() {
            return self.execute_search(search, candidate, context).await;
        }
        if let Operation::ProviderHttp(request) = request.operation() {
            return self
                .execute_live_call(request.clone(), candidate.upstream_model(), context)
                .await;
        }
        let Operation::Generate(generate) = request.operation() else {
            return Err(provider_error(
                ProviderErrorKind::Unsupported,
                UpstreamSendState::NotSent,
            ));
        };
        // 请求设置先形成原生正文基线；attempt 的显式改写进入终端后不再次被覆盖
        let mut operation = Operation::Generate(generate.clone());
        // 审核子代理覆盖（§5）必须在选号前落到操作上：选号、计费与上游正文都读
        // 模型名，只有先改写它们才会指向同一个模型。目录没有候选或不是审核请求时
        // 保持客户端选择，绝不猜一个可能不存在的模型
        let review = review_request_override(&self.catalog, &operation);
        if let Some(review) = review.as_ref() {
            operation = apply_review_override(operation, review)?;
        }
        let Some(routed_model) = candidate.upstream_model() else {
            return Err(provider_error(
                ProviderErrorKind::Protocol,
                UpstreamSendState::NotSent,
            ));
        };
        let upstream_model = match review.as_ref() {
            Some(review) => UpstreamModelId::new(review.upstream_model.clone()).map_err(|_| {
                provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent)
            })?,
            None => routed_model.clone(),
        };
        // Fast 档位改写排在审核覆盖之后：两者都改写本次发送副本，审核覆盖决定模型与
        // 推理强度，Fast 只改服务档位，后执行才能覆盖到审核改写后的正文
        if context.fast_mode() != gateway_core::account::FastMode::Default
            && let Operation::Generate(generate) = &operation
            && generate.protocol_payload().protocol() == PROVIDER_NAME
        {
            let mut request =
                CodexResponsesRequest::from_body(generate.protocol_payload().body().clone());
            let supports_priority = candidate.model_presentation().is_some_and(|model| {
                model
                    .service_tiers()
                    .iter()
                    .any(|tier| tier.id() == "priority")
            });
            if request.apply_fast_policy(context.fast_mode(), supports_priority) {
                let body = serde_json::to_vec(request.body()).map_err(|_| {
                    provider_error(
                        ProviderErrorKind::InvalidRequest,
                        UpstreamSendState::NotSent,
                    )
                })?;
                operation = operation
                    .replace_middleware_wire(PROVIDER_NAME, body.into())
                    .map_err(|_| {
                        provider_error(
                            ProviderErrorKind::InvalidRequest,
                            UpstreamSendState::NotSent,
                        )
                    })?;
            }
        }
        let Operation::Generate(generate) = &operation else {
            unreachable!("generate settings keep the operation kind")
        };
        let adapter = context.upstream_adapter(candidate.provider(), &upstream_model)?;
        if adapter.is_none()
            && generate
                .provider_session_state(PROVIDER_NAME)
                .is_some_and(|state| state.extension_owner().is_some())
        {
            return Err(continuation_replay_required_error("scope_unavailable"));
        }
        // 其他协议必须先取得真实账号，再按固定 attempt 阶段调用转换器；选号前不能
        // 把未知正文当成 OpenAI wire 解释会话、亲和或传输字段
        let upstream = (generate.protocol_payload().protocol() == PROVIDER_NAME)
            .then(|| encode_generate_request(generate, upstream_model.as_str(), None))
            .transpose()
            .map_err(map_request_error)?;
        // 固定调度约束：Guardian 分类先于选号，不能随原生/适配器发送路径改变
        // 复用编码器的权威 metadata 解析，但只为原生路径准备会话与传输状态
        let guardian = upstream
            .as_ref()
            .is_some_and(CodexResponsesRequest::is_guardian);
        let preselection = upstream
            .filter(|_| adapter.is_none())
            .map(|upstream| self.prepare_generate_request(generate, upstream, &context));
        if let Some(prepared) = preselection.as_ref() {
            prepared.checked_native_continuation_scope(&context)?;
        }
        let (selection_session_affinity, selection_cyber_policy_key, requires_websocket) =
            preselection.map_or((None, None, false), |prepared| {
                let requires_websocket =
                    transport_requirement(&prepared.upstream).requires_websocket()
                        || (context.continuation_attempt() == ContinuationAttempt::Native
                            && (prepared.previous_session.as_ref().is_some_and(|state| {
                                state.continuation_scope
                                    == OpenAiContinuationScope::ConnectionLocal
                            }) || matches!(context.continuation(), Some(ContinuationBinding::Pinned(binding))
                                    if binding.scope() == NativeContinuationScope::ConnectionLocal)));
                (
                    prepared.session_affinity,
                    prepared.cyber_policy_session_key,
                    requires_websocket,
                )
            });
        let selection_started_at = Instant::now();
        let selection = async {
            self.selector
                .select_with_cyber_policy(
                    &SelectCodexCredential {
                        upstream_model: upstream_model.as_str(),
                        request_url: &self.responses_url,
                        attempt: &context,
                        session_affinity_key: selection_session_affinity
                            .as_ref()
                            .map(|affinity| affinity.key()),
                    },
                    selection_cyber_policy_key.as_ref(),
                    selection_session_affinity.as_ref(),
                    requires_websocket,
                    guardian,
                )
                .await
                .map_err(map_selection_error)
        };
        // 恢复期间排队也消耗启动窗口；只包住选账号，不限制业务响应时长
        let lease = if let Some(remaining) = context.connection_budget().startup_remaining() {
            tokio::time::timeout(remaining, selection)
                .await
                .map_err(|_| {
                    provider_error(ProviderErrorKind::Timeout, UpstreamSendState::NotSent)
                })??
        } else {
            selection.await?
        };
        let account_selection_wait_ms =
            u64::try_from(selection_started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        let provider_kind = ProviderKind::new(PROVIDER_NAME)
            .map_err(|_| provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent))?;
        let account_id = lease.account_id().clone();
        let frozen_requirements = Operation::Generate(generate.clone()).capability_requirements();
        let provider = Arc::clone(&self);
        let terminal_context = context.clone();
        let terminal_model = upstream_model.clone();
        context
            .execute_middleware(
                Operation::Generate(generate.clone()),
                provider_kind,
                Some(upstream_model.as_str().to_owned()),
                account_id,
                Box::new(move |operation, middleware_headers| {
                    Box::pin(async move {
                        if let Some(adapter) = adapter {
                            return provider.execute_upstream_adapter(
                                operation,
                                middleware_headers,
                                terminal_model,
                                terminal_context,
                                lease,
                                account_selection_wait_ms,
                                frozen_requirements,
                                adapter,
                            );
                        }
                        provider
                            .execute_selected_generate(
                                operation,
                                middleware_headers,
                                terminal_model,
                                terminal_context,
                                SelectedGenerate {
                                    lease,
                                    account_selection_wait_ms,
                                    frozen_requirements,
                                },
                            )
                            .await
                    })
                }),
            )
            .await
    }
}

impl CodexProvider {
    async fn execute_selected_generate(
        self: Arc<Self>,
        operation: Operation,
        middleware_headers: Vec<MiddlewareHeader>,
        upstream_model: UpstreamModelId,
        context: AttemptContext,
        selected: SelectedGenerate,
    ) -> Result<ProviderStream, ProviderError> {
        let SelectedGenerate {
            mut lease,
            account_selection_wait_ms,
            frozen_requirements,
        } = selected;
        let Operation::Generate(generate) = operation else {
            return Err(provider_error(
                ProviderErrorKind::Protocol,
                UpstreamSendState::NotSent,
            ));
        };
        if generate.protocol_payload().protocol() != PROVIDER_NAME
            || native_request_requirements(&generate) != frozen_requirements
        {
            return Err(provider_error(
                ProviderErrorKind::InvalidRequest,
                UpstreamSendState::NotSent,
            ));
        }
        validate_openai_reasoning(generate.protocol_payload().body())?;
        let mut upstream = encode_generate_request(&generate, upstream_model.as_str(), None)
            .map_err(map_request_error)?;
        upstream.client_account_follow_only = crate::request_identity::follows_session_with_headers(
            generate.protocol_payload().body(),
            generate.protocol_payload().context(),
            &middleware_headers,
        );
        upstream.client_account_session_id = account_session_with_headers(
            generate.protocol_payload().body(),
            generate.protocol_payload().context(),
            &middleware_headers,
        );
        let processed = self.prepare_generate_request(&generate, upstream, &context);
        let native_scope = processed.checked_native_continuation_scope(&context)?;
        let mut upstream_request = processed.upstream;
        let session_transport_key =
            derive_codex_transport_key(&upstream_request, context.client_api_key_ref());
        let previous_session = processed.previous_session;
        let continuation_requested = processed.continuation_requested;
        let session_affinity = processed.session_affinity;
        let cyber_policy_session_key = processed.cyber_policy_session_key;
        if !context.is_diagnostic_required_account() {
            self.selector
                .validate_translated_selection(
                    &mut lease,
                    session_affinity.as_ref(),
                    cyber_policy_session_key.as_ref(),
                    context.account_selection_policy(),
                )
                .await
                .map_err(map_selection_error)?;
        }
        if !context.is_diagnostic_required_account()
            && let Some(affinity) = session_affinity.as_ref()
            && let Some(turn) = turn_id_with_headers(
                generate.protocol_payload().body(),
                generate.protocol_payload().context(),
                &middleware_headers,
            )
            .and_then(|turn| derive_turn_alias(&turn, context.client_api_key_ref()))
        {
            self.selector
                .remember_turn(
                    &turn,
                    affinity,
                    context
                        .account_selection_policy()
                        .openai_session_affinity_ttl(),
                )
                .await
                .map_err(map_selection_error)?;
        }
        let lease = Arc::new(lease);
        if previous_session.as_ref().is_some_and(|state| {
            state
                .credential_revision
                .is_some_and(|revision| revision != lease.account().revision().get())
        }) {
            return Err(continuation_replay_required_error("scope_unavailable"));
        }
        let provider_kind = ProviderKind::new(PROVIDER_NAME)
            .map_err(|_| provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent))?;
        let native_owner = context.continuation().and_then(ContinuationBinding::pinned);
        let state_owner_cross_account = native_owner
            .is_some_and(|owner| !owner.matches(&provider_kind, lease.account_id()))
            || context
                .account_state_owner()
                .is_some_and(|owner| !owner.matches(&provider_kind, lease.account_id()))
            || previous_session
                .as_ref()
                .is_some_and(|state| state.account_id != lease.account_id().as_str());
        let has_explicit_state_owner = context.account_state_owner().is_some()
            || previous_session.is_some()
            || native_owner.is_some();
        let account_scope =
            if state_owner_cross_account || (!has_explicit_state_owner && lease.account_switch()) {
                RequestAccountScope::Different
            } else if has_explicit_state_owner {
                RequestAccountScope::Same
            } else {
                RequestAccountScope::Unknown
            };
        if context.continuation_attempt() != ContinuationAttempt::None
            && let Some(continuation) = context.continuation()
        {
            match continuation {
                ContinuationBinding::Pinned(continuation) => {
                    let previous_response_scope = match context.continuation_attempt() {
                        ContinuationAttempt::Native => native_scope,
                        ContinuationAttempt::ReplayOwner | ContinuationAttempt::ReplayAny => {
                            PreviousResponseScope::ExternalUnknown
                        }
                        ContinuationAttempt::None => PreviousResponseScope::ExternalUnknown,
                    };
                    upstream_request.set_previous_response_id(Some(
                        continuation.upstream_response_id().as_str().to_owned(),
                    ));
                    upstream_request.previous_response_scope = Some(previous_response_scope);
                }
                ContinuationBinding::External(previous_response_id) => {
                    upstream_request
                        .set_previous_response_id(Some(previous_response_id.as_str().to_owned()));
                    upstream_request.previous_response_scope =
                        Some(PreviousResponseScope::ExternalUnknown);
                }
            }
        }
        if upstream_request.previous_response_id().is_some()
            && !account_scope.can_reuse_account_state()
        {
            return Err(continuation_replay_required_error("scope_unavailable"));
        }
        if upstream_request
            .downstream_websocket_connection_id
            .is_some()
            && upstream_request.previous_response_id().is_some()
            && upstream_request.generate()
            && previous_session.as_ref().is_some_and(|state| {
                state.continuation_scope == OpenAiContinuationScope::ReplayRequired
            })
        {
            // HTTP store=false 没有原生续链，完整历史仍由客户端持有；不能把 delta
            // 当作独立新请求发送，也不能用 previous_response_id 猜测上游存储状态
            return Err(continuation_replay_required_error("scope_unavailable"));
        }
        scope_request_to_account(
            &mut upstream_request,
            lease.installation_id(),
            account_scope,
        );
        let codex_oauth = matches!(
            lease.authentication(),
            crate::credential::CodexRuntimeAuthentication::OAuth(_)
        );
        // 跨模型历史清洗：按最终上游模型族执行，API Key 与 OAuth 路径都覆盖（规范 §6 末条），
        // 但只对 GPT 系列套用 GPT 的形状约束（§6-1「GPT 系列出站前」）。编码阶段已把最终
        // 上游模型名写进正文，因此这里直接读正文即可，不需要再传一个可能漂移的副本。
        let cleanup_model = upstream_request
            .body()
            .get("model")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        normalize_universal_history_cleanup(upstream_request.body_mut(), &cleanup_model);
        if codex_oauth {
            normalize_selected_codex_downstream_body(
                upstream_request.body_mut(),
                generate.protocol_payload().context(),
            );
        }
        // 网关本地压缩摘要（`cpr-local-v1:`）还原：带标记的历史项换回普通 user 摘要
        // 消息；非法载荷直接按 InvalidRequest 拒绝。还原后旧续接链不再成立
        // （摘要取代了此前全部历史），因此清掉 previous_response_id 及其作用域。
        if let Some(input) = upstream_request
            .body_mut()
            .get_mut("input")
            .and_then(serde_json::Value::as_array_mut)
        {
            match crate::transport::local_compaction::restore_local_compaction_history(input) {
                Ok(true) => upstream_request.set_previous_response_id(None),
                Ok(false) => {}
                Err(error) => return Err(failure::invalid_local_compaction_error(error)),
            }
        }
        // GPT 长上下文输入守卫（§4）：按**最终上游模型名**判定，压缩请求豁免。
        // 判定放在本地压缩还原之后，估算的是真正要发出的历史，而不是被 base64
        // 摘要放大的中间形态。拒绝发生在出站准备阶段，上游零请求。
        let is_compaction = upstream_request.semantics().compact;
        if let InputGuardDecision::Reject { estimated_tokens } = decide_input_guard(
            self.input_guard,
            upstream_request.body(),
            upstream_model.as_str(),
            is_compaction,
        ) {
            return Err(failure::context_length_exceeded_error(
                estimated_tokens,
                self.input_guard.threshold_tokens,
            ));
        }
        // 出站前拒绝网关不支持的存储语义：`store=true` 会让上游保存响应，
        // `background=true` 请求上游异步执行，两者都与网关的按次转发不符。
        // 只检查显式 true；缺失值由编码阶段补齐为 false，`null` 与显式 false 照常放行。
        for field in ["store", "background"] {
            if upstream_request
                .body()
                .get(field)
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            {
                return Err(failure::unsupported_storage_error(field));
            }
        }
        if let Some(location) = lease
            .account()
            .request_location()
            .or(context.request_location())
        {
            align_structured_location_fields(
                upstream_request.body_mut(),
                chrono::Utc::now(),
                location,
            );
        }
        let requirement = transport_requirement(&upstream_request);
        let http_only = lease.transport() == crate::credential::ResponsesTransport::Http;
        if http_only && requirement.requires_websocket() {
            return Err(provider_error(
                ProviderErrorKind::Unsupported,
                UpstreamSendState::NotSent,
            ));
        }
        let requested_transport = if http_only {
            CodexProviderTransport::HttpOnly
        } else {
            selected_transport(&upstream_request)
        };
        let session_http_fallback = requirement.allows_pre_send_http_fallback()
            && session_transport_key
                .as_ref()
                .is_some_and(|key| self.session_transport_recovery.uses_http(key));
        let mut transport = if requirement.requires_websocket() {
            CodexProviderTransport::PreferWebSocket
        } else if context.transport() == AttemptTransport::Fallback || session_http_fallback {
            CodexProviderTransport::HttpOnly
        } else {
            requested_transport
        };
        if codex_oauth
            && transport == CodexProviderTransport::PreferWebSocket
            && (matches!(
                requirement,
                TransportRequirement::NewChain | TransportRequirement::WebSocketNewChain
            ) || (requirement == TransportRequirement::ExactWebSocketContinuation
                && upstream_request
                    .downstream_websocket_connection_id
                    .is_some()))
        {
            let payload_bytes =
                websocket_response_create_payload_len(&upstream_request).map_err(|_| {
                    provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent)
                })?;
            if payload_bytes >= WEBSOCKET_HTTP_FALLBACK_THRESHOLD_BYTES {
                if requirement == TransportRequirement::ExactWebSocketContinuation {
                    tracing::info!(
                        request_id = context.request_id().as_str(),
                        payload_bytes,
                        threshold_bytes = WEBSOCKET_HTTP_FALLBACK_THRESHOLD_BYTES,
                        continuation_recovery_action = "client_replay_required",
                        "OpenAI oversized WebSocket continuation requires full client input before send"
                    );
                    return Err(continuation_replay_required_error(
                        "transport_payload_too_large",
                    ));
                }
                transport = CodexProviderTransport::HttpOnly;
                tracing::info!(
                    request_id = context.request_id().as_str(),
                    payload_bytes,
                    threshold_bytes = WEBSOCKET_HTTP_FALLBACK_THRESHOLD_BYTES,
                    "OpenAI oversized new-chain request selected HTTP before WebSocket send"
                );
            }
        }
        if transport == CodexProviderTransport::PreferWebSocket
            && middleware_headers.iter().any(|header| {
                match HeaderValue::from_bytes(header.value()) {
                    Ok(value) => value.to_str().is_err(),
                    Err(_) => true,
                }
            })
        {
            return Err(provider_error(
                ProviderErrorKind::Protocol,
                UpstreamSendState::NotSent,
            ));
        }
        apply_transport(&mut upstream_request, transport);
        let metadata = ProviderCallMetadata::new(
            provider_kind,
            upstream_model.clone(),
            lease.account_id().clone(),
            UpstreamTransport::new(transport_name(transport)).map_err(|_| {
                provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent)
            })?,
        )
        .with_selection_observation(ProviderSelectionObservation::new(
            account_selection_wait_ms,
            lease.capacity_snapshot(),
        ));
        let response_store = upstream_request.store();
        let session_capture =
            (!continuation_requested || previous_session.is_some()).then(|| OpenAiSessionCapture {
                account_id: lease.account_id().as_str().to_owned(),
                credential_revision: matches!(
                    lease.authentication(),
                    crate::credential::CodexRuntimeAuthentication::ApiKey(_)
                )
                .then_some(lease.account().revision().get()),
                conversation_id: upstream_request.local_conversation_id.clone(),
                turn_state: upstream_request.turn_state.clone(),
                client_turn_id: upstream_request.client_turn_id.clone(),
                response_store,
                continuation_scope: None,
            });
        let allows_account_state_mutation = lease.allows_account_state_mutation();
        let session_transport_key_hash = session_transport_key
            .as_ref()
            .map(|key| key.expose_to_store().chars().take(12).collect());
        let websocket_retry_count = match context.transport() {
            AttemptTransport::Retry(retry_index) => retry_index.get(),
            AttemptTransport::Default | AttemptTransport::Fallback => 0,
        };
        let events = cold_response_stream(ColdResponse {
            client: self
                .client_for_request(&context)?
                .for_account(lease.account())
                .map_err(|error| map_client_error(error, UpstreamSendState::NotSent, false).error)?
                .with_responses_api_base_url(lease.authentication().responses_api_base_url())
                .with_connection_budget(context.connection_budget().clone())
                .with_response_control(context.response_control().cloned())
                .with_middleware_headers(middleware_headers),
            response_origin: self.responses_url.clone(),
            request: upstream_request,
            upstream_model,
            transport_policy: transport,
            context,
            selector: Arc::clone(&self.selector),
            quota: Arc::clone(&self.quota),
            catalog: Arc::clone(&self.catalog),
            lease: Arc::clone(&lease),
            session_transport_key,
            session_transport_key_hash,
            session_transport_recovery: self.session_transport_recovery.clone(),
            websocket_retry_count,
            stream_max_retries: self.stream_max_retries,
            session_capture,
        });
        let stream = ProviderStream::new(metadata, events, lease);
        Ok(if allows_account_state_mutation {
            stream.with_filtered_account_feedback(
                Arc::clone(&self.account_feedback),
                openai_failure_affects_account_score,
            )
        } else {
            stream
        })
    }
}

fn native_request_requirements(request: &GenerateRequest) -> CapabilityRequirements {
    // 此处只解释已知 OpenAI wire；不在 Core 的通用转换路径推断任意目标协议
    Operation::Generate(GenerateRequest::from_protocol_payload(
        request.protocol_payload().clone(),
    ))
    .capability_requirements()
}

/// 审核子代理请求要改写成的模型与推理强度；非审核请求返回 `None`。
///
/// `available_models` 取目录 cache：目录由 Core 在本请求路由前刷新，缺失说明目录还
/// 没就绪，此时宁可不改写，也不猜一个上游可能不存在的审核模型。判族的模型名取客户
/// 端在正文里声明的那个，而不是路由结果，因为"其它系列保持客户端选择"是按声明判的。
fn review_request_override(
    catalog: &CodexCredentialCatalogService,
    operation: &Operation,
) -> Option<ReviewOverride> {
    let Operation::Generate(generate) = operation else {
        return None;
    };
    let payload = generate.protocol_payload();
    if payload.protocol() != PROVIDER_NAME {
        return None;
    }
    if !review_override::is_review_request(payload.body(), payload.context()) {
        return None;
    }
    let declared = payload.body().get("model").and_then(Value::as_str)?;
    let available = catalog
        .cached()
        .ok()
        .flatten()?
        .models()
        .iter()
        .map(|model| model.request_model().as_str().to_owned())
        .collect::<Vec<_>>();
    review_override::review_override(declared, &available)
}

/// 把审核覆盖写回操作正文，让中间件、上游 wire 与观测看到同一个模型名与推理强度。
fn apply_review_override(
    operation: Operation,
    review: &ReviewOverride,
) -> Result<Operation, ProviderError> {
    let Operation::Generate(generate) = &operation else {
        return Ok(operation);
    };
    let mut body = generate.protocol_payload().body().clone();
    body.insert(
        "model".to_owned(),
        Value::String(review.upstream_model.clone()),
    );
    // 审核请求必须带上覆盖后的强度；`reasoning` 形状不合规时整体替换，不保留客户端的非法值。
    let reasoning = body
        .entry("reasoning".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    if !reasoning.is_object() {
        *reasoning = Value::Object(Map::new());
    }
    if let Some(reasoning) = reasoning.as_object_mut() {
        reasoning.insert(
            "effort".to_owned(),
            Value::String(review.reasoning_effort.to_owned()),
        );
    }
    let encoded = serde_json::to_vec(&body).map_err(|_| {
        provider_error(
            ProviderErrorKind::InvalidRequest,
            UpstreamSendState::NotSent,
        )
    })?;
    operation
        .replace_middleware_wire(PROVIDER_NAME, encoded.into())
        .map_err(|_| {
            provider_error(
                ProviderErrorKind::InvalidRequest,
                UpstreamSendState::NotSent,
            )
        })
}

fn validate_openai_reasoning(body: &Map<String, Value>) -> Result<(), ProviderError> {
    let Some(effort) = body
        .get("reasoning")
        .and_then(Value::as_object)
        .and_then(|reasoning| reasoning.get("effort"))
    else {
        return Ok(());
    };
    let Some(effort) = effort.as_str() else {
        return Err(provider_error(
            ProviderErrorKind::InvalidRequest,
            UpstreamSendState::NotSent,
        ));
    };
    if effort.is_empty() || effort.len() > 64 || effort.chars().any(char::is_control) {
        return Err(provider_error(
            ProviderErrorKind::InvalidRequest,
            UpstreamSendState::NotSent,
        ));
    }
    Ok(())
}
