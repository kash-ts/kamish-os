use crate::common::{AppError, AppResult, AppState};
use crate::music::models::Track;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub query: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
}

pub async fn search_handler(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<impl IntoResponse> {
    let q = query.q.or(query.query).unwrap_or_default();
    let source = query.source.unwrap_or_else(|| "all".to_string());
    let limit = query.limit.unwrap_or(20);

    let results = state
        .media
        .search(&q, &source, limit, Some(&state.db))
        .await;

    Ok((
        StatusCode::OK,
        Json(json!({
            "query": q,
            "source": source,
            "count": results.len(),
            "results": results
        })),
    ))
}

pub async fn get_current_handler(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let playback_state = state.media.get_playback_state().await;
    Ok((StatusCode::OK, Json(playback_state)))
}

#[derive(Debug, Deserialize, Default)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
}

pub async fn get_history_handler(
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> AppResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(50);
    let history = state
        .media
        .get_history(&state.db, limit)
        .await
        .map_err(AppError::Internal)?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "count": history.len(),
            "history": history
        })),
    ))
}

#[derive(Debug, Deserialize, Default)]
pub struct FavoritesQuery {
    pub limit: Option<i64>,
}

pub async fn get_favorites_handler(
    State(state): State<AppState>,
    Query(query): Query<FavoritesQuery>,
) -> AppResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(50);
    let favorites = state
        .media
        .get_favorites(&state.db, limit)
        .await
        .map_err(AppError::Internal)?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "count": favorites.len(),
            "favorites": favorites
        })),
    ))
}

#[derive(Debug, Deserialize, Default)]
pub struct PlayRequest {
    pub track_id: Option<String>,
    pub query: Option<String>,
    pub uri: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
}

pub async fn play_handler(
    State(state): State<AppState>,
    Json(payload): Json<PlayRequest>,
) -> AppResult<impl IntoResponse> {
    let track = if let Some(ref title) = payload.title {
        let mut t = Track::new(
            title,
            payload.artist.as_deref().unwrap_or("Kamish Artist"),
            "manual",
        );
        t.uri = payload.uri;
        if let Some(tid) = payload.track_id {
            t.id = tid;
        }
        state.media.play_track(t.clone(), Some(&state.db)).await;
        t
    } else {
        let query_param = payload.track_id.as_deref().or(payload.query.as_deref());
        state
            .media
            .play_by_query_or_uri(query_param, payload.uri.as_deref(), Some(&state.db))
            .await
    };

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "playing",
            "track": track
        })),
    ))
}

pub async fn pause_handler(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    state.media.pause(Some(&state.db)).await;
    Ok((StatusCode::OK, Json(json!({ "status": "paused" }))))
}

pub async fn resume_handler(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    state.media.resume(Some(&state.db)).await;
    Ok((StatusCode::OK, Json(json!({ "status": "playing" }))))
}

pub async fn stop_handler(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    state.media.stop(Some(&state.db)).await;
    Ok((StatusCode::OK, Json(json!({ "status": "stopped" }))))
}

#[derive(Debug, Deserialize, Default)]
pub struct LikeRequest {
    pub track_id: Option<String>,
    pub liked: Option<bool>,
}

pub async fn like_handler(
    State(state): State<AppState>,
    Json(payload): Json<LikeRequest>,
) -> AppResult<impl IntoResponse> {
    let track_id = if let Some(tid) = payload.track_id {
        tid
    } else if let Some(cur) = state.media.get_current().await {
        cur.id
    } else {
        return Err(AppError::Config(
            "No track specified and nothing currently playing".to_string(),
        ));
    };

    let liked = if let Some(explicit) = payload.liked {
        state
            .media
            .set_like(&track_id, explicit, &state.db)
            .await
            .map_err(AppError::Internal)?;
        explicit
    } else {
        state
            .media
            .toggle_like(&track_id, &state.db)
            .await
            .map_err(AppError::Internal)?
    };

    // If track is a YouTube / YouTube Music track (e.g. "yt:<videoId>"), also sync like with YT Music
    if track_id.starts_with("yt:") {
        let video_id = &track_id[3..];
        let rating = if liked { "LIKE" } else { "INDIFFERENT" };
        state.media.rate_ytmusic(video_id, rating).await;
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "track_id": track_id,
            "liked": liked
        })),
    ))
}

#[derive(Debug, Deserialize, Default)]
pub struct SkipRequest {
    pub track_id: Option<String>,
    pub position_secs: Option<u64>,
}

pub async fn skip_handler(
    State(state): State<AppState>,
    Json(payload): Json<SkipRequest>,
) -> AppResult<impl IntoResponse> {
    let skipped_id = state
        .media
        .skip(payload.position_secs, Some(&state.db))
        .await;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "skipped",
            "track_id": skipped_id
        })),
    ))
}

#[derive(Debug, Deserialize, Default)]
pub struct RecommendationsQuery {
    pub limit: Option<usize>,
}

pub async fn get_recommendations_handler(
    State(state): State<AppState>,
    Query(query): Query<RecommendationsQuery>,
) -> AppResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(10);
    let recommendations = state
        .media
        .get_recommendations(&state.db, limit)
        .await
        .map_err(AppError::Internal)?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "count": recommendations.len(),
            "recommendations": recommendations
        })),
    ))
}

pub async fn get_radio_handler(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let stations = state.media.get_radio_stations();
    Ok((
        StatusCode::OK,
        Json(json!({
            "count": stations.len(),
            "stations": stations
        })),
    ))
}
