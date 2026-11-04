use crate::music::models::{RecommendationItem, Track};
use std::collections::{HashMap, HashSet};

/// Recommendation Engine using listening history frequency, favorite tracks/artists,
/// completion rate, and penalties for skipped tracks.
pub struct RecommendationEngine;

impl RecommendationEngine {
    pub fn score_tracks(
        candidates: Vec<Track>,
        liked_tracks: &HashSet<String>,
        liked_artists: &HashSet<String>,
        play_counts: &HashMap<String, usize>,
        completion_counts: &HashMap<String, usize>,
        skip_counts: &HashMap<String, usize>,
        artist_skips: &HashMap<String, usize>,
        limit: usize,
    ) -> Vec<RecommendationItem> {
        let mut scored_items: Vec<RecommendationItem> = candidates
            .into_iter()
            .map(|track| {
                let mut score = 50.0f64;
                let mut reasons: Vec<String> = Vec::new();

                let is_liked = liked_tracks.contains(&track.id);
                let artist_lower = track.artist.to_lowercase();
                let artist_is_liked = liked_artists.contains(&artist_lower);

                // 1. Favorites bonus
                if is_liked {
                    score += 35.0;
                    reasons.push("Liked track (+35)".to_string());
                } else if artist_is_liked {
                    score += 15.0;
                    reasons.push(format!("Favorite artist {} (+15)", track.artist));
                }

                // 2. Play history frequency bonus (up to +20)
                let plays = play_counts.get(&track.id).copied().unwrap_or(0);
                if plays > 0 {
                    let play_bonus = (plays as f64 * 4.0).min(20.0);
                    score += play_bonus;
                    reasons.push(format!("Played {} times (+{:.0})", plays, play_bonus));
                }

                // 3. Completion rate bonus
                let completions = completion_counts.get(&track.id).copied().unwrap_or(0);
                if completions > 0 {
                    score += 10.0;
                    reasons.push("Completed playback (+10)".to_string());
                }

                // 4. Skip penalty (significant negative weight)
                let skips = skip_counts.get(&track.id).copied().unwrap_or(0);
                if skips > 0 {
                    let penalty = (skips as f64 * 25.0).min(50.0);
                    score -= penalty;
                    reasons.push(format!("Skipped {} times (-{:.0})", skips, penalty));
                }

                // 5. Artist skip penalty
                let artist_skip_total = artist_skips.get(&artist_lower).copied().unwrap_or(0);
                if artist_skip_total > skips && artist_skip_total >= 2 {
                    score -= 10.0;
                    reasons.push("Artist has multiple skipped tracks (-10)".to_string());
                }

                // 6. Novelty exploration bonus for unplayed, non-skipped tracks
                if plays == 0 && skips == 0 && !is_liked {
                    score += 8.0;
                    reasons.push("Discovery track (+8)".to_string());
                }

                if reasons.is_empty() {
                    reasons.push("Catalog recommendation".to_string());
                }

                RecommendationItem {
                    track,
                    score: (score * 10.0).round() / 10.0,
                    reason: reasons.join(", "),
                }
            })
            .collect();

        // Sort by score descending
        scored_items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored_items.truncate(limit);

        scored_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_scoring_likes_and_skips() {
        let t1 = Track::new("Favorite Song", "Top Artist", "local");
        let t2 = Track::new("Skipped Song", "Bad Artist", "local");
        let t3 = Track::new("New Discovery", "Indie Artist", "local");

        let mut liked_tracks = HashSet::new();
        liked_tracks.insert(t1.id.clone());

        let mut liked_artists = HashSet::new();
        liked_artists.insert("top artist".to_string());

        let play_counts = HashMap::new();
        let completion_counts = HashMap::new();

        let mut skip_counts = HashMap::new();
        skip_counts.insert(t2.id.clone(), 2);

        let artist_skips = HashMap::new();

        let candidates = vec![t1.clone(), t2.clone(), t3.clone()];
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

        assert_eq!(results.len(), 3);
        // t1 (liked) should be top
        assert_eq!(results[0].track.id, t1.id);
        assert!(results[0].score > 80.0);

        // t3 (discovery) should be middle
        assert_eq!(results[1].track.id, t3.id);

        // t2 (skipped twice) should be penalized heavily
        assert_eq!(results[2].track.id, t2.id);
        assert!(results[2].score < 10.0);
    }
}
