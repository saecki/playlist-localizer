use std::path::Path;

#[derive(Debug, Default)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub duration: u64,
}

impl<T: AsRef<Path>> From<T> for SongMetadata {
    fn from(path: T) -> Self {
        if let Ok(tag) = id3::Tag::read_from_path(path.as_ref()) {
            Self {
                title: tag.title().unwrap_or("").to_string(),
                artist: tag.artist().unwrap_or("").to_string(),
                duration: tag.duration().unwrap_or(0) as u64 / 1000,
            }
        } else if let Ok(mut tag) = mp4ameta::Tag::read_from_path(path.as_ref()) {
            Self {
                title: tag.take_title().unwrap_or_default(),
                artist: tag.take_artist().unwrap_or_default(),
                duration: tag.duration().map(|d| d.as_secs()).unwrap_or(0),
            }
        } else {
            Self::default()
        }
    }
}
