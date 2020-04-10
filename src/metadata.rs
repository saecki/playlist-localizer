use std::path::Path;
use id3;
use mp4ameta;

pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub duration: f64,
}

impl SongMetadata {
    pub fn new() -> SongMetadata {
        SongMetadata { title: String::new(), artist: String::new(), duration: 0.0 }
    }

    pub fn from(path: &Path) -> SongMetadata {
        return if let Ok(tag) = id3::Tag::read_from_path(path) {
            SongMetadata {
                title: tag.title().unwrap_or("").to_string(),
                artist: tag.artist().unwrap_or("").to_string(),
                duration: tag.duration().unwrap_or(0) as f64 / 1000.0,
            }
        } else if let Ok(tag) = mp4ameta::Tag::read_from_path(path) {
            SongMetadata {
                title: tag.title().unwrap_or("").to_string(),
                artist: tag.artist().unwrap_or("").to_string(),
                duration: tag.duration().unwrap_or(0.0),
            }
        } else {
            SongMetadata::new()
        };
    }
}