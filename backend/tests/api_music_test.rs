use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kamish_core::{audio::alert::set_audio_muted, build_app, config::Config};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn setup_app() -> axum::Router {
    set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = format!("sqlite::memory:?cache=shared");

    let (app, _state) = build_app(&config)
        .await
        .expect("Failed to build app for music tests");

    app
}

#[tokio::test]
async fn test_music_radio_stations_endpoint() {
    set_audio_muted(true);
    let app = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/radio")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let stations = json["stations"].as_array().expect("stations array");
    assert!(stations.len() >= 5);

    let station_names: Vec<&str> = stations.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(station_names.contains(&"Chillhop Radio"));
    assert!(station_names.contains(&"SomaFM Groove Salad"));
}

#[tokio::test]
async fn test_music_search_endpoint() {
    set_audio_muted(true);
    let app = setup_app().await;

    // Search for radio or seeded track
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/search?q=chillhop&source=radio")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["source"], "radio");
    assert!(json["results"].as_array().unwrap().len() >= 1);
    assert_eq!(json["results"][0]["title"], "Chillhop Radio");
}

#[tokio::test]
async fn test_music_playback_lifecycle() {
    set_audio_muted(true);
    let app = setup_app().await;

    // 1. Initial status: stopped
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let cur: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(cur["status"], "stopped");

    // 2. Play a track
    let play_payload = json!({
        "title": "Acoustic Sunset",
        "artist": "Solar Wind",
        "uri": "https://example.com/audio.mp3"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/play")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&play_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let play_res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(play_res["status"], "playing");
    assert_eq!(play_res["track"]["title"], "Acoustic Sunset");

    // 3. Current should be playing
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let cur: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(cur["status"], "playing");
    assert_eq!(cur["current_track"]["title"], "Acoustic Sunset");

    // 4. Pause
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/pause")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let cur: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(cur["status"], "paused");

    // 5. Resume
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/resume")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. Stop
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let cur: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(cur["status"], "stopped");
}

#[tokio::test]
async fn test_music_likes_and_favorites() {
    set_audio_muted(true);
    let app = setup_app().await;

    // Play track
    let play_payload = json!({
        "track_id": "fav-track-42",
        "title": "Loved Symphony",
        "artist": "Masterpiece Composer"
    });
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/play")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&play_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Toggle like for current track
    let like_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/like")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "track_id": "fav-track-42" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(like_res.status(), StatusCode::OK);
    let body = like_res.into_body().collect().await.unwrap().to_bytes();
    let like_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(like_json["liked"], true);

    // Get favorites
    let fav_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/favorites")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fav_res.status(), StatusCode::OK);
    let body = fav_res.into_body().collect().await.unwrap().to_bytes();
    let fav_json: Value = serde_json::from_slice(&body).unwrap();
    let fav_list = fav_json["favorites"].as_array().unwrap();
    assert!(fav_list.iter().any(|t| t["title"] == "Loved Symphony"));

    // Toggle like off
    let unlike_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/like")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "track_id": "fav-track-42" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = unlike_res.into_body().collect().await.unwrap().to_bytes();
    let unlike_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(unlike_json["liked"], false);
}

#[tokio::test]
async fn test_music_skip_and_recommendations() {
    set_audio_muted(true);
    let app = setup_app().await;

    // Play a track and skip it
    let play_payload = json!({
        "track_id": "skipped-song-99",
        "title": "Annoying Noise",
        "artist": "Noisy Artist"
    });
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/play")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&play_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let skip_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/skip")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "track_id": "skipped-song-99", "position_secs": 8 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(skip_res.status(), StatusCode::OK);
    let body = skip_res.into_body().collect().await.unwrap().to_bytes();
    let skip_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(skip_json["status"], "skipped");

    // Check recommendations
    let rec_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/recommendations?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rec_res.status(), StatusCode::OK);
    let body = rec_res.into_body().collect().await.unwrap().to_bytes();
    let rec_json: Value = serde_json::from_slice(&body).unwrap();
    let recs = rec_json["recommendations"].as_array().unwrap();
    assert!(!recs.is_empty());

    // The skipped track should have a penalty in reasons or lower score
    if let Some(skipped_item) = recs.iter().find(|r| r["track"]["id"] == "skipped-song-99") {
        assert!(skipped_item["reason"].as_str().unwrap().contains("Skipped"));
    }
}

#[tokio::test]
async fn test_music_history_endpoint() {
    set_audio_muted(true);
    let app = setup_app().await;

    let play_payload = json!({
        "track_id": "hist-track-1",
        "title": "History Track Alpha",
        "artist": "Alpha Band"
    });
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/play")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&play_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let hist_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/history?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hist_res.status(), StatusCode::OK);
    let body = hist_res.into_body().collect().await.unwrap().to_bytes();
    let hist_json: Value = serde_json::from_slice(&body).unwrap();
    let history = hist_json["history"].as_array().unwrap();
    assert!(history.iter().any(|h| h["track_id"] == "hist-track-1"));
}

#[tokio::test]
async fn test_lavalink_status_endpoint() {
    set_audio_muted(true);
    let app = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/music/lavalink/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("connected").is_some());
    assert_eq!(json["port"], 2333);
    assert_eq!(json["host"], "127.0.0.1");
}

#[tokio::test]
async fn test_lavalink_config_endpoint() {
    set_audio_muted(true);
    let app = setup_app().await;

    let payload = json!({
        "host": "127.0.0.1",
        "port": 2333,
        "password": "youshallnotpass",
        "yandex_music_token": "test_yandex_token_xyz"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/music/lavalink/config")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["lavalink"]["yandex_music_configured"], true);
}

#[tokio::test]
async fn test_music_search_multisource_endpoints() {
    set_audio_muted(true);
    let app = setup_app().await;

    for source in ["yandex", "soundcloud", "youtube", "all"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/music/search?q=rock&source={source}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["source"], source);
        assert!(json["results"].is_array());
    }
}
