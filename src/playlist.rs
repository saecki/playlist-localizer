use std::fs;
use std::path::Path;
use std::str::FromStr;

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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum PlaylistFormat {
    #[default]
    M3u,
    Extm3u,
}

impl FromStr for PlaylistFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "m3u" => Ok(PlaylistFormat::M3u),
            "extm3u" => Ok(PlaylistFormat::Extm3u),
            _ => Err("Unknown playlist format"),
        }
    }
}

impl<'a> Playlist<'a> {
    pub fn new(name: String, songs: Vec<&'a Path>) -> Self {
        Playlist { name, songs }
    }

    pub fn write_to(&mut self, path: &Path, format: PlaylistFormat, extension: &str) {
        let file_path = path.join(&self.name).with_extension(extension);

        let r = fs::write(
            file_path,
            match format {
                PlaylistFormat::M3u => self.to_m3u(),
                PlaylistFormat::Extm3u => self.to_extm3u(),
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
