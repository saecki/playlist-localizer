use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{App, Arg};
use id3;
use walkdir::WalkDir;

const MUSIC_FILE_EXTENSIONS: [&str; 7] = [
    "aac",
    "flac",
    "m4a",
    "m4b",
    "mp3",
    "ogg",
    "opus",
];
const PLAYLIST_FILE_EXTENSIONS: [&str; 1] = ["m3u"];
const EXTM3U_SONG_PATTERN: &str = "
#EXTINF:<duration>,<artist> - <title>
<path>";

struct Playlist {
    name: String,
    songs: Vec<PathBuf>,
}

impl Playlist {
    fn new(name: String) -> Playlist {
        Playlist { name, songs: Vec::new() }
    }

    fn add(&mut self, song: PathBuf) {
        self.songs.push(song);
    }

    fn write_to(&mut self, path: &Path, format: &str, extension: &str) {
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
            Err(e) => println!("Couldn't write playlist because:\n{:?}", e)
        }
    }

    fn to_m3u(&self) -> String {
        let mut content = String::new();

        for p in &self.songs {
            if let Some(s) = p.to_str() {
                content.push_str(s);
                content.push('\n');
            }
        }

        content
    }

    fn to_extm3u(&self) -> String {
        let mut content = "[".to_string();

        for i in 0..self.songs.len() {
            let song_metadata = SongMetadata::from(&self.songs[i]);
            let song = EXTM3U_SONG_PATTERN
                .replace("<duration>", &song_metadata.duration.to_string())
                .replace("<artist>", &song_metadata.artist)
                .replace("<title>", &song_metadata.title)
                .replace("<path>", &self.songs[i].to_str().unwrap_or(""));

            if i != 0 {
                content.push(',');
            }

            content.push_str(&song);
        }

        content.push(']');

        content
    }
}

struct SongMetadata {
    title: String,
    artist: String,
    duration: u32,
}

impl SongMetadata {
    fn new() -> SongMetadata {
        SongMetadata { title: String::new(), artist: String::new(), duration: 0_u32 }
    }

    fn from(path: &Path) -> SongMetadata {
        return if let Ok(tag) = id3::Tag::read_from_path(path) {
            SongMetadata {
                title: tag.title().unwrap_or("").to_string(),
                artist: tag.artist().unwrap_or("").to_string(),
                duration: tag.duration().unwrap_or(0),
            }
        } else if let Ok(tag) = mp4ameta::Tag::read_from_path(path) {
            SongMetadata {
                title: tag.title().unwrap_or("").to_string(),
                artist: tag.artist().unwrap_or("").to_string(),
                duration: 0,
            }
        } else {
            SongMetadata::new()
        };
    }
}

fn main() {
    let params = params();

    let root_dir = Path::new(&params.0);
    let output_dir = Path::new(&params.1);
    let format = &params.2;
    let extension = &params.3;

    println!("indexing...");
    let indexes = index(root_dir);
    let music_index = indexes.0;
    let playlist_index = indexes.1;

    println!("searching playlists...");
    let m3u_playlists = match_file_extension(&playlist_index, PLAYLIST_FILE_EXTENSIONS[0]);

    println!("localizing songs...");
    let mut playlists: Vec<Playlist> = Vec::new();

    for p in m3u_playlists {
        let file_paths = m3u_playlist_paths(p);

        if let Some(stem) = p.file_stem() {
            if let Some(name) = stem.to_str() {
                playlists.push(m3u_playlist(&music_index, &file_paths, name.to_string()));
            }
        }
    }

    println!("writing playlists...");
    for mut p in playlists {
        p.write_to(output_dir, format, extension);
    }

    println!("done")
}

fn params() -> (String, String, String, String) {
    let matches = App::new("playlist localizer")
        .version("1.0-beta")
        .author("Tobias Schmitz")
        .about("Finds the local paths to your playlists' songs.")
        .arg(Arg::with_name("root-dir")
            .short("r")
            .long("root-dir")
            .help("The directory which will be searched for playlists and music files")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("output-dir")
            .short("o")
            .long("output-dir")
            .help("The directory which the playlists will be written to")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("format")
            .short("f")
            .long("format")
            .help("The wanted output format")
            .takes_value(true)
            .possible_value("m3u")
            .possible_value("extm3u"))
        .arg(Arg::with_name("output-file-extension")
            .short("e")
            .long("output-file-extension")
            .help("The file extension of the output playlist files")
            .takes_value(true))
        .get_matches();

    let root_dir = matches.value_of("root-dir").unwrap();
    let output_dir = matches.value_of("output-dir").unwrap();
    let format = matches.value_of("format").unwrap_or("m3u");
    let extension = matches.value_of("output-file-extension").unwrap_or("");

    (root_dir.to_string(),
     output_dir.to_string(),
     format.to_string(),
     extension.to_string())
}

fn index(root_dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let abs_root_path = match root_dir.canonicalize() {
        Ok(t) => t,
        Err(e) => {
            println!("Not a valid root path: {}\n{:?}", root_dir.display(), e);
            exit(1)
        }
    };
    let mut music_index = Vec::new();
    let mut playlist_index = Vec::new();

    for d in WalkDir::new(abs_root_path).into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| match e.metadata() {
            Ok(m) => m.is_file(),
            Err(_e) => false,
        })
    {
        if let Some(extension) = d.path().extension() {
            match matches_extension(extension) {
                1 => music_index.push(d.into_path()),
                2 => playlist_index.push(d.into_path()),
                _ => (),
            }
        }
    }

    (music_index, playlist_index)
}

fn m3u_playlist_paths(playlist_path: &Path) -> Vec<PathBuf> {
    let mut results: Vec<PathBuf> = Vec::new();
    if let Ok(mut file) = File::open(playlist_path) {
        let mut contents = String::new();
        let r = file.read_to_string(&mut contents);

        if r.is_err() {
            return results;
        }

        for l in contents.lines() {
            if !l.starts_with("#EXT") {
                results.push(PathBuf::from(l));
            }
        }
    }

    results
}

fn m3u_playlist(index: &Vec<PathBuf>, file_paths: &Vec<PathBuf>, name: String) -> Playlist {
    let mut playlist = Playlist::new(name);

    for f in file_paths {
        if let Some(s) = match_file(&index, f) {
            playlist.add(PathBuf::from(s));
        }
    }

    playlist
}

#[inline]
fn matches_extension(s: &OsStr) -> i8 {
    for e in &MUSIC_FILE_EXTENSIONS {
        if s.eq(*e) {
            return 1;
        }
    }

    for e in &PLAYLIST_FILE_EXTENSIONS {
        if s.eq(*e) {
            return 2;
        }
    }

    return 0;
}

#[inline]
fn match_file<'index>(index: &'index Vec<PathBuf>, file_path: &PathBuf) -> Option<&'index PathBuf> {
    let mut best_result: (u8, Option<&PathBuf>) = (0u8, None);

    for p in index {
        let mut local_components = p.components().rev();
        let mut components = file_path.components().rev();

        let mut i = 0_u8;
        while let Some(lc) = local_components.next() {
            if let Some(c) = components.next() {
                if lc != c {
                    if i != 0 && i > best_result.0 {
                        best_result = (i, Some(p));
                    }
                    break;
                }
            }
            i += 1;
        }
    }

    best_result.1
}

fn match_file_extension<'index>(index: &'index Vec<PathBuf>, extension: &str) -> Vec<&'index PathBuf> {
    let mut results: Vec<&PathBuf> = Vec::new();

    for p in index {
        match p.extension() {
            Some(e) => if e.eq(extension) { results.push(p) },
            None => (),
        }
    }

    results
}