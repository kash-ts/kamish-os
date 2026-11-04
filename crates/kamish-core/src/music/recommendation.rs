use crate::music::models::{RecommendationItem, Track};
use std::collections::{HashMap, HashSet};

/// Recommendation Engine delegating recommendations to YouTube Music.
/// Local scoring math and penalty calculations are removed in favor of YouTube's algorithms.
pub struct RecommendationEngine;

impl RecommendationEngine {
    pub fn score_tracks(
        candidates: Vec<Track>,
        _liked_tracks: &HashSet<String>,
        _liked_artists: &HashSet<String>,
        _play_counts: &HashMap<String, usize>,
        _completion_counts: &HashMap<String, usize>,
        _skip_counts: &HashMap<String, usize>,
        _artist_skips: &HashMap<String, usize>,
        limit: usize,
    ) -> Vec<RecommendationItem> {
        let scored_items: Vec<RecommendationItem> = candidates
            .into_iter()
            .take(limit)
            .map(|track| RecommendationItem {
                track,
                score: 100.0,
                reason: "YouTube Music recommendation".to_string(),
            })
            .collect();

        scored_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_scoring_delegates_to_youtube() {
        let t1 = Track::new("Favorite Song", "Top Artist", "youtube");
        let t2 = Track::new("Discovery Song", "Indie Artist", "youtube");

        let liked_tracks = HashSet::new();
        let liked_artists = HashSet::new();
        let play_counts = HashMap::new();
        let completion_counts = HashMap::new();
        let skip_counts = HashMap::new();
        let artist_skips = HashMap::new();

        let candidates = vec![t1.clone(), t2.clone()];
        let results = RecommendationEngine::score_tracks(
            candidates,
            &liked_tracks,
            &liked_artists,
            &play_counts,
            &completion_counts,
            &skip_counts,
            &artist_skips,
            10,
        );

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].track.id, t1.id);
        assert_eq!(results[0].reason, "YouTube Music recommendation");
        assert_eq!(results[1].track.id, t2.id);
        assert_eq!(results[1].reason, "YouTube Music recommendation");
    }
}
