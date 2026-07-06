use crate::client_common::Prompt;
use crate::client_common::ResponseStream;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_api::ResponseEvent;
use codex_login::default_client::build_reqwest_client;
use codex_protocol::error::CodexErr;
use codex_protocol::error::ConnectionFailedError;
use codex_protocol::error::Result;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ReasoningItemReasoningSummary;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ModelInfo;
use codex_tools::ToolSpec;
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MINC_SYSTEM_PROMPT_HEADER: &str = r#"You are Minc Agent, a CLI coding assistant.

When local tool execution is needed, you must output exactly one directive block and nothing else.
Otherwise, output only plain user-facing text.

Directive grammar:
1. Function tool call:
<minc:function name="tool_name">{"json":"arguments"}</minc:function>
2. Namespaced function tool call:
<minc:function namespace="tool_namespace" name="tool_name">{"json":"arguments"}</minc:function>
3. Freeform custom tool call:
<minc:custom name="tool_name">raw tool input</minc:custom>
4. Namespaced freeform custom tool call:
<minc:custom namespace="tool_namespace" name="tool_name">raw tool input</minc:custom>

Rules:
- Never mix prose with a directive block.
- Never emit markdown fences around a directive block.
- Use `exec_command` for shell execution and `apply_patch` for patch application.
- For `exec_command`, the JSON argument key is `cmd`, not `command`.
- For repository questions, code tasks, debugging, summaries, or implementation requests, inspect the repo and relevant files first unless the user explicitly asks for a no-tools answer.
- Most of the time, if the user asks about the codebase, you should use local tools to read the repo before answering.
- Only call tools that appear in the advertised tool list.
- Function arguments must be valid JSON objects.
- When tool results are present, continue working on the latest user request using those results instead of restarting the conversation.
- If you are unsure a tool call is needed, answer in plain text instead."#;
const MAX_MINC_QUERY_CHARS: usize = 1_800;
const MAX_MINC_SYSTEM_CHARS: usize = 1_800;
const MAX_MINC_BASE_INSTRUCTIONS_CHARS: usize = 1_000;
const MAX_MINC_EXECUTION_STATE_CHARS: usize = 700;
const MAX_MINC_URL_CHARS: usize = 5_500;
const MIN_MINC_QUERY_CHARS: usize = 300;
const MIN_MINC_SYSTEM_CHARS: usize = 500;

pub(crate) async fn stream_minc_api(
    base_url: &str,
    prompt: &Prompt,
    model_info: &ModelInfo,
) -> Result<ResponseStream> {
    let raw_query = render_prompt_transcript(prompt);
    let raw_system = build_system_prompt(prompt, model_info);
    let (query, system) = fit_request_parts(
        base_url,
        model_info.slug.as_str(),
        raw_query.as_str(),
        raw_system.as_str(),
    );
    let response = fetch_minc_response(base_url, &query, model_info.slug.as_str(), &system).await?;
    response.into_response_stream(prompt.tools.as_slice())
}

async fn fetch_minc_response(
    base_url: &str,
    query: &str,
    model: &str,
    system: &str,
) -> Result<MincAskResponse> {
    let mut url = Url::parse(base_url)
        .map_err(|err| CodexErr::Fatal(format!("invalid Minc base_url: {err}")))?;
    url.set_path("/api/v1/ask");
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("model", model)
        .append_pair("format", "json")
        .append_pair("system", system);

    let response = build_reqwest_client()
        .get(url)
        .send()
        .await
        .map_err(|source| CodexErr::ConnectionFailed(ConnectionFailedError { source }))?
        .error_for_status()
        .map_err(|source| CodexErr::ConnectionFailed(ConnectionFailedError { source }))?;
    let payload: MincAskResponse = response
        .json()
        .await
        .map_err(|source| CodexErr::ConnectionFailed(ConnectionFailedError { source }))?;
    if !payload.success {
        return Err(CodexErr::InvalidRequest(
            "Minc API returned success: false".to_string(),
        ));
    }
    if payload.answer.trim().is_empty() {
        return Err(CodexErr::InvalidRequest(
            "Minc API response did not include an answer".to_string(),
        ));
    }
    Ok(payload)
}

fn build_system_prompt(prompt: &Prompt, model_info: &ModelInfo) -> String {
    let mut sections = vec![model_info.get_model_instructions(None)];
    let base_instructions = sanitize_base_instructions(prompt.base_instructions.text.as_str());
    if !base_instructions.trim().is_empty() {
        sections.push(truncate_from_end(
            base_instructions.as_str(),
            MAX_MINC_BASE_INSTRUCTIONS_CHARS,
        ));
    }
    if let Some(user_request) = latest_user_request(prompt) {
        sections.push(format!(
            "Active user request:\n{}",
            truncate_from_end(user_request.as_str(), 400)
        ));
    }
    if let Some(execution_state) = render_execution_state(prompt) {
        sections.push(format!("Current execution state:\n{execution_state}"));
    }
    sections.push(MINC_SYSTEM_PROMPT_HEADER.to_string());
    sections.push(
        "If a tool was already run for this request, continue from that tool result instead of restarting or asking what to do next. Either answer the active user request using the available results, or emit one next valid directive."
            .to_string(),
    );
    sections.push(format!(
        "Advertised local tools:\n{}",
        render_tool_catalog(prompt.tools.as_slice())
    ));
    sections.join("\n\n")
}

fn latest_user_request(prompt: &Prompt) -> Option<String> {
    prompt.input.iter().rev().find_map(|item| match item {
        ResponseItem::Message { role, content, .. } if role == "user" => {
            let text = content
                .iter()
                .filter_map(|part| match part {
                    ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                        Some(text.as_str())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");
            (!text.trim().is_empty()).then_some(text)
        }
        _ => None,
    })
}

fn render_execution_state(prompt: &Prompt) -> Option<String> {
    let entries = prompt
        .input
        .iter()
        .filter_map(render_execution_state_item)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return None;
    }

    let recent = entries
        .into_iter()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Some(truncate_from_start(
        recent.as_str(),
        MAX_MINC_EXECUTION_STATE_CHARS,
    ))
}

fn render_execution_state_item(item: &ResponseItem) -> Option<String> {
    match item {
        ResponseItem::FunctionCall {
            namespace,
            name,
            arguments,
            ..
        } => Some(format!(
            "- Tool call: {}{} {}",
            namespace
                .as_deref()
                .map(|ns| format!("{ns}."))
                .unwrap_or_default(),
            name,
            truncate_from_end(arguments, 140)
        )),
        ResponseItem::CustomToolCall {
            namespace,
            name,
            input,
            ..
        } => Some(format!(
            "- Tool call: {}{} {}",
            namespace
                .as_deref()
                .map(|ns| format!("{ns}."))
                .unwrap_or_default(),
            name,
            truncate_from_end(input, 140)
        )),
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        } => output.body.to_text().map(|text| {
            format!(
                "- Tool result [{call_id}]: {}",
                truncate_from_end(text.as_str(), 180)
            )
        }),
        ResponseItem::CustomToolCallOutput {
            call_id,
            name,
            output,
            ..
        } => output.body.to_text().map(|text| {
            format!(
                "- Tool result [{}{}]: {}",
                name.as_deref().map(|n| format!("{n}:")).unwrap_or_default(),
                call_id,
                truncate_from_end(text.as_str(), 180)
            )
        }),
        _ => None,
    }
}

fn fit_request_parts(base_url: &str, model: &str, query: &str, system: &str) -> (String, String) {
    let mut query_limit = query.chars().count().min(MAX_MINC_QUERY_CHARS);
    let mut system_limit = system.chars().count().min(MAX_MINC_SYSTEM_CHARS);

    loop {
        let query_text = truncate_from_start(query, query_limit);
        let system_text = truncate_from_end(system, system_limit);
        let Some(url_len) =
            minc_request_url_length(base_url, query_text.as_str(), model, system_text.as_str())
        else {
            return (query_text, system_text);
        };

        if url_len <= MAX_MINC_URL_CHARS
            || (query_limit <= MIN_MINC_QUERY_CHARS && system_limit <= MIN_MINC_SYSTEM_CHARS)
        {
            return (query_text, system_text);
        }

        if system_limit >= query_limit && system_limit > MIN_MINC_SYSTEM_CHARS {
            system_limit = (system_limit * 4 / 5).max(MIN_MINC_SYSTEM_CHARS);
        } else if query_limit > MIN_MINC_QUERY_CHARS {
            query_limit = (query_limit * 4 / 5).max(MIN_MINC_QUERY_CHARS);
        } else {
            return (query_text, system_text);
        }
    }
}

fn minc_request_url_length(
    base_url: &str,
    query: &str,
    model: &str,
    system: &str,
) -> Option<usize> {
    let mut url = Url::parse(base_url).ok()?;
    url.set_path("/api/v1/ask");
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("model", model)
        .append_pair("format", "json")
        .append_pair("system", system);
    Some(url.as_str().len())
}

fn render_tool_catalog(tools: &[ToolSpec]) -> String {
    if tools.is_empty() {
        return "<none>".to_string();
    }

    tools
        .iter()
        .map(ToolSpec::name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_prompt_transcript(prompt: &Prompt) -> String {
    let entries = prompt
        .input
        .iter()
        .filter_map(render_response_item_for_minc)
        .collect::<Vec<_>>();
    let original_user_request = prompt
        .input
        .iter()
        .find_map(extract_user_message_text)
        .unwrap_or_default();
    let latest_user_request = prompt
        .input
        .iter()
        .rev()
        .find_map(extract_user_message_text)
        .unwrap_or_default();

    let mut sections = Vec::new();
    if !original_user_request.is_empty() {
        sections.push(format!("original_user_request: {original_user_request}"));
    }
    if !latest_user_request.is_empty() && latest_user_request != original_user_request {
        sections.push(format!("latest_user_request: {latest_user_request}"));
    }
    if !entries.is_empty() {
        sections.push(format!("turn_log:\n{}", entries.join("\n")));
    }

    sections.join("\n\n")
}

fn render_response_item_for_minc(item: &ResponseItem) -> Option<String> {
    match item {
        ResponseItem::Message { role, content, .. } => {
            let text = extract_message_text(content)?;
            Some(match role.as_str() {
                "user" => format!("user_request: {text}"),
                "assistant" => format!("assistant_response: {text}"),
                other => format!("{other}_message: {text}"),
            })
        }
        ResponseItem::FunctionCall {
            namespace,
            name,
            arguments,
            ..
        } => Some(format!(
            "assistant_action: call {}{} with {}",
            namespace
                .as_deref()
                .map(|ns| format!("{ns}."))
                .unwrap_or_default(),
            name,
            arguments
        )),
        ResponseItem::CustomToolCall {
            namespace,
            name,
            input,
            ..
        } => Some(format!(
            "assistant_action: call {}{} with {}",
            namespace
                .as_deref()
                .map(|ns| format!("{ns}."))
                .unwrap_or_default(),
            name,
            input
        )),
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        } => output
            .body
            .to_text()
            .map(|text| format!("tool_result: call_id={call_id}; output={text}")),
        ResponseItem::CustomToolCallOutput {
            call_id,
            name,
            output,
            ..
        } => output.body.to_text().map(|text| {
            format!(
                "tool_result: tool={}{}; output={text}",
                name.as_deref().map(|n| format!("{n}:")).unwrap_or_default(),
                call_id
            )
        }),
        _ => None,
    }
}

fn extract_message_text(content: &[ContentItem]) -> Option<String> {
    let text = content
        .iter()
        .filter_map(|part| match part {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    let text = text.trim();
    (!text.is_empty()).then_some(text.to_string())
}

fn extract_user_message_text(item: &ResponseItem) -> Option<String> {
    let ResponseItem::Message { role, content, .. } = item else {
        return None;
    };
    (role == "user")
        .then(|| extract_message_text(content))
        .flatten()
}

#[derive(Debug, Deserialize)]
struct MincAskResponse {
    answer: String,
    thinking: Option<String>,
    model: Option<String>,
    #[serde(rename = "latencyMs")]
    latency_ms: Option<i64>,
    success: bool,
}

impl MincAskResponse {
    fn into_response_stream(self, tools: &[ToolSpec]) -> Result<ResponseStream> {
        let (tx_event, rx_event) = mpsc::channel(32);
        let consumer_dropped = CancellationToken::new();
        let consumer_dropped_for_task = consumer_dropped.clone();
        let response_id = format!("minc-{}", now_unix_timestamp_ms());
        let parsed_directive = parse_minc_directive(self.answer.trim(), tools)?;
        let answer = self.answer;
        let thinking = self.thinking.filter(|text| !text.trim().is_empty());
        let model = self.model;
        let _latency_ms = self.latency_ms;

        tokio::spawn(async move {
            let send = |event| async {
                if consumer_dropped_for_task.is_cancelled() {
                    return false;
                }
                tx_event.send(Ok(event)).await.is_ok()
            };

            if !send(ResponseEvent::Created).await {
                return;
            }
            if let Some(model) = model
                && !send(ResponseEvent::ServerModel(model)).await
            {
                return;
            }
            if let Some(thinking) = thinking {
                let reasoning_item = ResponseItem::Reasoning {
                    id: Some("minc-reasoning".to_string()),
                    summary: vec![ReasoningItemReasoningSummary::SummaryText {
                        text: "Minc thinking".to_string(),
                    }],
                    content: Some(vec![ReasoningItemContent::ReasoningText {
                        text: thinking.clone(),
                    }]),
                    encrypted_content: None,
                    internal_chat_message_metadata_passthrough: None,
                };
                if !send(ResponseEvent::OutputItemAdded(reasoning_item.clone())).await {
                    return;
                }
                if !send(ResponseEvent::OutputItemDone(reasoning_item)).await {
                    return;
                }
            }

            let final_item = match parsed_directive {
                Some(directive) => directive.into_response_item(),
                None => ResponseItem::Message {
                    id: Some("minc-message".to_string()),
                    role: "assistant".to_string(),
                    content: vec![ContentItem::OutputText {
                        text: answer.clone(),
                    }],
                    phase: None,
                    internal_chat_message_metadata_passthrough: None,
                },
            };

            if !send(ResponseEvent::OutputItemAdded(final_item.clone())).await {
                return;
            }
            if !send(ResponseEvent::OutputItemDone(final_item)).await {
                return;
            }
            let _ = send(ResponseEvent::Completed {
                response_id,
                token_usage: None,
                end_turn: Some(true),
            })
            .await;
        });

        Ok(ResponseStream {
            rx_event,
            consumer_dropped,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum MincDirective {
    Function {
        namespace: Option<String>,
        name: String,
        arguments: String,
    },
    Custom {
        namespace: Option<String>,
        name: String,
        input: String,
    },
}

impl MincDirective {
    fn into_response_item(self) -> ResponseItem {
        let call_id = format!("minc-call-{}", now_unix_timestamp_ms());
        match self {
            Self::Function {
                namespace,
                name,
                arguments,
            } => ResponseItem::FunctionCall {
                id: Some(call_id.clone()),
                name,
                namespace,
                arguments,
                call_id,
                internal_chat_message_metadata_passthrough: None,
            },
            Self::Custom {
                namespace,
                name,
                input,
            } => ResponseItem::CustomToolCall {
                id: Some(call_id.clone()),
                status: Some("completed".to_string()),
                call_id,
                name,
                namespace,
                input,
                internal_chat_message_metadata_passthrough: None,
            },
        }
    }
}

fn parse_minc_directive(answer: &str, tools: &[ToolSpec]) -> Result<Option<MincDirective>> {
    let answer = extract_directive_candidate(answer).unwrap_or(answer).trim();
    if answer.is_empty() {
        return Ok(None);
    }
    if !answer.starts_with("<minc:") {
        return Ok(None);
    }

    if let Some((attrs, body)) = strip_tag(answer, "minc:function") {
        let Some(name) = parse_required_attr(attrs, "name") else {
            return Ok(None);
        };
        let namespace = parse_optional_attr(attrs, "namespace");
        let Ok(arguments) = serde_json::from_str::<Value>(body) else {
            return Ok(None);
        };
        if !arguments.is_object() {
            return Ok(None);
        }
        let Some(arguments) = normalize_function_arguments(name.as_str(), arguments) else {
            return Ok(None);
        };
        let directive = MincDirective::Function {
            namespace,
            name,
            arguments,
        };
        return Ok(Some(coerce_directive_to_tool_spec(directive, tools)?));
    }

    if let Some((attrs, body)) = strip_tag(answer, "minc:custom") {
        let Some(name) = parse_required_attr(attrs, "name") else {
            return Ok(None);
        };
        let namespace = parse_optional_attr(attrs, "namespace");
        let directive = MincDirective::Custom {
            namespace,
            name,
            input: body.to_string(),
        };
        return Ok(Some(coerce_directive_to_tool_spec(directive, tools)?));
    }

    Ok(None)
}

fn coerce_directive_to_tool_spec(
    directive: MincDirective,
    tools: &[ToolSpec],
) -> Result<MincDirective> {
    let kind = advertised_tool_kind(
        tools,
        directive.namespace().as_deref(),
        directive.name().as_str(),
    );

    match (directive, kind) {
        (
            MincDirective::Custom {
                namespace,
                name,
                input,
            },
            Some(AdvertisedToolKind::Function),
        ) => {
            let arguments = custom_input_to_function_arguments(name.as_str(), input.as_str())?;
            Ok(MincDirective::Function {
                namespace,
                name,
                arguments,
            })
        }
        (
            MincDirective::Function {
                namespace,
                name,
                arguments,
            },
            Some(AdvertisedToolKind::Freeform),
        ) => {
            let input = function_arguments_to_custom_input(arguments.as_str())?;
            Ok(MincDirective::Custom {
                namespace,
                name,
                input,
            })
        }
        (directive, _) => Ok(directive),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdvertisedToolKind {
    Function,
    Freeform,
}

impl MincDirective {
    fn namespace(&self) -> Option<String> {
        match self {
            Self::Function { namespace, .. } | Self::Custom { namespace, .. } => namespace.clone(),
        }
    }

    fn name(&self) -> String {
        match self {
            Self::Function { name, .. } | Self::Custom { name, .. } => name.clone(),
        }
    }
}

fn advertised_tool_kind(
    tools: &[ToolSpec],
    namespace: Option<&str>,
    name: &str,
) -> Option<AdvertisedToolKind> {
    tools.iter().find_map(|tool| match tool {
        ToolSpec::Function(tool) if namespace.is_none() && tool.name == name => {
            Some(AdvertisedToolKind::Function)
        }
        ToolSpec::Freeform(tool) if namespace.is_none() && tool.name == name => {
            Some(AdvertisedToolKind::Freeform)
        }
        ToolSpec::Namespace(tool_namespace) if Some(tool_namespace.name.as_str()) == namespace => {
            tool_namespace.tools.iter().find_map(|tool| match tool {
                codex_tools::ResponsesApiNamespaceTool::Function(tool) if tool.name == name => {
                    Some(AdvertisedToolKind::Function)
                }
                _ => None,
            })
        }
        ToolSpec::ToolSearch { .. } if namespace.is_none() && name == "tool_search" => {
            Some(AdvertisedToolKind::Function)
        }
        ToolSpec::ImageGeneration { .. } if namespace.is_none() && name == "image_generation" => {
            Some(AdvertisedToolKind::Function)
        }
        ToolSpec::WebSearch { .. } if namespace.is_none() && name == "web_search" => {
            Some(AdvertisedToolKind::Function)
        }
        _ => None,
    })
}

fn custom_input_to_function_arguments(name: &str, input: &str) -> Result<String> {
    let trimmed = input.trim();
    if name == "exec_command" && !trimmed.starts_with('{') {
        return Ok(serde_json::json!({ "cmd": trimmed }).to_string());
    }

    let arguments: Value = serde_json::from_str(trimmed).map_err(|_| {
        CodexErr::InvalidRequest(format!(
            "Minc emitted a custom payload for function tool {name} that was not valid JSON"
        ))
    })?;
    normalize_function_arguments(name, arguments).ok_or_else(|| {
        CodexErr::InvalidRequest(format!(
            "Minc emitted invalid arguments for function tool {name}"
        ))
    })
}

fn function_arguments_to_custom_input(arguments: &str) -> Result<String> {
    let value: Value = serde_json::from_str(arguments).map_err(|_| {
        CodexErr::InvalidRequest(
            "Minc emitted JSON arguments for a freeform tool that could not be parsed".to_string(),
        )
    })?;
    match value {
        Value::Object(mut object) => match object.remove("input") {
            Some(Value::String(input)) => Ok(input),
            _ => Err(CodexErr::InvalidRequest(
                "Minc emitted JSON arguments for a freeform tool without an `input` string"
                    .to_string(),
            )),
        },
        _ => Err(CodexErr::InvalidRequest(
            "Minc emitted non-object JSON arguments for a freeform tool".to_string(),
        )),
    }
}

fn extract_directive_candidate(answer: &str) -> Option<&str> {
    let answer = answer.trim();
    if answer.starts_with("<minc:") {
        return Some(answer);
    }

    let function_tag = extract_tag_block(answer, "minc:function");
    let custom_tag = extract_tag_block(answer, "minc:custom");
    let candidate = match (function_tag, custom_tag) {
        (Some(_), Some(_)) => return None,
        (Some(tag), None) | (None, Some(tag)) => tag,
        (None, None) => return None,
    };

    let prefix = answer[..candidate.start].trim();
    let suffix = answer[candidate.end..].trim();
    if prefix.contains("<minc:") || suffix.contains("<minc:") {
        return None;
    }
    if prefix.contains("```") || suffix.contains("```") {
        return None;
    }
    if prefix.chars().count() + suffix.chars().count() > 240 {
        return None;
    }

    Some(&answer[candidate.start..candidate.end])
}

struct TagBlock {
    start: usize,
    end: usize,
}

fn extract_tag_block(source: &str, tag: &str) -> Option<TagBlock> {
    let open_prefix = format!("<{tag}");
    let close_tag = format!("</{tag}>");
    let start = source.find(&open_prefix)?;
    let after_start = &source[start..];
    let close_rel = after_start.find(&close_tag)?;
    let end = start + close_rel + close_tag.len();
    Some(TagBlock { start, end })
}

fn normalize_function_arguments(name: &str, mut arguments: Value) -> Option<String> {
    if name == "exec_command"
        && let Some(object) = arguments.as_object_mut()
        && !object.contains_key("cmd")
        && let Some(command) = object.remove("command")
    {
        object.insert("cmd".to_string(), command);
    }

    Some(serde_json::to_string(&arguments).ok()?)
}

fn strip_tag<'a>(source: &'a str, tag: &str) -> Option<(&'a str, &'a str)> {
    let open_prefix = format!("<{tag}");
    let close_tag = format!("</{tag}>");
    if !source.starts_with(&open_prefix) || !source.ends_with(&close_tag) {
        return None;
    }
    let after_prefix = &source[open_prefix.len()..];
    let open_end = after_prefix.find('>')?;
    let attrs = after_prefix[..open_end].trim();
    let body = &source[(open_prefix.len() + open_end + 1)..(source.len() - close_tag.len())];
    Some((attrs, body))
}

fn parse_required_attr(attrs: &str, key: &str) -> Option<String> {
    parse_optional_attr(attrs, key).filter(|value| !value.is_empty())
}

fn parse_optional_attr(attrs: &str, key: &str) -> Option<String> {
    let pattern = format!(r#"{key}=""#);
    let start = attrs.find(&pattern)?;
    let value_start = start + pattern.len();
    let value_rest = &attrs[value_start..];
    let value_end = value_rest.find('"')?;
    Some(value_rest[..value_end].to_string())
}

fn truncate_from_start(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let kept = text
        .chars()
        .rev()
        .take(max_chars.saturating_sub(31))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("[Earlier context truncated for Minc]\n{kept}")
}

fn truncate_from_end(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let kept = text
        .chars()
        .take(max_chars.saturating_sub(35))
        .collect::<String>();
    format!("{kept}\n[Remaining system prompt truncated]")
}

fn sanitize_base_instructions(text: &str) -> String {
    let mut sanitized = text.to_string();
    for tag in [
        "skills_instructions",
        "plugins_instructions",
        "apps_instructions",
        "permissions instructions",
        "environment_context",
    ] {
        sanitized = strip_tagged_section(&sanitized, tag);
    }

    let lines = sanitized
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("⚠")
                && !trimmed.contains("skills context budget")
                && !trimmed.contains("Skill descriptions were shortened")
        })
        .fold(Vec::new(), |mut acc: Vec<&str>, line| {
            let trimmed = line.trim();
            let is_blank = trimmed.is_empty();
            let previous_was_blank = acc
                .last()
                .map(|previous| previous.trim().is_empty())
                .unwrap_or(false);
            if !is_blank || !previous_was_blank {
                acc.push(line);
            }
            acc
        });

    let mut compact = lines.join("\n");
    while compact.contains("\n\n") {
        compact = compact.replace("\n\n", "\n");
    }

    compact.trim().to_string()
}

fn strip_tagged_section(text: &str, tag: &str) -> String {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    let mut remaining = text;
    let mut output = String::new();

    loop {
        let Some(start) = remaining.find(&open_tag) else {
            output.push_str(remaining);
            break;
        };
        output.push_str(&remaining[..start]);
        let after_open = &remaining[start + open_tag.len()..];
        let Some(end) = after_open.find(&close_tag) else {
            break;
        };
        remaining = &after_open[end + close_tag.len()..];
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputPayload;
    use codex_protocol::models::ResponseItem;
    use codex_tools::FreeformTool;
    use codex_tools::FreeformToolFormat;
    use codex_tools::JsonSchema;
    use codex_tools::ResponsesApiTool;
    use codex_tools::ToolSpec;
    use std::collections::BTreeMap;

    fn empty_tools() -> Vec<ToolSpec> {
        Vec::new()
    }

    #[test]
    fn parses_exec_command_directive() {
        let directive = parse_minc_directive(
            r#"<minc:function name="exec_command">{"cmd":"pwd"}</minc:function>"#,
            &empty_tools(),
        )
        .expect("directive should parse");

        assert_eq!(
            directive,
            Some(MincDirective::Function {
                namespace: None,
                name: "exec_command".to_string(),
                arguments: r#"{"cmd":"pwd"}"#.to_string(),
            })
        );
    }

    #[test]
    fn normalizes_exec_command_argument_alias() {
        let directive = parse_minc_directive(
            r#"<minc:function name="exec_command">{"command":"pwd"}</minc:function>"#,
            &empty_tools(),
        )
        .expect("directive should parse");

        assert_eq!(
            directive,
            Some(MincDirective::Function {
                namespace: None,
                name: "exec_command".to_string(),
                arguments: r#"{"cmd":"pwd"}"#.to_string(),
            })
        );
    }

    #[test]
    fn parses_apply_patch_directive() {
        let directive = parse_minc_directive(
            "<minc:custom name=\"apply_patch\">*** Begin Patch\n*** End Patch</minc:custom>",
            &empty_tools(),
        )
        .expect("directive should parse");

        assert_eq!(
            directive,
            Some(MincDirective::Custom {
                namespace: None,
                name: "apply_patch".to_string(),
                input: "*** Begin Patch\n*** End Patch".to_string(),
            })
        );
    }

    #[test]
    fn plain_text_is_not_treated_as_directive() {
        assert_eq!(
            parse_minc_directive("hello there", &empty_tools()).expect("parse should succeed"),
            None
        );
    }

    #[test]
    fn wrapped_single_directive_is_accepted() {
        let directive = parse_minc_directive(
            "I'll explore the repository structure and summarize it for you.\n<minc:function name=\"exec_command\">{\"command\":\"pwd\"}</minc:function>",
            &empty_tools(),
        )
        .expect("parse should succeed");

        assert_eq!(
            directive,
            Some(MincDirective::Function {
                namespace: None,
                name: "exec_command".to_string(),
                arguments: r#"{"cmd":"pwd"}"#.to_string(),
            })
        );
    }

    #[test]
    fn multiple_directives_are_rejected() {
        assert_eq!(
            parse_minc_directive(
                "<minc:function name=\"exec_command\">{\"cmd\":\"pwd\"}</minc:function>\n<minc:function name=\"exec_command\">{\"cmd\":\"ls\"}</minc:function>",
                &empty_tools(),
            )
            .expect("parse should succeed"),
            None
        );
    }

    #[test]
    fn custom_exec_command_is_coerced_to_function_payload() {
        let mut properties = BTreeMap::new();
        properties.insert("cmd".to_string(), JsonSchema::string(None));
        let tools = vec![ToolSpec::Function(ResponsesApiTool {
            name: "exec_command".to_string(),
            description: "Run a shell command".to_string(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::object(properties, Some(vec!["cmd".to_string()]), None),
            output_schema: None,
        })];

        let directive = parse_minc_directive(
            "<minc:custom name=\"exec_command\">ls -la</minc:custom>",
            &tools,
        )
        .expect("directive should parse");

        assert_eq!(
            directive,
            Some(MincDirective::Function {
                namespace: None,
                name: "exec_command".to_string(),
                arguments: r#"{"cmd":"ls -la"}"#.to_string(),
            })
        );
    }

    #[test]
    fn function_apply_patch_with_input_is_coerced_to_custom_payload() {
        let tools = vec![ToolSpec::Freeform(FreeformTool {
            name: "apply_patch".to_string(),
            description: "Apply a patch".to_string(),
            format: FreeformToolFormat {
                r#type: "grammar".to_string(),
                syntax: "lark".to_string(),
                definition: "patch".to_string(),
            },
        })];

        let directive = parse_minc_directive(
            r#"<minc:function name="apply_patch">{"input":"*** Begin Patch\n*** End Patch"}</minc:function>"#,
            &tools,
        )
        .expect("directive should parse");

        assert_eq!(
            directive,
            Some(MincDirective::Custom {
                namespace: None,
                name: "apply_patch".to_string(),
                input: "*** Begin Patch\n*** End Patch".to_string(),
            })
        );
    }

    #[test]
    fn transcript_includes_tool_outputs() {
        let rendered = render_response_item_for_minc(&ResponseItem::FunctionCallOutput {
            id: None,
            call_id: "call-1".to_string(),
            output: FunctionCallOutputPayload::from_text("ok".to_string()),
            internal_chat_message_metadata_passthrough: None,
        })
        .expect("tool output should render");

        assert_eq!(rendered, "tool_result: call_id=call-1; output=ok");
    }

    #[test]
    fn sanitize_base_instructions_removes_large_skills_sections() {
        let text = r#"keep me
<skills_instructions>
very large skills blob
</skills_instructions>
⚠ Skill descriptions were shortened to fit the 2% skills context budget.
also keep me"#;

        let sanitized = sanitize_base_instructions(text);

        assert_eq!(sanitized, "keep me\nalso keep me");
    }

    #[test]
    fn tool_catalog_renders_compact_names() {
        let tools = vec![
            ToolSpec::ImageGeneration {
                output_format: "png".to_string(),
            },
            ToolSpec::WebSearch {
                external_web_access: None,
                index_gated_web_access: None,
                filters: None,
                user_location: None,
                search_context_size: None,
                search_content_types: None,
            },
        ];

        assert_eq!(render_tool_catalog(&tools), "image_generation, web_search");
    }

    #[test]
    fn fit_request_parts_respects_url_budget() {
        let query = "user: hi\n".repeat(700);
        let system = "developer instructions\n".repeat(700);

        let (query, system) = fit_request_parts(
            "https://mincapi.space-z.ai",
            "Auto",
            query.as_str(),
            system.as_str(),
        );
        let url_len = minc_request_url_length(
            "https://mincapi.space-z.ai",
            query.as_str(),
            "Auto",
            system.as_str(),
        )
        .expect("url should build");

        assert!(url_len <= MAX_MINC_URL_CHARS);
    }

    #[test]
    fn transcript_carries_original_request_and_turn_log() {
        let mut prompt = Prompt::default();
        prompt.input = vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "Read and summarize the repo".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                namespace: None,
                arguments: r#"{"cmd":"ls"}"#.to_string(),
                call_id: "call-1".to_string(),
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: "call-1".to_string(),
                output: FunctionCallOutputPayload::from_text("Cargo.toml\nsrc".to_string()),
                internal_chat_message_metadata_passthrough: None,
            },
        ];

        let rendered = render_prompt_transcript(&prompt);

        assert!(rendered.contains("original_user_request: Read and summarize the repo"));
        assert!(rendered.contains("turn_log:"));
        assert!(rendered.contains(r#"assistant_action: call exec_command with {"cmd":"ls"}"#));
        assert!(rendered.contains("tool_result: call_id=call-1; output=Cargo.toml\nsrc"));
    }

    #[test]
    fn build_system_prompt_caps_base_instructions() {
        let mut prompt = Prompt::default();
        prompt.base_instructions.text = "x".repeat(MAX_MINC_BASE_INSTRUCTIONS_CHARS * 2);

        let model_info = ModelInfo {
            slug: "Auto".to_string(),
            display_name: "Auto".to_string(),
            description: None,
            default_reasoning_level: None,
            supported_reasoning_levels: Vec::new(),
            shell_type: codex_protocol::openai_models::ConfigShellToolType::ShellCommand,
            visibility: codex_protocol::openai_models::ModelVisibility::List,
            supported_in_api: true,
            priority: 0,
            additional_speed_tiers: Vec::new(),
            service_tiers: Vec::new(),
            default_service_tier: None,
            availability_nux: None,
            upgrade: None,
            base_instructions: "base".to_string(),
            model_messages: None,
            include_skills_usage_instructions: false,
            supports_reasoning_summaries: false,
            default_reasoning_summary: codex_protocol::config_types::ReasoningSummary::None,
            support_verbosity: false,
            default_verbosity: None,
            apply_patch_tool_type: None,
            web_search_tool_type: codex_protocol::openai_models::WebSearchToolType::Text,
            truncation_policy: codex_protocol::openai_models::TruncationPolicyConfig::bytes(1_000),
            supports_parallel_tool_calls: false,
            supports_image_detail_original: false,
            context_window: None,
            max_context_window: None,
            auto_compact_token_limit: None,
            comp_hash: None,
            effective_context_window_percent: 100,
            experimental_supported_tools: Vec::new(),
            input_modalities: vec![codex_protocol::openai_models::InputModality::Text],
            used_fallback_model_metadata: false,
            supports_search_tool: false,
            use_responses_lite: false,
            auto_review_model_override: None,
            tool_mode: None,
            multi_agent_version: None,
        };

        let prompt_text = build_system_prompt(&prompt, &model_info);

        assert!(prompt_text.chars().count() < 4_000);
        assert!(prompt_text.contains("[Remaining system prompt truncated]"));
    }

    #[test]
    fn build_system_prompt_includes_active_request_and_tool_state() {
        let mut prompt = Prompt::default();
        prompt.input = vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "Read and summarize the repo".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                namespace: None,
                arguments: r#"{"cmd":"ls"}"#.to_string(),
                call_id: "call-1".to_string(),
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: "call-1".to_string(),
                output: FunctionCallOutputPayload::from_text("Cargo.toml\nREADME.md".to_string()),
                internal_chat_message_metadata_passthrough: None,
            },
        ];

        let model_info = ModelInfo {
            slug: "Auto".to_string(),
            display_name: "Auto".to_string(),
            description: None,
            default_reasoning_level: None,
            supported_reasoning_levels: Vec::new(),
            shell_type: codex_protocol::openai_models::ConfigShellToolType::ShellCommand,
            visibility: codex_protocol::openai_models::ModelVisibility::List,
            supported_in_api: true,
            priority: 0,
            additional_speed_tiers: Vec::new(),
            service_tiers: Vec::new(),
            default_service_tier: None,
            availability_nux: None,
            upgrade: None,
            base_instructions: "base".to_string(),
            model_messages: None,
            include_skills_usage_instructions: false,
            supports_reasoning_summaries: false,
            default_reasoning_summary: codex_protocol::config_types::ReasoningSummary::None,
            support_verbosity: false,
            default_verbosity: None,
            apply_patch_tool_type: None,
            web_search_tool_type: codex_protocol::openai_models::WebSearchToolType::Text,
            truncation_policy: codex_protocol::openai_models::TruncationPolicyConfig::bytes(1_000),
            supports_parallel_tool_calls: false,
            supports_image_detail_original: false,
            context_window: None,
            max_context_window: None,
            auto_compact_token_limit: None,
            comp_hash: None,
            effective_context_window_percent: 100,
            experimental_supported_tools: Vec::new(),
            input_modalities: vec![codex_protocol::openai_models::InputModality::Text],
            used_fallback_model_metadata: false,
            supports_search_tool: false,
            use_responses_lite: false,
            auto_review_model_override: None,
            tool_mode: None,
            multi_agent_version: None,
        };

        let prompt_text = build_system_prompt(&prompt, &model_info);

        assert!(prompt_text.contains("Active user request:\nRead and summarize the repo"));
        assert!(prompt_text.contains(r#"Tool call: exec_command {"cmd":"ls"}"#));
        assert!(prompt_text.contains("Tool result [call-1]: Cargo.toml"));
    }
}
