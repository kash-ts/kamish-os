//! Integration tests for AI API endpoints:
//! - GET /api/v1/ai/tools
//! - POST /api/v1/ai/prompt
//! - GET /api/v1/ai/sessions/:id

use axum::http::StatusCode;
use kamish_core::common::AppState;
use kamish_core::database::{init_pool, run_migrations};
use reqwest::Client;
use serde_json::{json, Value};

async fn spawn_test_server() -> (String, AppState) {
    kamish_core::audio::alert::set_audio_muted(true);
    let pool = init_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // Use Mock/Intelligent Fallback Engine (cli_enabled = false for offline test determinism)
    let state = AppState::with_options(pool, "agy".to_string(), false);
    let app = kamish_core::api::create_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), state)
}

#[tokio::test]
async fn test_get_ai_tools_returns_all_14_schemas() {
    let (base_url, _state) = spawn_test_server().await;
    let client = Client::new();

    let res = client
        .get(format!("{base_url}/api/v1/ai/tools"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();

    assert_eq!(body["count"], 14);
    let tools = body["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools.len(), 14);

    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool must have name"))
        .collect();

    let expected_14_tools = [
        "create_alarm",
        "delete_alarm",
        "list_alarms",
        "snooze_alarm",
        "create_reminder",
        "list_reminders",
        "set_volume",
        "play_music",
        "pause_music",
        "resume_music",
        "stop_music",
        "search_music",
        "get_current_track",
        "get_device_status",
    ];

    for expected in expected_14_tools {
        assert!(
            tool_names.contains(&expected),
            "Missing expected tool declaration: '{expected}'"
        );
    }

    // Verify parameter schemas exist and are objects
    for tool in tools {
        assert!(tool["description"].is_string());
        assert!(tool["parameters"].is_object());
    }
}

#[tokio::test]
async fn test_ai_prompt_alarm_lifecycle() {
    let (base_url, state) = spawn_test_server().await;
    let client = Client::new();

    // 1. Create alarm via natural language
    let prompt_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Поставь будильник на 07:45 по будням с меткой работа"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(prompt_res.status(), StatusCode::OK);
    let res_body: Value = prompt_res.json().await.unwrap();
    assert!(!res_body["session_id"].as_str().unwrap().is_empty());
    assert!(res_body["response"].as_str().unwrap().contains("07:45"));

    let tool_calls = res_body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["tool"], "create_alarm");
    assert_eq!(tool_calls[0]["arguments"]["time"], "07:45");
    assert_eq!(tool_calls[0]["arguments"]["recurrence"], "weekdays");
    assert_eq!(tool_calls[0]["arguments"]["label"], "работа");
    assert_eq!(tool_calls[0]["success"], true);

    let alarm_id = tool_calls[0]["result"]["id"].as_str().unwrap();

    // Verify alarm exists in DB
    let db_alarms = kamish_core::scheduler::repository::list_alarms(&state.db)
        .await
        .unwrap();
    assert_eq!(db_alarms.len(), 1);
    assert_eq!(db_alarms[0].id, alarm_id);

    // 2. List alarms
    let list_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Покажи будильники"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);
    let list_body: Value = list_res.json().await.unwrap();
    assert_eq!(list_body["tool_calls"][0]["tool"], "list_alarms");

    // 3. Snooze alarm
    let snooze_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Отложи будильник на 10 минут"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(snooze_res.status(), StatusCode::OK);
    let snooze_body: Value = snooze_res.json().await.unwrap();
    assert_eq!(snooze_body["tool_calls"][0]["tool"], "snooze_alarm");
    assert_eq!(snooze_body["tool_calls"][0]["success"], true);

    // 4. Delete alarm
    let del_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": format!("Удали будильник {}", alarm_id)
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(del_res.status(), StatusCode::OK);
    let del_body: Value = del_res.json().await.unwrap();
    assert_eq!(del_body["tool_calls"][0]["tool"], "delete_alarm");
    assert_eq!(del_body["tool_calls"][0]["success"], true);

    let remaining_alarms = kamish_core::scheduler::repository::list_alarms(&state.db)
        .await
        .unwrap();
    assert_eq!(remaining_alarms.len(), 0);
}

#[tokio::test]
async fn test_ai_prompt_reminders_and_volume_and_music() {
    let (base_url, state) = spawn_test_server().await;
    let client = Client::new();

    // Reminder
    let rem_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Напомни через 15 минут выпить воды"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(rem_res.status(), StatusCode::OK);
    let rem_body: Value = rem_res.json().await.unwrap();
    assert_eq!(rem_body["tool_calls"][0]["tool"], "create_reminder");
    assert_eq!(rem_body["tool_calls"][0]["success"], true);

    // Volume
    let vol_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Установи громкость 70%"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(vol_res.status(), StatusCode::OK);
    let vol_body: Value = vol_res.json().await.unwrap();
    assert_eq!(vol_body["tool_calls"][0]["tool"], "set_volume");
    assert_eq!(vol_body["tool_calls"][0]["arguments"]["volume"], 70);
    assert_eq!(state.media.get_volume(), 70);

    // Play music
    let play_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Включи музыку Queen"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(play_res.status(), StatusCode::OK);
    let play_body: Value = play_res.json().await.unwrap();
    assert_eq!(play_body["tool_calls"][0]["tool"], "play_music");
    assert_eq!(play_body["tool_calls"][0]["arguments"]["query"], "Queen");
    assert_eq!(state.media.get_status().await.to_string(), "playing");

    // Pause music
    let pause_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Пауза"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(pause_res.status(), StatusCode::OK);
    let pause_body: Value = pause_res.json().await.unwrap();
    assert_eq!(pause_body["tool_calls"][0]["tool"], "pause_music");
    assert_eq!(state.media.get_status().await.to_string(), "paused");

    // Resume music
    let resume_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Продолжи воспроизведение"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resume_res.status(), StatusCode::OK);
    let resume_body: Value = resume_res.json().await.unwrap();
    assert_eq!(resume_body["tool_calls"][0]["tool"], "resume_music");
    assert_eq!(state.media.get_status().await.to_string(), "playing");

    // Stop music
    let stop_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Стоп"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(stop_res.status(), StatusCode::OK);
    let stop_body: Value = stop_res.json().await.unwrap();
    assert_eq!(stop_body["tool_calls"][0]["tool"], "stop_music");
    assert_eq!(state.media.get_status().await.to_string(), "stopped");

    // Device status
    let status_res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Статус устройства"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(status_res.status(), StatusCode::OK);
    let status_body: Value = status_res.json().await.unwrap();
    assert_eq!(status_body["tool_calls"][0]["tool"], "get_device_status");
}

#[tokio::test]
async fn test_ai_session_persistence_and_history() {
    let (base_url, _state) = spawn_test_server().await;
    let client = Client::new();
    let session_id = "test-session-uuid-12345";

    // First prompt in session
    let res1 = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Привет!",
            "session_id": session_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let body1: Value = res1.json().await.unwrap();
    assert_eq!(body1["session_id"], session_id);

    // Second prompt in same session
    let res2 = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "Поставь громкость 80",
            "session_id": session_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::OK);

    // Fetch session history
    let sess_res = client
        .get(format!("{base_url}/api/v1/ai/sessions/{session_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(sess_res.status(), StatusCode::OK);
    let sess_body: Value = sess_res.json().await.unwrap();
    assert_eq!(sess_body["session_id"], session_id);

    let messages = sess_body["messages"].as_array().unwrap();
    // 2 turns = 4 messages (user, assistant, user, assistant)
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "Привет!");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"], "Поставь громкость 80");
    assert_eq!(messages[3]["role"], "assistant");
    assert!(messages[3]["tool_calls"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn test_ai_prompt_validation_error_on_empty() {
    let (base_url, _state) = spawn_test_server().await;
    let client = Client::new();

    let res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "    "
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_ai_prompt_reports_tool_errors_transparently() {
    let (base_url, _state) = spawn_test_server().await;
    let client = Client::new();

    // 1. Trying to delete a non-existent alarm ID -> tool returns success: false
    let res = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "удали будильник non-existent-id-12345"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();

    // Tool call must reflect failure
    let tool_calls = body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["tool"], "delete_alarm");
    assert_eq!(tool_calls[0]["success"], false);
    assert!(tool_calls[0]["error"].as_str().is_some());

    // Response must NOT falsely claim success
    let response_text = body["response"].as_str().unwrap();
    assert!(
        response_text.contains("не найден") || response_text.contains("Не удалось"),
        "Response should state failure, got: {response_text}"
    );
}

#[tokio::test]
async fn test_ai_readiness_probe_and_config_switching() {
    let (base_url, _state) = spawn_test_server().await;
    let client = Client::new();

    // 1. Initial status: server started in mock mode
    let res = client
        .get(format!("{base_url}/api/v1/ai/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let status_json: Value = res.json().await.unwrap();
    assert_eq!(status_json["mode"], "mock");
    assert_eq!(status_json["ready"], true);

    // 2. Switch to CLI mode pointing to non-existent binary
    let res_config = client
        .post(format!("{base_url}/api/v1/ai/config"))
        .json(&json!({
            "mode": "cli",
            "cli_path": "/non/existent/agy"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res_config.status(), StatusCode::OK);
    let cli_status: Value = res_config.json().await.unwrap();
    assert_eq!(cli_status["mode"], "cli");
    assert_eq!(cli_status["ready"], false);
    assert!(cli_status["error_reason"]
        .as_str()
        .unwrap()
        .contains("не найден"));

    // 3. When in broken CLI mode, speaker REFUSES to answer commands!
    let res_refuse = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "поставь будильник на 07:00"
        }))
        .send()
        .await
        .unwrap();
    // Must fail and inform user that setup is incomplete
    assert_ne!(res_refuse.status(), StatusCode::OK);

    // 4. Point to real working CLI script (scripts/agy-mock)
    let mock_script_path = std::fs::canonicalize("../scripts/agy-mock")
        .or_else(|_| std::fs::canonicalize("scripts/agy-mock"))
        .expect("scripts/agy-mock must exist");

    let res_working_cli = client
        .post(format!("{base_url}/api/v1/ai/config"))
        .json(&json!({
            "mode": "cli",
            "cli_path": mock_script_path.to_str().unwrap()
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res_working_cli.status(), StatusCode::OK);
    let working_status: Value = res_working_cli.json().await.unwrap();
    assert_eq!(working_status["mode"], "cli");
    assert_eq!(working_status["ready"], true);
    assert!(working_status["cli_version"]
        .as_str()
        .unwrap()
        .contains("Antigravity CLI"));

    // 5. Prompt now successfully executes through the real CLI process!
    let res_cli_prompt = client
        .post(format!("{base_url}/api/v1/ai/prompt"))
        .json(&json!({
            "prompt": "поставь будильник на 07:30"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res_cli_prompt.status(), StatusCode::OK);
    let cli_prompt_body: Value = res_cli_prompt.json().await.unwrap();
    assert!(cli_prompt_body["response"]
        .as_str()
        .unwrap()
        .contains("Antigravity CLI"));
    assert_eq!(cli_prompt_body["tool_calls"].as_array().unwrap().len(), 1);
}
