use std::collections::VecDeque;
use std::sync::Mutex;

use bytes::Bytes;
use chrono::Utc;
use hyper::StatusCode;
use serde_json::{Map, Value as JsonValue, json};
use uuid::Uuid;

use localairouter_core::{LocalAIRouterError, Result};

const RESPONSE_STORE_CAPACITY: usize = 500;
const RESPONSE_CHAIN_LIMIT: usize = 64;

#[derive(Debug, Clone)]
struct StoredResponse {
    input: Vec<JsonValue>,
    output: Vec<JsonValue>,
    previous_response_id: Option<String>,
}

#[derive(Debug)]
pub struct ResponseStore {
    entries: Mutex<VecDeque<(String, StoredResponse)>>,
    capacity: usize,
}

impl ResponseStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            capacity: RESPONSE_STORE_CAPACITY,
        }
    }

    pub fn resolve_previous_response_id(&self, body: Bytes) -> Result<Bytes> {
        let mut request = serde_json::from_slice::<JsonValue>(&body).map_err(|error| {
            LocalAIRouterError::Validation(format!("invalid JSON payload: {error}"))
        })?;
        let Some(previous_response_id) = request
            .get("previous_response_id")
            .and_then(JsonValue::as_str)
            .map(str::to_owned)
        else {
            return Ok(body);
        };

        let chain_items = self.resolve_chain(&previous_response_id);
        if chain_items.is_empty() {
            return Ok(body);
        }

        let Some(object) = request.as_object_mut() else {
            return Ok(body);
        };
        let current_input = normalize_input_to_array(object.get("input"));
        object.insert(
            "input".into(),
            JsonValue::Array(chain_items.into_iter().chain(current_input).collect()),
        );
        object.remove("previous_response_id");
        serde_json::to_vec(&request)
            .map(Bytes::from)
            .map_err(LocalAIRouterError::from)
    }

    pub fn store_response(&self, original_request_body: &str, responses_body: &[u8]) -> Result<()> {
        let request = serde_json::from_str::<JsonValue>(original_request_body)?;
        let response = serde_json::from_slice::<JsonValue>(responses_body)?;
        let Some(response_id) = response
            .get("id")
            .and_then(JsonValue::as_str)
            .map(str::to_owned)
        else {
            return Ok(());
        };
        let input = normalize_input_to_array(request.get("input"));
        let output = response
            .get("output")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        let previous_response_id = request
            .get("previous_response_id")
            .and_then(JsonValue::as_str)
            .map(str::to_owned);
        self.insert(
            response_id,
            StoredResponse {
                input,
                output,
                previous_response_id,
            },
        );
        Ok(())
    }

    pub fn clear(&self) {
        self.entries.lock().expect("response store").clear();
    }

    fn insert(&self, response_id: String, response: StoredResponse) {
        let mut entries = self.entries.lock().expect("response store");
        if let Some(index) = entries.iter().position(|(id, _)| id == &response_id) {
            entries.remove(index);
        }
        entries.push_front((response_id, response));
        while entries.len() > self.capacity {
            entries.pop_back();
        }
    }

    fn get(&self, response_id: &str) -> Option<StoredResponse> {
        let mut entries = self.entries.lock().expect("response store");
        let index = entries.iter().position(|(id, _)| id == response_id)?;
        let (id, response) = entries.remove(index)?;
        let cloned = response.clone();
        entries.push_front((id, response));
        Some(cloned)
    }

    fn resolve_chain(&self, previous_response_id: &str) -> Vec<JsonValue> {
        let mut chain = Vec::new();
        let mut current = Some(previous_response_id.to_owned());
        let mut visited = Vec::<String>::new();

        while let Some(response_id) = current {
            if visited.iter().any(|id| id == &response_id) || visited.len() >= RESPONSE_CHAIN_LIMIT
            {
                break;
            }
            visited.push(response_id.clone());
            let Some(response) = self.get(&response_id) else {
                break;
            };
            current = response.previous_response_id.clone();
            chain.push(response);
        }

        chain.reverse();
        let mut items = Vec::new();
        for response in chain {
            items.extend(response.input);
            items.extend(response.output);
        }
        items
    }
}

impl Default for ResponseStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ConvertedRequest {
    pub upstream_path: String,
    pub query: Option<String>,
    pub body: Bytes,
    pub logged_body_text: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConvertedResponse {
    pub body: Bytes,
    pub logged_body_text: String,
}

pub fn convert_deepseek_v4_request(
    store: &ResponseStore,
    upstream_path: &str,
    body: Bytes,
) -> Result<Option<ConvertedRequest>> {
    let normalized_path = normalize_path(upstream_path);
    match normalized_path.as_str() {
        "/responses" | "/v1/responses" => {
            let body = store.resolve_previous_response_id(body)?;
            let request = responses_to_chat_completions(&body)?;
            Ok(Some(request))
        }
        "/chat/completions" | "/v1/chat/completions" => {
            let request = normalize_chat_completions(&body)?;
            Ok(Some(request))
        }
        _ => Ok(None),
    }
}

pub fn convert_deepseek_v4_response(
    original_upstream_path: &str,
    status: StatusCode,
    request_body_text: &str,
    response_body: Bytes,
) -> Result<ConvertedResponse> {
    let normalized_path = normalize_path(original_upstream_path);
    if matches!(normalized_path.as_str(), "/responses" | "/v1/responses") {
        if status.is_success() {
            let request = serde_json::from_str::<JsonValue>(request_body_text).ok();
            let response = serde_json::from_slice::<JsonValue>(&response_body)?;
            let converted = chat_completion_to_response(response, request.as_ref());
            let body = serde_json::to_vec(&converted)?;
            let logged_body_text =
                serde_json::to_string_pretty(&converted).unwrap_or_else(|_| String::new());
            return Ok(ConvertedResponse {
                body: Bytes::from(body),
                logged_body_text,
            });
        }

        let converted = upstream_error_to_openai_error(status, &response_body);
        let body = serde_json::to_vec(&converted)?;
        let logged_body_text =
            serde_json::to_string_pretty(&converted).unwrap_or_else(|_| String::new());
        return Ok(ConvertedResponse {
            body: Bytes::from(body),
            logged_body_text,
        });
    }

    let logged_body_text = String::from_utf8_lossy(&response_body).into_owned();
    Ok(ConvertedResponse {
        body: response_body,
        logged_body_text,
    })
}

pub fn is_openai_responses_path(upstream_path: &str) -> bool {
    matches!(
        normalize_path(upstream_path).as_str(),
        "/responses" | "/v1/responses"
    )
}

pub fn response_json_to_sse(body: &[u8]) -> Result<Bytes> {
    let response = serde_json::from_slice::<JsonValue>(body)?;
    let response_id = response
        .get("id")
        .and_then(JsonValue::as_str)
        .unwrap_or("resp_localairouter");
    let model = response.get("model").cloned().unwrap_or(JsonValue::Null);
    let previous_response_id = response
        .get("previous_response_id")
        .cloned()
        .unwrap_or(JsonValue::Null);
    let metadata = response
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let usage = response
        .get("usage")
        .cloned()
        .unwrap_or_else(|| translate_usage(None));
    let output = response
        .get("output")
        .cloned()
        .unwrap_or_else(|| JsonValue::Array(Vec::new()));

    let base_response = json!({
        "id": response_id,
        "object": "response",
        "created_at": response
            .get("created_at")
            .and_then(JsonValue::as_i64)
            .unwrap_or_else(|| Utc::now().timestamp()),
        "status": "in_progress",
        "model": model,
        "output": [],
        "previous_response_id": previous_response_id,
        "metadata": metadata,
        "usage": { "input_tokens": 0, "output_tokens": 0, "total_tokens": 0 },
    });

    let mut events = String::new();
    push_sse(
        &mut events,
        "response.created",
        json!({ "type": "response.created", "response": base_response }),
    );
    push_sse(
        &mut events,
        "response.in_progress",
        json!({ "type": "response.in_progress", "response": base_response }),
    );

    if let Some(items) = output.as_array() {
        for (output_index, item) in items.iter().enumerate() {
            let mut in_progress_item = item.clone();
            if let Some(object) = in_progress_item.as_object_mut() {
                object.insert("status".into(), JsonValue::String("in_progress".into()));
                if object.get("type").and_then(JsonValue::as_str) == Some("message") {
                    object.insert("content".into(), JsonValue::Array(Vec::new()));
                }
                if object.get("type").and_then(JsonValue::as_str) == Some("function_call") {
                    object.insert("arguments".into(), JsonValue::String(String::new()));
                }
            }
            push_sse(
                &mut events,
                "response.output_item.added",
                json!({
                    "type": "response.output_item.added",
                    "output_index": output_index,
                    "item": in_progress_item,
                }),
            );

            match item.get("type").and_then(JsonValue::as_str) {
                Some("message") => {
                    if let Some(content) = item.get("content").and_then(JsonValue::as_array) {
                        for (content_index, part) in content.iter().enumerate() {
                            if part.get("type").and_then(JsonValue::as_str) == Some("output_text") {
                                let text = part
                                    .get("text")
                                    .and_then(JsonValue::as_str)
                                    .unwrap_or_default();
                                let empty_part = json!({
                                    "type": "output_text",
                                    "text": "",
                                    "annotations": [],
                                });
                                push_sse(
                                    &mut events,
                                    "response.content_part.added",
                                    json!({
                                        "type": "response.content_part.added",
                                        "output_index": output_index,
                                        "content_index": content_index,
                                        "part": empty_part,
                                    }),
                                );
                                if !text.is_empty() {
                                    push_sse(
                                        &mut events,
                                        "response.output_text.delta",
                                        json!({
                                            "type": "response.output_text.delta",
                                            "output_index": output_index,
                                            "content_index": content_index,
                                            "delta": text,
                                        }),
                                    );
                                }
                                push_sse(
                                    &mut events,
                                    "response.output_text.done",
                                    json!({
                                        "type": "response.output_text.done",
                                        "output_index": output_index,
                                        "content_index": content_index,
                                        "text": text,
                                    }),
                                );
                                push_sse(
                                    &mut events,
                                    "response.content_part.done",
                                    json!({
                                        "type": "response.content_part.done",
                                        "output_index": output_index,
                                        "content_index": content_index,
                                        "part": part,
                                    }),
                                );
                            }
                        }
                    }
                }
                Some("function_call") => {
                    let call_id = item
                        .get("call_id")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("call_unknown");
                    let arguments = item
                        .get("arguments")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("{}");
                    if !arguments.is_empty() {
                        push_sse(
                            &mut events,
                            "response.function_call_arguments.delta",
                            json!({
                                "type": "response.function_call_arguments.delta",
                                "output_index": output_index,
                                "call_id": call_id,
                                "delta": arguments,
                            }),
                        );
                    }
                    push_sse(
                        &mut events,
                        "response.function_call_arguments.done",
                        json!({
                            "type": "response.function_call_arguments.done",
                            "output_index": output_index,
                            "call_id": call_id,
                            "arguments": arguments,
                        }),
                    );
                }
                _ => {}
            }

            push_sse(
                &mut events,
                "response.output_item.done",
                json!({
                    "type": "response.output_item.done",
                    "output_index": output_index,
                    "item": item,
                }),
            );
        }
    }

    let completed_response = json!({
        "id": response_id,
        "object": "response",
        "created_at": response
            .get("created_at")
            .and_then(JsonValue::as_i64)
            .unwrap_or_else(|| Utc::now().timestamp()),
        "status": response
            .get("status")
            .and_then(JsonValue::as_str)
            .unwrap_or("completed"),
        "model": response.get("model").cloned().unwrap_or(JsonValue::Null),
        "output": output,
        "previous_response_id": response
            .get("previous_response_id")
            .cloned()
            .unwrap_or(JsonValue::Null),
        "metadata": response
            .get("metadata")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "usage": usage,
        "incomplete_details": response
            .get("incomplete_details")
            .cloned()
            .unwrap_or(JsonValue::Null),
    });
    push_sse(
        &mut events,
        "response.completed",
        json!({ "type": "response.completed", "response": completed_response }),
    );
    Ok(Bytes::from(events))
}

fn responses_to_chat_completions(body: &[u8]) -> Result<ConvertedRequest> {
    let value = serde_json::from_slice::<JsonValue>(body).map_err(|error| {
        LocalAIRouterError::Validation(format!("invalid JSON payload: {error}"))
    })?;
    let object = value.as_object().ok_or_else(|| {
        LocalAIRouterError::Validation("Responses converter requires a JSON object body".into())
    })?;

    let model = object
        .get("model")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned);
    let mut output = Map::new();
    if let Some(model) = model.as_ref() {
        output.insert("model".into(), JsonValue::String(model.clone()));
    }
    let messages = responses_input_to_messages(object.get("instructions"), object.get("input"));
    let has_assistant_tool_calls = messages.iter().any(|message| {
        message.get("role").and_then(JsonValue::as_str) == Some("assistant")
            && message
                .get("tool_calls")
                .and_then(JsonValue::as_array)
                .map(|calls| !calls.is_empty())
                .unwrap_or(false)
    });
    output.insert("messages".into(), JsonValue::Array(messages));
    output.insert("stream".into(), JsonValue::Bool(false));

    copy_number(object, &mut output, "temperature");
    copy_number(object, &mut output, "top_p");
    copy_value(object, &mut output, "parallel_tool_calls");
    if let Some(max_tokens) = object.get("max_output_tokens").and_then(JsonValue::as_u64) {
        output.insert("max_tokens".into(), json!(max_tokens));
    }
    if let Some(tools) = response_tools_to_chat_tools(object.get("tools")) {
        output.insert("tools".into(), tools);
    }
    if let Some(tool_choice) = response_tool_choice_to_chat(object.get("tool_choice")) {
        output.insert("tool_choice".into(), tool_choice);
    }
    apply_effort_translation(&mut output, object.get("reasoning"));
    if has_assistant_tool_calls
        && output
            .get("thinking")
            .and_then(|value| value.get("type"))
            .and_then(JsonValue::as_str)
            != Some("disabled")
    {
        output.insert("thinking".into(), json!({ "type": "disabled" }));
        output.remove("reasoning_effort");
    }

    let request = JsonValue::Object(output);
    let body = serde_json::to_vec(&request)?;
    let logged_body_text = serde_json::to_string_pretty(&request).unwrap_or_default();
    Ok(ConvertedRequest {
        upstream_path: "/chat/completions".into(),
        query: None,
        body: Bytes::from(body),
        logged_body_text,
        model,
    })
}

fn normalize_chat_completions(body: &[u8]) -> Result<ConvertedRequest> {
    let mut value = serde_json::from_slice::<JsonValue>(body).map_err(|error| {
        LocalAIRouterError::Validation(format!("invalid JSON payload: {error}"))
    })?;
    let model = value
        .get("model")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned);
    if let Some(object) = value.as_object_mut() {
        if let Some(effort) = object
            .remove("reasoning")
            .and_then(|value| value.get("effort").cloned())
        {
            apply_effort_string(object, effort.as_str().unwrap_or_default());
        }
        let has_assistant_tool_calls = object
            .get("messages")
            .and_then(JsonValue::as_array)
            .map(|messages| {
                messages.iter().any(|message| {
                    message.get("role").and_then(JsonValue::as_str) == Some("assistant")
                        && message
                            .get("tool_calls")
                            .and_then(JsonValue::as_array)
                            .map(|calls| !calls.is_empty())
                            .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if has_assistant_tool_calls
            && object
                .get("thinking")
                .and_then(|value| value.get("type"))
                .and_then(JsonValue::as_str)
                != Some("disabled")
        {
            object.insert("thinking".into(), json!({ "type": "disabled" }));
            object.remove("reasoning_effort");
        }
    }
    let body = serde_json::to_vec(&value)?;
    let logged_body_text = serde_json::to_string_pretty(&value).unwrap_or_default();
    Ok(ConvertedRequest {
        upstream_path: "/chat/completions".into(),
        query: None,
        body: Bytes::from(body),
        logged_body_text,
        model,
    })
}

fn normalize_input_to_array(input: Option<&JsonValue>) -> Vec<JsonValue> {
    match input {
        Some(JsonValue::Array(items)) => items.clone(),
        Some(JsonValue::String(text)) => vec![json!({
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": text,
            }],
        })],
        _ => Vec::new(),
    }
}

fn responses_input_to_messages(
    instructions: Option<&JsonValue>,
    input: Option<&JsonValue>,
) -> Vec<JsonValue> {
    let mut messages = Vec::new();
    if let Some(instructions) = instructions.and_then(JsonValue::as_str) {
        if !instructions.trim().is_empty() {
            messages.push(json!({
                "role": "system",
                "content": instructions,
            }));
        }
    }

    match input {
        Some(JsonValue::String(text)) => messages.push(json!({
            "role": "user",
            "content": text,
        })),
        Some(JsonValue::Array(items)) => {
            let mut pending_tool_calls = Vec::<JsonValue>::new();
            for item in items {
                let Some(object) = item.as_object() else {
                    continue;
                };
                let item_type = object.get("type").and_then(JsonValue::as_str).or_else(|| {
                    object
                        .get("role")
                        .and_then(JsonValue::as_str)
                        .map(|_| "message")
                });
                match item_type {
                    Some("message") => {
                        flush_tool_calls(&mut messages, &mut pending_tool_calls);
                        let role = normalize_role(object.get("role").and_then(JsonValue::as_str));
                        messages.push(json!({
                            "role": role,
                            "content": response_content_to_chat(object.get("content")),
                        }));
                    }
                    Some("function_call") => {
                        pending_tool_calls.push(json!({
                            "id": object
                                .get("call_id")
                                .or_else(|| object.get("id"))
                                .and_then(JsonValue::as_str)
                                .unwrap_or("call_unknown"),
                            "type": "function",
                            "function": {
                                "name": object
                                    .get("name")
                                    .and_then(JsonValue::as_str)
                                    .unwrap_or("unknown"),
                                "arguments": stringify_json_value(object.get("arguments")),
                            }
                        }));
                    }
                    Some("function_call_output") => {
                        flush_tool_calls(&mut messages, &mut pending_tool_calls);
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": object
                                .get("call_id")
                                .and_then(JsonValue::as_str)
                                .unwrap_or("call_unknown"),
                            "content": stringify_json_value(object.get("output")),
                        }));
                    }
                    _ => {}
                }
            }
            flush_tool_calls(&mut messages, &mut pending_tool_calls);
        }
        _ => {}
    }

    let messages = normalize_chat_messages(messages);
    if messages.is_empty() {
        return vec![json!({
            "role": "user",
            "content": "",
        })];
    }
    messages
}

fn flush_tool_calls(messages: &mut Vec<JsonValue>, pending_tool_calls: &mut Vec<JsonValue>) {
    if pending_tool_calls.is_empty() {
        return;
    }
    messages.push(json!({
        "role": "assistant",
        "content": null,
        "tool_calls": std::mem::take(pending_tool_calls),
    }));
}

fn normalize_chat_messages(messages: Vec<JsonValue>) -> Vec<JsonValue> {
    let mut work = messages;
    let mut fixed = Vec::<JsonValue>::new();
    let mut index = 0;

    while index < work.len() {
        let message = std::mem::take(&mut work[index]);
        if message.is_null() {
            index += 1;
            continue;
        }

        if message_has_tool_calls(&message) {
            let call_ids = tool_call_ids(&message);
            fixed.push(message);
            for candidate in work.iter_mut().skip(index + 1) {
                if is_tool_response_for_any(candidate, &call_ids) {
                    fixed.push(std::mem::take(candidate));
                }
            }
        } else if message.get("role").and_then(JsonValue::as_str) == Some("tool") {
            if let Some(position) = fixed.iter().rposition(message_has_tool_calls) {
                let mut insert_at = position + 1;
                while insert_at < fixed.len()
                    && fixed[insert_at].get("role").and_then(JsonValue::as_str) == Some("tool")
                {
                    insert_at += 1;
                }
                fixed.insert(insert_at, message);
            }
        } else {
            fixed.push(message);
        }
        index += 1;
    }

    let mut merged = Vec::<JsonValue>::new();
    for message in fixed {
        let previous = merged.last_mut();
        if let Some(previous) = previous {
            if merge_user_or_text_assistant(previous, &message) {
                continue;
            }
            if previous.get("role").and_then(JsonValue::as_str) == Some("assistant")
                && message.get("role").and_then(JsonValue::as_str) == Some("assistant")
                && !message_has_tool_calls(previous)
                && message_has_tool_calls(&message)
            {
                *previous = message;
                continue;
            }
            if previous.get("role").and_then(JsonValue::as_str) == Some("assistant")
                && message.get("role").and_then(JsonValue::as_str) == Some("assistant")
                && message_has_tool_calls(previous)
                && !message_has_tool_calls(&message)
            {
                continue;
            }
        }
        merged.push(message);
    }

    let mut validated = Vec::<JsonValue>::new();
    for message in merged {
        if message.get("role").and_then(JsonValue::as_str) == Some("tool") {
            let Some(previous) = validated.last() else {
                continue;
            };
            if previous.get("role").and_then(JsonValue::as_str) == Some("tool")
                || message_has_tool_calls(previous)
            {
                validated.push(message);
            }
        } else {
            validated.push(message);
        }
    }

    merge_adjacent_text_messages(drop_unanswered_tool_calls(validated))
}

fn merge_user_or_text_assistant(previous: &mut JsonValue, message: &JsonValue) -> bool {
    let previous_role = previous.get("role").and_then(JsonValue::as_str);
    let message_role = message.get("role").and_then(JsonValue::as_str);
    if previous_role != message_role || !matches!(previous_role, Some("user" | "assistant")) {
        return false;
    }
    if previous_role == Some("assistant")
        && (message_has_tool_calls(previous) || message_has_tool_calls(message))
    {
        return false;
    }
    let Some(previous_content) = previous
        .get("content")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
    else {
        return false;
    };
    let Some(message_content) = message.get("content").and_then(JsonValue::as_str) else {
        return false;
    };
    let merged = format!("{previous_content}\n\n{message_content}");
    if let Some(object) = previous.as_object_mut() {
        object.insert("content".into(), JsonValue::String(merged));
        true
    } else {
        false
    }
}

fn drop_unanswered_tool_calls(messages: Vec<JsonValue>) -> Vec<JsonValue> {
    let mut filtered = Vec::<JsonValue>::new();
    let mut index = 0;
    while index < messages.len() {
        let message = &messages[index];
        if !message_has_tool_calls(message) {
            filtered.push(message.clone());
            index += 1;
            continue;
        }

        let call_ids = tool_call_ids(message);
        let mut answered = Vec::<String>::new();
        let mut cursor = index + 1;
        while cursor < messages.len() {
            let candidate = &messages[cursor];
            if candidate.get("role").and_then(JsonValue::as_str) != Some("tool") {
                break;
            }
            if let Some(tool_call_id) = candidate
                .get("tool_call_id")
                .and_then(JsonValue::as_str)
                .filter(|id| call_ids.iter().any(|call_id| call_id == id))
            {
                answered.push(tool_call_id.to_owned());
            }
            cursor += 1;
        }

        if call_ids.iter().all(|call_id| answered.contains(call_id)) {
            filtered.push(message.clone());
            filtered.extend(messages[index + 1..cursor].iter().cloned());
        }
        index = cursor;
    }
    filtered
}

fn merge_adjacent_text_messages(messages: Vec<JsonValue>) -> Vec<JsonValue> {
    let mut merged = Vec::<JsonValue>::new();
    for message in messages {
        if let Some(previous) = merged.last_mut() {
            if merge_user_or_text_assistant(previous, &message) {
                continue;
            }
        }
        merged.push(message);
    }
    merged
}

fn message_has_tool_calls(message: &JsonValue) -> bool {
    message.get("role").and_then(JsonValue::as_str) == Some("assistant")
        && message
            .get("tool_calls")
            .and_then(JsonValue::as_array)
            .map(|calls| !calls.is_empty())
            .unwrap_or(false)
}

fn tool_call_ids(message: &JsonValue) -> Vec<String> {
    message
        .get("tool_calls")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|call| call.get("id").and_then(JsonValue::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

fn is_tool_response_for_any(message: &JsonValue, call_ids: &[String]) -> bool {
    message.get("role").and_then(JsonValue::as_str) == Some("tool")
        && message
            .get("tool_call_id")
            .and_then(JsonValue::as_str)
            .map(|id| call_ids.iter().any(|call_id| call_id == id))
            .unwrap_or(false)
}

fn normalize_role(role: Option<&str>) -> &'static str {
    match role {
        Some("assistant") => "assistant",
        Some("tool") => "tool",
        Some("system") | Some("developer") => "system",
        _ => "user",
    }
}

fn response_content_to_chat(content: Option<&JsonValue>) -> JsonValue {
    match content {
        Some(JsonValue::String(value)) => JsonValue::String(value.clone()),
        Some(JsonValue::Array(parts)) => {
            let mapped = parts
                .iter()
                .filter_map(|part| {
                    let object = part.as_object()?;
                    match object.get("type").and_then(JsonValue::as_str) {
                        Some("input_text") | Some("output_text") => Some(json!({
                            "type": "text",
                            "text": object
                                .get("text")
                                .and_then(JsonValue::as_str)
                                .unwrap_or_default(),
                        })),
                        Some("input_image") => Some(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": object
                                    .get("image_url")
                                    .or_else(|| object.get("url"))
                                    .and_then(JsonValue::as_str)
                                    .unwrap_or_default(),
                            }
                        })),
                        _ => Some(part.clone()),
                    }
                })
                .collect::<Vec<_>>();
            if mapped.len() == 1
                && mapped[0].get("type").and_then(JsonValue::as_str) == Some("text")
            {
                mapped[0]
                    .get("text")
                    .cloned()
                    .unwrap_or_else(|| JsonValue::String(String::new()))
            } else {
                JsonValue::Array(mapped)
            }
        }
        Some(value) => value.clone(),
        None => JsonValue::String(String::new()),
    }
}

fn response_tools_to_chat_tools(tools: Option<&JsonValue>) -> Option<JsonValue> {
    let tools = tools?.as_array()?;
    let mapped = tools
        .iter()
        .filter_map(|tool| {
            let object = tool.as_object()?;
            if object.get("type").and_then(JsonValue::as_str) != Some("function") {
                return None;
            }
            if object.contains_key("function") {
                Some(tool.clone())
            } else {
                Some(json!({
                    "type": "function",
                    "function": {
                        "name": object.get("name").and_then(JsonValue::as_str).unwrap_or_default(),
                        "description": object
                            .get("description")
                            .and_then(JsonValue::as_str)
                            .unwrap_or_default(),
                        "parameters": object
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    }
                }))
            }
        })
        .collect::<Vec<_>>();
    (!mapped.is_empty()).then(|| JsonValue::Array(mapped))
}

fn response_tool_choice_to_chat(tool_choice: Option<&JsonValue>) -> Option<JsonValue> {
    match tool_choice {
        Some(JsonValue::Object(object)) => {
            object.get("name").and_then(JsonValue::as_str).map(|name| {
                json!({
                    "type": "function",
                    "function": { "name": name },
                })
            })
        }
        Some(value) => Some(value.clone()),
        None => None,
    }
}

fn apply_effort_translation(output: &mut Map<String, JsonValue>, reasoning: Option<&JsonValue>) {
    let Some(effort) = reasoning
        .and_then(|value| value.get("effort"))
        .and_then(JsonValue::as_str)
    else {
        return;
    };
    apply_effort_string(output, effort);
}

fn apply_effort_string(output: &mut Map<String, JsonValue>, effort: &str) {
    match effort.trim().to_ascii_lowercase().as_str() {
        "" => {}
        "none" => {
            output.insert("thinking".into(), json!({ "type": "disabled" }));
        }
        "minimal" => {
            output.insert("reasoning_effort".into(), JsonValue::String("low".into()));
        }
        value => {
            output.insert("reasoning_effort".into(), JsonValue::String(value.into()));
        }
    }
}

fn chat_completion_to_response(cc: JsonValue, original_request: Option<&JsonValue>) -> JsonValue {
    let response_id = format!("resp_{}", Uuid::new_v4().simple());
    let created_at = cc
        .get("created")
        .and_then(JsonValue::as_i64)
        .unwrap_or_else(|| Utc::now().timestamp());
    let model = original_request
        .and_then(|value| value.get("model"))
        .and_then(JsonValue::as_str)
        .or_else(|| cc.get("model").and_then(JsonValue::as_str))
        .unwrap_or_default();
    let previous_response_id = original_request
        .and_then(|value| value.get("previous_response_id"))
        .cloned()
        .unwrap_or(JsonValue::Null);
    let metadata = original_request
        .and_then(|value| value.get("metadata"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let choice = cc
        .get("choices")
        .and_then(JsonValue::as_array)
        .and_then(|choices| choices.first());
    let mut output = Vec::new();
    let mut status = "completed";
    let mut incomplete_details = JsonValue::Null;

    if let Some(choice) = choice {
        let message = choice.get("message").unwrap_or(&JsonValue::Null);
        if let Some(tool_calls) = message.get("tool_calls").and_then(JsonValue::as_array) {
            for tool_call in tool_calls {
                output.push(json!({
                    "type": "function_call",
                    "id": format!("fc_{}", Uuid::new_v4().simple()),
                    "call_id": tool_call
                        .get("id")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("call_unknown"),
                    "name": tool_call
                        .get("function")
                        .and_then(|value| value.get("name"))
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default(),
                    "arguments": tool_call
                        .get("function")
                        .and_then(|value| value.get("arguments"))
                        .and_then(JsonValue::as_str)
                        .unwrap_or("{}"),
                    "status": "completed",
                }));
            }
        }

        let text = message
            .get("content")
            .and_then(JsonValue::as_str)
            .map(strip_think_blocks)
            .unwrap_or_default();
        if !text.trim().is_empty() {
            output.push(json!({
                "type": "message",
                "id": format!("msg_{}", Uuid::new_v4().simple()),
                "status": "completed",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": text.trim(),
                    "annotations": [],
                }],
            }));
        }

        match choice.get("finish_reason").and_then(JsonValue::as_str) {
            Some("length") => {
                status = "incomplete";
                incomplete_details = json!({ "reason": "max_output_tokens" });
            }
            Some("content_filter") => {
                status = "incomplete";
                incomplete_details = json!({ "reason": "content_filter" });
            }
            _ => {}
        }
    }

    json!({
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "status": status,
        "model": model,
        "output": output,
        "previous_response_id": previous_response_id,
        "metadata": metadata,
        "usage": translate_usage(cc.get("usage")),
        "incomplete_details": incomplete_details,
    })
}

fn translate_usage(usage: Option<&JsonValue>) -> JsonValue {
    let Some(usage) = usage else {
        return json!({
            "input_tokens": 0,
            "output_tokens": 0,
            "total_tokens": 0,
        });
    };
    let prompt_tokens = usage.get("prompt_tokens").and_then(JsonValue::as_u64);
    let prompt_cache_hit_tokens = usage
        .get("prompt_cache_hit_tokens")
        .and_then(JsonValue::as_u64);
    let prompt_cache_miss_tokens = usage
        .get("prompt_cache_miss_tokens")
        .and_then(JsonValue::as_u64);
    let deepseek_prompt_tokens =
        if prompt_cache_hit_tokens.is_some() || prompt_cache_miss_tokens.is_some() {
            Some(prompt_cache_hit_tokens.unwrap_or(0) + prompt_cache_miss_tokens.unwrap_or(0))
        } else {
            None
        };
    let input_tokens = prompt_tokens.or(deepseek_prompt_tokens).unwrap_or(0);
    let output_tokens = usage
        .get("completion_tokens")
        .and_then(JsonValue::as_u64)
        .unwrap_or(0);
    let total_tokens = usage
        .get("total_tokens")
        .and_then(JsonValue::as_u64)
        .unwrap_or(input_tokens + output_tokens);
    json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "total_tokens": total_tokens,
        "input_tokens_details": {
            "cached_tokens": usage
                .get("prompt_tokens_details")
                .and_then(|value| value.get("cached_tokens"))
                .and_then(JsonValue::as_u64)
                .or(prompt_cache_hit_tokens)
                .unwrap_or(0),
        },
        "output_tokens_details": {
            "reasoning_tokens": usage
                .get("completion_tokens_details")
                .and_then(|value| value.get("reasoning_tokens"))
                .and_then(JsonValue::as_u64)
                .unwrap_or(0),
        },
    })
}

fn upstream_error_to_openai_error(status: StatusCode, response_body: &[u8]) -> JsonValue {
    let parsed = serde_json::from_slice::<JsonValue>(response_body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| value.get("error"))
        .and_then(|error| {
            error
                .get("message")
                .and_then(JsonValue::as_str)
                .or_else(|| error.as_str())
        })
        .or_else(|| {
            parsed
                .as_ref()
                .and_then(|value| value.get("message"))
                .and_then(JsonValue::as_str)
        })
        .map(str::to_owned)
        .unwrap_or_else(|| String::from_utf8_lossy(response_body).trim().to_owned())
        .trim()
        .to_owned();
    json!({
        "error": {
            "message": if message.is_empty() {
                format!("DeepSeek upstream returned HTTP {}", status.as_u16())
            } else {
                message
            },
            "type": "upstream_error",
            "param": null,
            "code": parsed
                .as_ref()
                .and_then(|value| value.get("error"))
                .and_then(|error| error.get("code"))
                .cloned()
                .unwrap_or_else(|| JsonValue::String(status.as_u16().to_string())),
        }
    })
}

fn strip_think_blocks(text: &str) -> String {
    let mut remaining = text;
    let mut output = String::new();
    while let Some(start) = remaining.find("<think>") {
        output.push_str(&remaining[..start]);
        let after_start = &remaining[start + "<think>".len()..];
        if let Some(end) = after_start.find("</think>") {
            remaining = &after_start[end + "</think>".len()..];
        } else {
            remaining = "";
            break;
        }
    }
    output.push_str(remaining);
    output
}

fn copy_number(source: &Map<String, JsonValue>, target: &mut Map<String, JsonValue>, key: &str) {
    if source.get(key).and_then(JsonValue::as_f64).is_some() {
        copy_value(source, target, key);
    }
}

fn copy_value(source: &Map<String, JsonValue>, target: &mut Map<String, JsonValue>, key: &str) {
    if let Some(value) = source.get(key) {
        target.insert(key.into(), value.clone());
    }
}

fn stringify_json_value(value: Option<&JsonValue>) -> String {
    match value {
        Some(JsonValue::String(value)) => value.clone(),
        Some(value) => serde_json::to_string(value).unwrap_or_default(),
        None => String::new(),
    }
}

fn normalize_path(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() || path == "/" {
        return "/".into();
    }
    format!("/{}", path.trim_start_matches('/').trim_end_matches('/'))
}

fn push_sse(events: &mut String, event: &str, data: JsonValue) {
    events.push_str("event: ");
    events.push_str(event);
    events.push_str("\ndata: ");
    events.push_str(&serde_json::to_string(&data).unwrap_or_else(|_| "{}".into()));
    events.push_str("\n\n");
}

#[cfg(test)]
mod tests {
    use super::{
        ResponseStore, convert_deepseek_v4_request, convert_deepseek_v4_response,
        response_json_to_sse,
    };
    use bytes::Bytes;
    use hyper::StatusCode;
    use serde_json::Value as JsonValue;

    #[test]
    fn converts_responses_request_to_chat_completions() {
        let input = Bytes::from(
            r#"{
              "model":"deepseek-v4",
              "instructions":"Be concise.",
              "input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}],
              "stream":true,
              "max_output_tokens":123,
              "reasoning":{"effort":"minimal"}
            }"#,
        );
        let store = ResponseStore::new();
        let converted = convert_deepseek_v4_request(&store, "/responses", input)
            .unwrap()
            .unwrap();
        assert_eq!(converted.upstream_path, "/chat/completions");
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();
        assert_eq!(body["model"], "deepseek-v4");
        assert_eq!(body["stream"], false);
        assert_eq!(body["max_tokens"], 123);
        assert_eq!(body["reasoning_effort"], "low");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "hello");
    }

    #[test]
    fn converts_chat_completion_response_to_responses_and_sse() {
        let request =
            r#"{"model":"deepseek-v4","previous_response_id":"resp_prev","metadata":{"a":"b"}}"#;
        let upstream = Bytes::from(
            r#"{
              "id":"chatcmpl_1",
              "created":123,
              "model":"deepseek-v4",
              "choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"<think>x</think>Hello"}}],
              "usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}
            }"#,
        );
        let converted =
            convert_deepseek_v4_response("/responses", StatusCode::OK, request, upstream).unwrap();
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();
        assert_eq!(body["object"], "response");
        assert_eq!(body["previous_response_id"], "resp_prev");
        assert_eq!(body["usage"]["total_tokens"], 15);
        assert_eq!(body["output"][0]["content"][0]["text"], "Hello");

        let sse = response_json_to_sse(&converted.body).unwrap();
        let text = String::from_utf8_lossy(&sse);
        assert!(text.contains("event: response.created"));
        assert!(text.contains("event: response.output_text.delta"));
        assert!(text.contains("event: response.completed"));
    }

    #[test]
    fn converts_deepseek_cache_usage_to_response_usage() {
        let request = r#"{"model":"deepseek-v4","input":"hello"}"#;
        let upstream = Bytes::from(
            r#"{
              "id":"chatcmpl_1",
              "created":123,
              "model":"deepseek-v4",
              "choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"ok"}}],
              "usage":{
                "prompt_cache_hit_tokens":192000,
                "prompt_cache_miss_tokens":107,
                "completion_tokens":51
              }
            }"#,
        );
        let converted =
            convert_deepseek_v4_response("/responses", StatusCode::OK, request, upstream).unwrap();
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();

        assert_eq!(body["usage"]["input_tokens"], 192_107);
        assert_eq!(
            body["usage"]["input_tokens_details"]["cached_tokens"],
            192_000
        );
        assert_eq!(body["usage"]["output_tokens"], 51);
        assert_eq!(body["usage"]["total_tokens"], 192_158);
    }

    #[test]
    fn resolves_previous_response_id_into_upstream_messages() {
        let store = ResponseStore::new();
        let first_request = r#"{"model":"deepseek-v4","input":"hello"}"#;
        let first_response = br#"{
          "id":"resp_first",
          "object":"response",
          "output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]
        }"#;
        store.store_response(first_request, first_response).unwrap();

        let next = Bytes::from(
            r#"{
              "model":"deepseek-v4",
              "previous_response_id":"resp_first",
              "input":"who are you?"
            }"#,
        );
        let converted = convert_deepseek_v4_request(&store, "/responses", next)
            .unwrap()
            .unwrap();
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["content"], "hello");
        assert_eq!(messages[1]["content"], "hi");
        assert_eq!(messages[2]["content"], "who are you?");
    }

    #[test]
    fn disables_thinking_when_tool_calls_are_replayed() {
        let store = ResponseStore::new();
        let input = Bytes::from(
            r#"{
              "model":"deepseek-v4",
              "input":[
                {"type":"function_call","call_id":"call_1","name":"read_file","arguments":"{\"path\":\"a\"}"},
                {"type":"function_call_output","call_id":"call_1","output":"content"}
              ],
              "reasoning":{"effort":"high"}
            }"#,
        );
        let converted = convert_deepseek_v4_request(&store, "/responses", input)
            .unwrap()
            .unwrap();
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn drops_unanswered_tool_calls_before_sending_upstream() {
        let store = ResponseStore::new();
        let input = Bytes::from(
            r#"{
              "model":"deepseek-v4",
              "input":[
                {"type":"message","role":"user","content":"start"},
                {"type":"function_call","call_id":"call_missing","name":"read_file","arguments":"{\"path\":\"a\"}"},
                {"type":"message","role":"user","content":"continue"}
              ]
            }"#,
        );
        let converted = convert_deepseek_v4_request(&store, "/responses", input)
            .unwrap()
            .unwrap();
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();
        let messages = body["messages"].as_array().unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert!(messages[0]["content"].as_str().unwrap().contains("start"));
        assert!(
            messages[0]["content"]
                .as_str()
                .unwrap()
                .contains("continue")
        );
    }

    #[test]
    fn reorders_tool_output_after_matching_tool_call() {
        let store = ResponseStore::new();
        let input = Bytes::from(
            r#"{
              "model":"deepseek-v4",
              "input":[
                {"type":"function_call","call_id":"call_1","name":"read_file","arguments":"{\"path\":\"a\"}"},
                {"type":"message","role":"user","content":"next"},
                {"type":"function_call_output","call_id":"call_1","output":"content"}
              ]
            }"#,
        );
        let converted = convert_deepseek_v4_request(&store, "/responses", input)
            .unwrap()
            .unwrap();
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();
        let messages = body["messages"].as_array().unwrap();

        assert_eq!(messages[0]["role"], "assistant");
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "call_1");
        assert_eq!(messages[2]["role"], "user");
    }

    #[test]
    fn converts_upstream_error_to_openai_error_shape() {
        let converted = convert_deepseek_v4_response(
            "/responses",
            StatusCode::BAD_REQUEST,
            r#"{"model":"deepseek-v4","input":"hello"}"#,
            Bytes::from(r#"{"error":{"message":"bad payload","code":"invalid_request"}}"#),
        )
        .unwrap();
        let body: JsonValue = serde_json::from_slice(&converted.body).unwrap();
        assert_eq!(body["error"]["message"], "bad payload");
        assert_eq!(body["error"]["type"], "upstream_error");
        assert_eq!(body["error"]["code"], "invalid_request");
    }
}
