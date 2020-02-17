use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{App, Arg};
use id3::Tag;
use walkdir::WalkDir;

const MUSIC_FILE_EXTENSIONS: [&str; 3] = ["m4a", "mp3", "aac"];
const PLAYLIST_FILE_EXTENSIONS: [&str; 1] = ["m3u"];
const VOLUMIO_SONG_PATTERN: &str = "
    {
        \"service\":\"mpd\",
        \"uri\":\"<path>\",
        \"title\":\"<title>\",
        \"artist\":\"<artist>\",
        \"album\":\"<album>\",
        \"albumart\":\"<albumart>\"
    }";


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
        let mut file = match File::create(file_path) {
            Ok(f) => f,
            Err(e) => {
                println!("couldn't write playlist: {}\n{}", &self.name, e.to_string());
                return;
            }
        };

        let r = file.write(
            match format {
                "volumio" => self.to_volumio(),
                _ => self.to_m3u(),
            }.as_bytes()
        );

        match r {
            Ok(_t) => (),
            Err(e) => println!("Couldn't write playlist because:\n{}", e.description())
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

    fn to_volumio(&self) -> String {
        let mut content = "[".to_string();

        for i in 0..self.songs.len() {
            let tag = match Tag::read_from_path(&self.songs[i]) {
                Ok(t) => t,
                Err(_e) => {
                    println!("couldn't read tag: {}", _e);
                    Tag::new()
                }
            };

            let song = VOLUMIO_SONG_PATTERN
                .replace("<path>", &self.songs[i].to_str().unwrap_or(""))
                .replace("<title>", tag.title().unwrap_or(""))
                .replace("<artist>", tag.artist().unwrap_or(""))
                .replace("<album>", tag.album().unwrap_or(""))
                .replace("<albumart>", "");

            if i != 0 {
                content.push(',');
            }

            content.push_str(&song);
        }

        content.push(']');

        content
    }
}

fn main() {
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
            .possible_value("volumio"))
        .arg(Arg::with_name("output-file-extension")
            .short("e")
            .long("output-file-extension")
            .help("The file extension of the output playlist files")
            .takes_value(true))
        .get_matches();

    let root = matches.value_of("root-dir").unwrap();
    let output = matches.value_of("output-dir").unwrap();
    let format = matches.value_of("format").unwrap_or("m3u");
    let extension = matches.value_of("output-file-extension").unwrap_or("");

    let root_dir = Path::new(root);
    let output_dir = Path::new(output);

    println!("indexing...");
    let indexes = index(root_dir);
    let music_index = indexes.0;
    let playlist_index = indexes.1;

    println!("searching playlists...");
    let m3u_playlist_paths = match_file_extension(&playlist_index, PLAYLIST_FILE_EXTENSIONS[0]);

    println!("localizing songs...");
    let mut playlists: Vec<Playlist> = Vec::new();

    for p in m3u_playlist_paths {
        let file_paths = m3u_playlist_path(p);

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

fn index(root_dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let abs_root_path = match root_dir.canonicalize() {
        Ok(t) => t,
        Err(e) => {
            println!("Not a valid root path: {}\n{}", root_dir.display(), e.to_string());
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

fn m3u_playlist_path(playlist_path: &Path) -> Vec<PathBuf> {
    let mut results: Vec<PathBuf> = Vec::new();
    if let Ok(mut file) = File::open(playlist_path) {
        let mut contents = String::new();
        let r = file.read_to_string(&mut contents);

        if r.is_err() {
            return results;
        }

        for l in contents.lines() {
            if !l.starts_with("#EXT") {
                results.push(PathBuf::from(l))
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