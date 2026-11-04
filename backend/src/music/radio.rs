use crate::music::models::{RadioStation, Track};

pub struct RadioService;

impl RadioService {
    /// Returns curated popular internet radio stations.
    pub fn get_curated_stations() -> Vec<RadioStation> {
        vec![
            RadioStation {
                id: "chillhop".to_string(),
                name: "Chillhop Radio".to_string(),
                genre: "Lofi / Chillhop".to_string(),
                stream_uri: "https://streams.ilovemusic.de/iloveradio17.mp3".to_string(),
                homepage_url: Some("https://chillhop.com".to_string()),
                bitrate: Some(192),
                favicon_url: Some("https://chillhop.com/favicon.ico".to_string()),
            },
            RadioStation {
                id: "somafm-groovesalad".to_string(),
                name: "SomaFM Groove Salad".to_string(),
                genre: "Ambient / Downtempo".to_string(),
                stream_uri: "https://ice1.somafm.com/groovesalad-128-mp3".to_string(),
                homepage_url: Some("https://somafm.com/groovesalad/".to_string()),
                bitrate: Some(128),
                favicon_url: Some("https://somafm.com/favicon.ico".to_string()),
            },
            RadioStation {
                id: "somafm-secretagent".to_string(),
                name: "SomaFM Secret Agent".to_string(),
                genre: "Spy / Lounge".to_string(),
                stream_uri: "https://ice1.somafm.com/secretagent-128-mp3".to_string(),
                homepage_url: Some("https://somafm.com/secretagent/".to_string()),
                bitrate: Some(128),
                favicon_url: Some("https://somafm.com/favicon.ico".to_string()),
            },
            RadioStation {
                id: "radioparadise".to_string(),
                name: "Radio Paradise".to_string(),
                genre: "Eclectic Rock / Indie".to_string(),
                stream_uri: "https://stream.radioparadise.com/mp3-128".to_string(),
                homepage_url: Some("https://radioparadise.com".to_string()),
                bitrate: Some(128),
                favicon_url: Some("https://radioparadise.com/favicon.ico".to_string()),
            },
            RadioStation {
                id: "classicfm".to_string(),
                name: "Classic FM".to_string(),
                genre: "Classical".to_string(),
                stream_uri: "https://media-ice.musicradio.com/ClassicFMMP3".to_string(),
                homepage_url: Some("https://www.classicfm.com".to_string()),
                bitrate: Some(128),
                favicon_url: Some("https://www.classicfm.com/favicon.ico".to_string()),
            },
            RadioStation {
                id: "jazzradio".to_string(),
                name: "Jazz Radio".to_string(),
                genre: "Jazz / Blues".to_string(),
                stream_uri: "https://jazzradio.ice.infomaniak.ch/jazzradio-high.mp3".to_string(),
                homepage_url: Some("https://www.jazzradio.fr".to_string()),
                bitrate: Some(128),
                favicon_url: Some("https://www.jazzradio.fr/favicon.ico".to_string()),
            },
            RadioStation {
                id: "nightwave-plaza".to_string(),
                name: "Nightwave Plaza".to_string(),
                genre: "Vaporwave / Future Funk".to_string(),
                stream_uri: "https://radio.plaza.one/mp3".to_string(),
                homepage_url: Some("https://plaza.one".to_string()),
                bitrate: Some(128),
                favicon_url: Some("https://plaza.one/favicon.ico".to_string()),
            },
            RadioStation {
                id: "bbc-world-service".to_string(),
                name: "BBC World Service".to_string(),
                genre: "News / Talk".to_string(),
                stream_uri: "https://stream.live.vc.bbcmedia.co.uk/bbc_world_service".to_string(),
                homepage_url: Some("https://www.bbc.co.uk/worldserviceradio".to_string()),
                bitrate: Some(96),
                favicon_url: Some("https://www.bbc.com/favicon.ico".to_string()),
            },
        ]
    }

    /// Search radio stations by name or genre.
    pub fn search_stations(query: &str) -> Vec<RadioStation> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Self::get_curated_stations();
        }

        Self::get_curated_stations()
            .into_iter()
            .filter(|s| s.name.to_lowercase().contains(&q) || s.genre.to_lowercase().contains(&q))
            .collect()
    }

    /// Convert a radio station into a playable Track.
    pub fn station_to_track(station: &RadioStation) -> Track {
        station.to_track()
    }
}
