use crate::error::WebResult;
use crate::state::WebState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use kamish_ipc::CoreRequest;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub query: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PlayRequest {
    pub track_id: Option<String>,
    pub query: Option<String>,
    pub uri: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LimitQuery {
    pub limit: Option<usize>,
}

pub async fn search_handler(
    State(state): State<WebState>,
    Query(query): Query<SearchQuery>,
) -> WebResult<impl IntoResponse> {
    let q = query.q.or(query.query).unwrap_or_default();
    let source = query.source;
    let limit = query.limit.unwrap_or(20);

    let results = state
        .call_core(CoreRequest::SearchMusic {
            query: q.clone(),
            source: source.clone(),
            limit,
        })
        .await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "query": q,
            "source": source.unwrap_or_else(|| "all".to_string()),
            "results": results
        })),
    ))
}

pub async fn get_current_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let current = state.call_core(CoreRequest::GetPlaybackState).await?;
    Ok((StatusCode::OK, Json(current)))
}

pub async fn play_handler(
    State(state): State<WebState>,
    Json(payload): Json<PlayRequest>,
) -> WebResult<impl IntoResponse> {
    let title = payload
        .title
        .or(payload.query)
        .or(payload.track_id)
        .unwrap_or_else(|| "Unknown Track".to_string());

    let track = state
        .call_core(CoreRequest::PlayTrack {
            title,
            uri: payload.uri,
            artist: payload.artist,
        })
        .await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "playing",
            "track": track
        })),
    ))
}

pub async fn pause_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let resp = state.call_core(CoreRequest::Pause).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn resume_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let resp = state.call_core(CoreRequest::Resume).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn stop_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let resp = state.call_core(CoreRequest::Stop).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn skip_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let resp = state.call_core(CoreRequest::Next).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn get_history_handler(
    State(state): State<WebState>,
    Query(query): Query<LimitQuery>,
) -> WebResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(50);
    let history = state
        .call_core(CoreRequest::GetMusicHistory { limit })
        .await?;
    Ok((StatusCode::OK, Json(history)))
}

pub async fn get_recommendations_handler(
    State(state): State<WebState>,
    Query(query): Query<LimitQuery>,
) -> WebResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(20);
    let recs = state
        .call_core(CoreRequest::GetRecommendations { limit })
        .await?;
    Ok((StatusCode::OK, Json(recs)))
}

pub async fn get_youtube_auth_status(
    State(state): State<WebState>,
) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::GetYouTubeAuthStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}

pub async fn start_youtube_auth(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let flow = state.call_core(CoreRequest::StartYouTubeAuth).await?;
    Ok((StatusCode::OK, Json(flow)))
}

pub async fn poll_youtube_auth(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::PollYouTubeAuth).await?;
    Ok((StatusCode::OK, Json(status)))
}
