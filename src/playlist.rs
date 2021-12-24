use std::fs;
use std::path::Path;

use crate::metadata::SongMetadata;

const EXTM3U_HEADER: &str = "#EXTM3U";
const EXTM3U_SONG_PATTERN: &str = "
#EXTINF:<duration>,<artist> - <title>
<path>";

#[derive(Debug)]
pub struct Playlist<'a> {
    name: String,
    songs: Vec<&'a Path>,
}

impl<'a> Playlist<'a> {
    pub fn new(name: String, songs: Vec<&'a Path>) -> Self {
        Playlist { name, songs }
    }

    pub fn write_to(&mut self, path: &Path, format: &str, extension: &str) {
        let file_path = path.join(&self.name).with_extension(extension);

        let r = fs::write(
            file_path,
            match format {
                "extm3u" => self.to_extm3u(),
                _ => self.to_m3u(),
            },
        );

        match r {
            Ok(_) => (),
            Err(e) => println!("Couldn't write playlist because:\n{:?}", e),
        }
    }

    pub fn to_m3u(&self) -> String {
        let mut content = String::new();

        for p in &self.songs {
            if let Some(s) = p.to_str() {
                content.push_str(s);
                content.push('\n');
            }
        }

        content
    }

    pub fn to_extm3u(&self) -> String {
        let mut content = String::from(EXTM3U_HEADER);

        for i in 0..self.songs.len() {
            let song_metadata = SongMetadata::from(&self.songs[i]);
            let song = EXTM3U_SONG_PATTERN
                .replace("<duration>", &song_metadata.duration.to_string())
                .replace("<artist>", &song_metadata.artist)
                .replace("<title>", &song_metadata.title)
                .replace("<path>", self.songs[i].to_str().unwrap_or(""));

            content.push_str(&song);
        }

        content
    }
}
