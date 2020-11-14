use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::exit;
use std::str::FromStr;

use clap::{App, Arg, Shell};
use walkdir::WalkDir;

use crate::playlist::Playlist;

mod metadata;
mod playlist;

const MUSIC_FILE_EXTENSIONS: [&str; 7] = ["aac", "flac", "m4a", "m4b", "mp3", "ogg", "opus"];
const PLAYLIST_FILE_EXTENSIONS: [&str; 1] = ["m3u"];

fn main() {
    let app = App::new("playlist localizer")
        .version("0.2.0")
        .author("Saecki")
        .about("Finds the local paths to your playlists' songs.")
        .arg(
            Arg::with_name("music-dir")
                .short("m")
                .long("music-dir")
                .help("The directory which will be searched for playlists and music files")
                .takes_value(true)
                .required_unless("generate-completion")
                .conflicts_with("generate-completion"),
        )
        .arg(
            Arg::with_name("output-dir")
                .short("o")
                .long("output-dir")
                .help("The directory which the playlists will be written to")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .help("The wanted output format")
                .takes_value(true)
                .possible_value("m3u")
                .possible_value("extm3u"),
        )
        .arg(
            Arg::with_name("output-file-extension")
                .short("e")
                .long("output-file-extension")
                .value_name("extension")
                .help("The file extension of the output playlist files")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("generate-completion")
                .short("g")
                .long("generate-completion")
                .value_name("shell")
                .help("Generates a completion script for the specified shell")
                .conflicts_with("music-dir")
                .takes_value(true)
                .possible_values(&Shell::variants()),
        );

    let matches = app.clone().get_matches();

    let music_dir = PathBuf::from(matches.value_of("music-dir").unwrap_or(""));
    let output_dir = PathBuf::from(matches.value_of("output-dir").unwrap());
    let format = matches.value_of("format").unwrap_or("m3u");
    let extension = matches.value_of("output-file-extension").unwrap_or("");
    let generate_completion = matches.value_of("generate-completion").unwrap_or("");

    if generate_completion != "" {
        let shell = Shell::from_str(generate_completion).unwrap();

        app.clone()
            .gen_completions("playlist_localizer", shell, output_dir);

        exit(0);
    }

    println!("indexing...");
    let (music_index, playlist_index) = index(&music_dir);

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
        p.write_to(&output_dir, format, extension);
    }

    println!("done");
}

fn index(music_dir: &PathBuf) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let abs_music_path = match canonicalize(music_dir) {
        Ok(t) => t,
        Err(e) => {
            println!(
                "Not a valid music dir path: {}\n{:?}",
                music_dir.display(),
                e
            );
            exit(1)
        }
    };
    let mut music_index = Vec::new();
    let mut playlist_index = Vec::new();

    WalkDir::new(abs_music_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| match e.metadata() {
            Ok(m) => m.is_file(),
            Err(_e) => false,
        })
        .for_each(|d| {
            if let Some(extension) = d.path().extension() {
                match matches_extension(extension) {
                    1 => music_index.push(d.into_path()),
                    2 => playlist_index.push(d.into_path()),
                    _ => (),
                }
            }
        });

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
                results.push(platform_path(l));
            }
        }
    }

    results
}

fn m3u_playlist(index: &[PathBuf], file_paths: &[PathBuf], name: String) -> Playlist {
    let mut playlist = Playlist::new(name);

    for f in file_paths {
        if let Some(s) = match_file(index, f) {
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

    0
}

#[inline]
fn match_file<'index>(index: &'index [PathBuf], file_path: &PathBuf) -> Option<&'index PathBuf> {
    let mut best_result = (0, None);

    let components: Vec<Component> = file_path.components().rev().collect();

    for p in index {
        let local_components = p.components().rev();

        for (i, lc) in local_components.enumerate() {
            if let Some(c) = components.get(0) {
                if &lc != c {
                    if i != 0 && i > best_result.0 {
                        best_result = (i, Some(p));
                    }
                    break;
                }
            }
        }
    }

    best_result.1
}

fn match_file_extension<'index>(index: &'index [PathBuf], extension: &str) -> Vec<&'index PathBuf> {
    let mut results: Vec<&PathBuf> = Vec::new();

    for p in index {
        if let Some(e) = p.extension() {
            if e.eq(extension) {
                results.push(p)
            }
        }
    }

    results
}

#[cfg(not(target_os = "windows"))]
fn canonicalize(path: &PathBuf) -> io::Result<PathBuf> {
    path.canonicalize()
}

#[cfg(target_os = "windows")]
fn canonicalize(path: &PathBuf) -> io::Result<PathBuf> {
    let string = path.canonicalize()?.display().to_string();

    Ok(PathBuf::from(string.replace("\\\\?\\", "")))
}

#[cfg(not(target_os = "windows"))]
fn platform_path(string: &str) -> PathBuf {
    let path = string.replace("\\", "/");
    PathBuf::from(path)
}

#[cfg(target_os = "windows")]
fn platform_path(string: &str) -> PathBuf {
    let path = string.replace("/", "\\");
    PathBuf::from(path)
}
