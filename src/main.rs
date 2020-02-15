extern crate walkdir;

use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Write};
use std::error::Error;
use std::ffi::OsStr;
use std::process::exit;
use clap::{App, Arg};

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

    fn write_to(&mut self, path: &Path) {
        let file_path = path.join(&self.name);
        let mut file = match File::create(file_path) {
            Ok(f) => f,
            Err(e) => {
                println!("couldn't write playlist: {}\n{}", &self.name, e.to_string());
                return;
            }
        };

        let r = file.write(self.contents().as_bytes());

        match r {
            Ok(_t) => (),
            Err(e) => println!("Couldn't write playlist because:\n{}", e.description())
        }
    }

    fn contents(&mut self) -> String {
        let mut content = String::new(); // TODO replace with some sort of buffer

        for s in &self.songs {
            content.push_str(&String::from(s.to_str().unwrap()));
            content.push('\n');
        }

        content
    }
}

const MUSIC_FILE_EXTENSIONS: [&str; 6] = ["m4a", "mp3", "aac", "flac", "opus", "wav"];
const PLAYLIST_FILE_EXTENSIONS: [&str; 1] = ["m3u"];

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
        .get_matches();

    let root = matches.value_of("root-dir").unwrap();
    let output = matches.value_of("output-dir").unwrap();

    let root_dir = Path::new(root);
    let output_dir = Path::new(output);

    println!("indexing...");
    let indexes = index(root_dir);
    let music_index = indexes.0;
    let playlist_index = indexes.1;

    println!("searching playlists...");
    let m3u_playlist_paths = match_file_extension(&playlist_index, PLAYLIST_FILE_EXTENSIONS[0]);

    println!("localizing playlists...");
    for p in m3u_playlist_paths {
        let file_names = m3u_playlist_file_names(p);
        let playlist_name = String::from(p.file_stem().unwrap().to_str().unwrap());
        let mut playlist = m3u_playlist(&music_index, &file_names, playlist_name);

        playlist.write_to(output_dir);
    }

    println!("finished")
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
        let matching_extension = match d.path().extension() {
            Some(s) => matches_extension(s),
            None => 0,
        };

        match matching_extension {
            1 => music_index.push(d.into_path()),
            2 => playlist_index.push(d.into_path()),
            _ => (),
        }
    }

    (music_index, playlist_index)
}

fn m3u_playlist_file_names(playlist_path: &Path) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    let mut file = File::open(playlist_path).unwrap();
    let mut contents = String::new();

    let r = file.read_to_string(&mut contents);

    if r.is_err() {
        return results;
    }

    for l in contents.lines() {
        if !l.starts_with("#EXT") {
            let path = Path::new(l);
            let name = path.file_name().unwrap().to_str().unwrap();
            results.push(String::from(name))
        }
    }

    results
}

fn m3u_playlist(index: &Vec<PathBuf>, file_names: &Vec<String>, name: String) -> Playlist {
    let mut playlist = Playlist::new(name);

    for f in file_names {
        let file_path = match_file(&index, f);

        match file_path {
            Some(s) => playlist.add(PathBuf::from(s)),
            None => (),
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
fn match_file<'index>(index: &'index Vec<PathBuf>, file_name: &str) -> Option<&'index PathBuf> {
    for p in index {
        if p.ends_with(file_name) {
            return Some(p);
        }
    }

    return None;
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