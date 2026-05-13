use std::{fs, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::Manager;

const AI_CONFIG_FILE: &str = "ai-config.json";
const DEEPSEEK_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
const DEEPSEEK_MODEL: &str = "deepseek-v4-flash";
const DEEPSEEK_TIMEOUT_MS: u64 = 4_500;
const MAX_PARSED_TASKS: usize = 5;
const MAX_TITLE_CHARS: usize = 80;
const MAX_DUE_CHARS: usize = 32;
const MAX_PET_COMMENT_CHARS: usize = 120;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseTaskRequest {
    pub input: String,
    pub now_iso: String,
    pub mood: String,
    pub existing_tasks: Vec<ExistingTask>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingTask {
    pub title: String,
    pub category: String,
    pub priority: String,
    pub due: Option<String>,
    pub completed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTaskResult {
    pub tasks: Vec<ParsedTaskDraft>,
    pub pet_comment: String,
    pub source: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTaskDraft {
    pub title: String,
    pub category: String,
    pub priority: String,
    pub due: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
enum AiConfig {
    PersonalKey {
        #[serde(rename = "deepseekApiKey")]
        deepseek_api_key: String,
    },
    Invite {
        #[serde(rename = "inviteCode")]
        invite_code: String,
        #[serde(rename = "proxyUrl")]
        proxy_url: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfigStatus {
    mode: String,
    has_personal_key: bool,
    has_invite_code: bool,
    proxy_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePersonalAiConfigRequest {
    deepseek_api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInviteAiConfigRequest {
    invite_code: String,
    proxy_url: String,
}

#[derive(Serialize)]
struct DeepSeekRequest {
    model: &'static str,
    messages: Vec<DeepSeekMessage>,
    response_format: DeepSeekResponseFormat,
    temperature: f32,
    max_tokens: u16,
    stream: bool,
}

#[derive(Serialize)]
struct DeepSeekMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct DeepSeekResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InviteProxyRequest<'a> {
    invite_code: &'a str,
    request: &'a ParseTaskRequest,
}

fn build_invite_proxy_request<'a>(
    invite_code: &'a str,
    request: &'a ParseTaskRequest,
) -> InviteProxyRequest<'a> {
    InviteProxyRequest {
        invite_code,
        request,
    }
}

#[cfg(test)]
fn parse_backend_name(config: &AiConfig) -> &'static str {
    match config {
        AiConfig::PersonalKey { .. } => "personalKey",
        AiConfig::Invite { .. } => "invite",
    }
}

#[derive(Deserialize)]
struct DeepSeekResponse {
    choices: Vec<DeepSeekChoice>,
}

#[derive(Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekChoiceMessage,
}

#[derive(Deserialize)]
struct DeepSeekChoiceMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelParsedResult {
    tasks: Vec<ModelParsedTaskDraft>,
    pet_comment: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelParsedTaskDraft {
    title: String,
    category: String,
    priority: String,
    due: Option<String>,
}

fn ai_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|_| "AI config path is unavailable".to_string())?
        .join(AI_CONFIG_FILE))
}

#[tauri::command]
pub async fn parse_task_with_ai(
    app: tauri::AppHandle,
    request: ParseTaskRequest,
) -> Result<ParsedTaskResult, String> {
    if request.input.trim().is_empty() {
        return Err("empty input".into());
    }

    let config_content = fs::read_to_string(ai_config_path(&app)?)
        .map_err(|_| "AI config file is missing".to_string())?;
    let config = parse_ai_config(&config_content)?;

    match config {
        AiConfig::PersonalKey { deepseek_api_key } => {
            call_deepseek(&deepseek_api_key, &request).await
        }
        AiConfig::Invite {
            invite_code,
            proxy_url,
        } => call_invite_proxy(&proxy_url, &invite_code, &request).await,
    }
}

#[tauri::command]
pub fn load_ai_config_status(app: tauri::AppHandle) -> Result<AiConfigStatus, String> {
    let config_path = ai_config_path(&app)?;
    let config_content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ai_config_status(None));
        }
        Err(_) => return Err("AI config could not be read".into()),
    };
    let config = parse_ai_config(&config_content)?;

    Ok(ai_config_status(Some(&config)))
}

#[tauri::command]
pub fn save_personal_ai_config(
    app: tauri::AppHandle,
    request: SavePersonalAiConfigRequest,
) -> Result<AiConfigStatus, String> {
    let config = normalize_ai_config(AiConfig::PersonalKey {
        deepseek_api_key: request.deepseek_api_key,
    })?;
    let content = serde_json::to_string_pretty(&config)
        .map_err(|_| "AI config could not be saved".to_string())?;
    let config_path = ai_config_path(&app)?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|_| "AI config could not be saved".to_string())?;
    }

    fs::write(config_path, content).map_err(|_| "AI config could not be saved".to_string())?;
    Ok(ai_config_status(Some(&config)))
}

#[tauri::command]
pub fn save_invite_ai_config(
    app: tauri::AppHandle,
    request: SaveInviteAiConfigRequest,
) -> Result<AiConfigStatus, String> {
    let config = normalize_ai_config(AiConfig::Invite {
        invite_code: request.invite_code,
        proxy_url: request.proxy_url,
    })?;
    let content = serde_json::to_string_pretty(&config)
        .map_err(|_| "AI config could not be saved".to_string())?;
    let config_path = ai_config_path(&app)?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|_| "AI config could not be saved".to_string())?;
    }

    fs::write(config_path, content).map_err(|_| "AI config could not be saved".to_string())?;
    Ok(ai_config_status(Some(&config)))
}

#[tauri::command]
pub fn clear_ai_config(app: tauri::AppHandle) -> Result<AiConfigStatus, String> {
    let config_path = ai_config_path(&app)?;

    match fs::remove_file(config_path) {
        Ok(()) => Ok(ai_config_status(None)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ai_config_status(None)),
        Err(_) => Err("AI config could not be cleared".into()),
    }
}

#[tauri::command]
pub fn get_app_local_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|_| "app data path is unavailable".to_string())?
        .to_string_lossy()
        .into_owned())
}

async fn call_deepseek(
    api_key: &str,
    request: &ParseTaskRequest,
) -> Result<ParsedTaskResult, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(DEEPSEEK_TIMEOUT_MS))
        .build()
        .map_err(|_| "DeepSeek client could not be created".to_string())?;

    let response = client
        .post(DEEPSEEK_ENDPOINT)
        .bearer_auth(api_key)
        .json(&build_deepseek_request(request))
        .send()
        .await
        .map_err(|_| "DeepSeek request failed".to_string())?;

    if !response.status().is_success() {
        return Err("DeepSeek request failed".into());
    }

    let body = response
        .text()
        .await
        .map_err(|_| "DeepSeek response was invalid".to_string())?;

    parse_deepseek_response(&body)
}

async fn call_invite_proxy(
    proxy_url: &str,
    invite_code: &str,
    request: &ParseTaskRequest,
) -> Result<ParsedTaskResult, String> {
    validate_proxy_url(proxy_url)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(DEEPSEEK_TIMEOUT_MS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "proxy client could not be created".to_string())?;

    let response = client
        .post(proxy_url)
        .json(&build_invite_proxy_request(invite_code, request))
        .send()
        .await
        .map_err(|_| "proxy request failed".to_string())?;

    if !response.status().is_success() {
        return Err("proxy request failed".into());
    }

    let body = response
        .text()
        .await
        .map_err(|_| "proxy response was invalid".to_string())?;

    normalize_proxy_response(&body)
}

fn normalize_proxy_response(content: &str) -> Result<ParsedTaskResult, String> {
    let result: ParsedTaskResult =
        serde_json::from_str(content).map_err(|_| "proxy response was invalid".to_string())?;

    if result.source != "deepseek" {
        return Err("proxy response was invalid".into());
    }

    normalize_parsed_result(result)
}

fn normalize_parsed_result(result: ParsedTaskResult) -> Result<ParsedTaskResult, String> {
    if result.tasks.is_empty() || result.tasks.len() > MAX_PARSED_TASKS {
        return Err("proxy response was invalid".into());
    }

    let pet_comment = result.pet_comment.trim();

    if pet_comment.is_empty() || pet_comment.chars().count() > MAX_PET_COMMENT_CHARS {
        return Err("proxy response was invalid".into());
    }

    let tasks = result
        .tasks
        .into_iter()
        .map(|task| {
            normalize_model_task(ModelParsedTaskDraft {
                title: task.title,
                category: task.category,
                priority: task.priority,
                due: task.due,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ParsedTaskResult {
        tasks,
        pet_comment: pet_comment.into(),
        source: "deepseek".into(),
    })
}

fn parse_ai_config(content: &str) -> Result<AiConfig, String> {
    let config: AiConfig =
        serde_json::from_str(content).map_err(|_| "AI config JSON is invalid".to_string())?;
    normalize_ai_config(config)
}

fn normalize_ai_config(config: AiConfig) -> Result<AiConfig, String> {
    match config {
        AiConfig::PersonalKey { deepseek_api_key } => {
            let key = deepseek_api_key.trim();

            if key.is_empty() {
                Err("DeepSeek API key is missing".into())
            } else {
                Ok(AiConfig::PersonalKey {
                    deepseek_api_key: key.to_owned(),
                })
            }
        }
        AiConfig::Invite {
            invite_code,
            proxy_url,
        } => {
            let code = invite_code.trim();
            let url = proxy_url.trim();

            if code.is_empty() {
                return Err("invite code is missing".into());
            }

            validate_proxy_url(url)?;

            Ok(AiConfig::Invite {
                invite_code: code.to_owned(),
                proxy_url: url.to_owned(),
            })
        }
    }
}

fn validate_proxy_url(value: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(value).map_err(|_| "proxy URL is invalid".to_string())?;

    match url.scheme() {
        "https" => Ok(()),
        "http" if is_localhost_url(&url) => Ok(()),
        _ => Err("proxy URL is invalid".into()),
    }
}

fn is_localhost_url(url: &reqwest::Url) -> bool {
    matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"))
}

fn ai_config_status(config: Option<&AiConfig>) -> AiConfigStatus {
    match config {
        Some(AiConfig::PersonalKey { .. }) => AiConfigStatus {
            mode: "personalKey".into(),
            has_personal_key: true,
            has_invite_code: false,
            proxy_url: None,
        },
        Some(AiConfig::Invite { proxy_url, .. }) => AiConfigStatus {
            mode: "invite".into(),
            has_personal_key: false,
            has_invite_code: true,
            proxy_url: Some(proxy_url.clone()),
        },
        None => AiConfigStatus {
            mode: "none".into(),
            has_personal_key: false,
            has_invite_code: false,
            proxy_url: None,
        },
    }
}

fn build_deepseek_request(request: &ParseTaskRequest) -> DeepSeekRequest {
    DeepSeekRequest {
        model: DEEPSEEK_MODEL,
        messages: vec![
            DeepSeekMessage {
                role: "system",
                content: system_prompt(),
            },
            DeepSeekMessage {
                role: "user",
                content: build_user_prompt(request),
            },
        ],
        response_format: DeepSeekResponseFormat {
            kind: "json_object",
        },
        temperature: 0.2,
        max_tokens: 700,
        stream: false,
    }
}

fn system_prompt() -> String {
    [
        "你是 FocusFlow 的任务解析器，只能输出 JSON，不能输出 Markdown 或解释。",
        "把用户输入拆成 1 到 5 个任务。category 只能是 work、study、life、idea。",
        "priority 只能是 high、medium、low。due 使用用户原文里的简短时间表达，没有就为 null。",
        "petComment 要简短、温暖、支持注意力困难用户，不要提到 AI、模型或服务商。",
        "JSON 结构必须是 {\"tasks\":[{\"title\":string,\"category\":string,\"priority\":string,\"due\":string|null}],\"petComment\":string}。",
    ]
    .join("\n")
}

fn build_user_prompt(request: &ParseTaskRequest) -> String {
    let active_tasks: Vec<&ExistingTask> = request
        .existing_tasks
        .iter()
        .filter(|task| !task.completed && !task.title.trim().is_empty())
        .take(8)
        .collect();
    let existing_tasks = serde_json::to_string(&active_tasks).unwrap_or_else(|_| "[]".into());

    format!(
        "当前时间：{}\n当前心情：{}\n未完成任务上下文：{}\n用户输入：{}",
        request.now_iso.trim(),
        request.mood.trim(),
        existing_tasks,
        request.input.trim()
    )
}

fn parse_deepseek_response(content: &str) -> Result<ParsedTaskResult, String> {
    let response: DeepSeekResponse =
        serde_json::from_str(content).map_err(|_| "DeepSeek response was invalid".to_string())?;
    let model_content = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or_else(|| "DeepSeek response was invalid".to_string())?;

    parse_model_content(model_content)
}

fn parse_model_content(content: &str) -> Result<ParsedTaskResult, String> {
    let model_result: ModelParsedResult =
        serde_json::from_str(content).map_err(|_| "DeepSeek response was invalid".to_string())?;
    normalize_model_result(model_result)
}

fn normalize_model_result(model_result: ModelParsedResult) -> Result<ParsedTaskResult, String> {
    if model_result.tasks.is_empty() || model_result.tasks.len() > MAX_PARSED_TASKS {
        return Err("DeepSeek response was invalid".into());
    }

    let pet_comment = model_result.pet_comment.trim();

    if pet_comment.is_empty() || pet_comment.chars().count() > MAX_PET_COMMENT_CHARS {
        return Err("DeepSeek response was invalid".into());
    }

    let tasks = model_result
        .tasks
        .into_iter()
        .map(normalize_model_task)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ParsedTaskResult {
        tasks,
        pet_comment: pet_comment.into(),
        source: "deepseek".into(),
    })
}

fn normalize_model_task(task: ModelParsedTaskDraft) -> Result<ParsedTaskDraft, String> {
    let title = task.title.trim();

    if title.is_empty()
        || title.chars().count() > MAX_TITLE_CHARS
        || !is_category(&task.category)
        || !is_priority(&task.priority)
    {
        return Err("DeepSeek response was invalid".into());
    }

    let due = match task.due {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else if trimmed.chars().count() > MAX_DUE_CHARS {
                return Err("DeepSeek response was invalid".into());
            } else {
                Some(trimmed.into())
            }
        }
        None => None,
    };

    Ok(ParsedTaskDraft {
        title: title.into(),
        category: task.category,
        priority: task.priority,
        due,
    })
}

fn is_category(value: &str) -> bool {
    matches!(value, "work" | "study" | "life" | "idea")
}

fn is_priority(value: &str) -> bool {
    matches!(value, "high" | "medium" | "low")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_model_json_normalizes_to_deepseek_result() {
        let result = parse_model_content(
            r#"
            {
              "tasks": [
                {
                  "title": " 回复导师邮件 ",
                  "category": "work",
                  "priority": "high",
                  "due": " 今天 16:00 "
                }
              ],
              "petComment": " 我先帮你拆好了，选中要加入的就行。 "
            }
            "#,
        )
        .unwrap();

        assert_eq!(result.source, "deepseek");
        assert_eq!(result.pet_comment, "我先帮你拆好了，选中要加入的就行。");
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].title, "回复导师邮件");
        assert_eq!(result.tasks[0].category, "work");
        assert_eq!(result.tasks[0].priority, "high");
        assert_eq!(result.tasks[0].due.as_deref(), Some("今天 16:00"));
    }

    #[test]
    fn model_json_rejects_empty_tasks() {
        assert!(parse_model_content(r#"{"tasks":[],"petComment":"我整理好了。"}"#).is_err());
    }

    #[test]
    fn model_json_rejects_invalid_category() {
        assert!(parse_model_content(
            r#"{"tasks":[{"title":"买牛奶","category":"shopping","priority":"low","due":null}],"petComment":"我整理好了。"}"#,
        )
        .is_err());
    }

    #[test]
    fn model_json_rejects_invalid_priority() {
        assert!(parse_model_content(
            r#"{"tasks":[{"title":"买牛奶","category":"life","priority":"urgent","due":null}],"petComment":"我整理好了。"}"#,
        )
        .is_err());
    }

    #[test]
    fn model_json_rejects_empty_title() {
        assert!(parse_model_content(
            r#"{"tasks":[{"title":"   ","category":"life","priority":"low","due":null}],"petComment":"我整理好了。"}"#,
        )
        .is_err());
    }

    #[test]
    fn empty_due_string_normalizes_to_none() {
        let result = parse_model_content(
            r#"{"tasks":[{"title":"买牛奶","category":"life","priority":"low","due":"   "}],"petComment":"我整理好了。"}"#,
        )
        .unwrap();

        assert_eq!(result.tasks[0].due, None);
    }

    #[test]
    fn personal_key_config_status_does_not_expose_secret() {
        let config =
            parse_ai_config(r#"{"mode":"personalKey","deepseekApiKey":" sk-test "}"#).unwrap();
        let status = ai_config_status(Some(&config));

        assert_eq!(status.mode, "personalKey");
        assert!(status.has_personal_key);
        assert!(!status.has_invite_code);
        assert_eq!(status.proxy_url, None);
    }

    #[test]
    fn invite_config_status_exposes_proxy_url_only() {
        let config = parse_ai_config(
            r#"{"mode":"invite","inviteCode":" friend-001 ","proxyUrl":"https://example.vercel.app/api/parse-task"}"#,
        )
        .unwrap();
        let status = ai_config_status(Some(&config));

        assert_eq!(status.mode, "invite");
        assert!(!status.has_personal_key);
        assert!(status.has_invite_code);
        assert_eq!(
            status.proxy_url.as_deref(),
            Some("https://example.vercel.app/api/parse-task")
        );
    }

    #[test]
    fn missing_config_status_is_not_configured() {
        let status = ai_config_status(None);

        assert_eq!(status.mode, "none");
        assert!(!status.has_personal_key);
        assert!(!status.has_invite_code);
        assert_eq!(status.proxy_url, None);
    }

    #[test]
    fn config_rejects_empty_personal_key() {
        assert!(parse_ai_config(r#"{"mode":"personalKey","deepseekApiKey":"   "}"#).is_err());
    }

    #[test]
    fn config_rejects_empty_invite_code() {
        assert!(parse_ai_config(
            r#"{"mode":"invite","inviteCode":"   ","proxyUrl":"https://example.vercel.app/api/parse-task"}"#,
        )
        .is_err());
    }

    #[test]
    fn config_accepts_https_proxy_url() {
        assert!(parse_ai_config(
            r#"{"mode":"invite","inviteCode":"friend-001","proxyUrl":"https://example.vercel.app/api/parse-task"}"#
        )
        .is_ok());
    }

    #[test]
    fn config_accepts_http_localhost_proxy_url() {
        assert!(parse_ai_config(
            r#"{"mode":"invite","inviteCode":"friend-001","proxyUrl":"http://localhost:3000/api/parse-task"}"#
        )
        .is_ok());
    }

    #[test]
    fn config_rejects_http_public_proxy_url() {
        assert!(parse_ai_config(
            r#"{"mode":"invite","inviteCode":"friend-001","proxyUrl":"http://example.com/api/parse-task"}"#
        )
        .is_err());
    }

    #[test]
    fn config_rejects_file_proxy_url() {
        assert!(parse_ai_config(
            r#"{"mode":"invite","inviteCode":"friend-001","proxyUrl":"file:///tmp/key"}"#
        )
        .is_err());
    }

    #[test]
    fn invite_proxy_payload_contains_code_and_request() {
        let request = ParseTaskRequest {
            input: "今晚八点前回复导师邮件".into(),
            now_iso: "2026-05-06T10:00:00Z".into(),
            mood: "一般".into(),
            existing_tasks: vec![],
        };
        let payload = build_invite_proxy_request("friend-001", &request);
        let value = serde_json::to_value(payload).unwrap();

        assert_eq!(value["inviteCode"], "friend-001");
        assert_eq!(value["request"]["input"], "今晚八点前回复导师邮件");
        assert_eq!(value["request"]["mood"], "一般");
    }

    #[test]
    fn invite_config_routes_to_proxy_backend() {
        let config = AiConfig::Invite {
            invite_code: "friend-001".into(),
            proxy_url: "https://example.vercel.app/api/parse-task".into(),
        };

        assert_eq!(parse_backend_name(&config), "invite");
    }

    #[test]
    fn personal_config_routes_to_local_backend() {
        let config = AiConfig::PersonalKey {
            deepseek_api_key: "sk-test".into(),
        };

        assert_eq!(parse_backend_name(&config), "personalKey");
    }

    #[test]
    fn proxy_response_normalizes_existing_result_shape() {
        let result = normalize_proxy_response(
            r#"{
              "tasks":[{"title":" 回复导师邮件 ","category":"work","priority":"high","due":" 今天 "}],
              "petComment":" 我整理好了。 ",
              "source":"deepseek"
            }"#,
        )
        .unwrap();

        assert_eq!(result.source, "deepseek");
        assert_eq!(result.pet_comment, "我整理好了。");
        assert_eq!(result.tasks[0].title, "回复导师邮件");
        assert_eq!(result.tasks[0].due.as_deref(), Some("今天"));
    }

    #[test]
    fn proxy_response_rejects_unexpected_source() {
        assert!(normalize_proxy_response(
            r#"{"tasks":[{"title":"买牛奶","category":"life","priority":"low","due":null}],"petComment":"我整理好了。","source":"proxy"}"#,
        )
        .is_err());
    }

    #[test]
    fn deepseek_response_without_content_is_rejected() {
        assert!(parse_deepseek_response(r#"{"choices":[{"message":{}}]}"#).is_err());
    }
}
