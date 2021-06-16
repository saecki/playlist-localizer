use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{crate_authors, crate_version, App, AppSettings, Arg, ValueHint};
use clap_generate::generate;
use clap_generate::generators::{Bash, Elvish, Fish, PowerShell, Zsh};
use walkdir::WalkDir;

use crate::playlist::Playlist;

mod metadata;
mod playlist;

const BIN_NAME: &str = "playlist_localizer";

const MUSIC_EXTENSIONS: [&str; 7] = ["aac", "flac", "m4a", "m4b", "mp3", "ogg", "opus"];
const PLAYLIST_EXTENSIONS: [&str; 1] = ["m3u"];
const PLAYLIST_EXTENSION_M3U: &str = "m3u";

const BASH: &str = "bash";
const ELVISH: &str = "elvish";
const FISH: &str = "fish";
const PWRSH: &str = "powershell";
const ZSH: &str = "zsh";

fn main() {
    let mut app = App::new("playlist localizer")
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::ColorAuto)
        .version(crate_version!())
        .author(crate_authors!())
        .about("Finds the local paths to your playlists' songs.")
        .arg(
            Arg::new("music-dir")
                .short('m')
                .long("music-dir")
                .about("The directory which will be searched for playlists and music files")
                .takes_value(true)
                .required_unless_present("generate-completion")
                .conflicts_with("generate-completion")
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("output-dir")
                .short('o')
                .long("output-dir")
                .about("The output directory which files will be written to")
                .takes_value(true)
                .required_unless_present("generate-completion")
                .conflicts_with("generate-completion")
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .about("The wanted output format")
                .takes_value(true)
                .possible_values(&["m3u", "extm3u"]),
        )
        .arg(
            Arg::new("output-file-extension")
                .short('e')
                .long("output-file-extension")
                .value_name("extension")
                .about("The file extension of the output playlist files")
                .takes_value(true),
        )
        .arg(
            Arg::new("generate-completion")
                .short('g')
                .long("generate-completion")
                .value_name("shell")
                .about("Generates a completion script for the specified shell")
                .conflicts_with("music-dir")
                .takes_value(true)
                .possible_values(&[BASH, ELVISH, FISH, PWRSH, ZSH]),
        );

    let matches = app.clone().get_matches();

    let generate_completion = matches.value_of("generate-completion");

    if let Some(shell) = generate_completion {
        let mut stdout = std::io::stdout();
        match shell {
            BASH => generate::<Bash, _>(&mut app, BIN_NAME, &mut stdout),
            ELVISH => generate::<Elvish, _>(&mut app, BIN_NAME, &mut stdout),
            FISH => generate::<Fish, _>(&mut app, BIN_NAME, &mut stdout),
            ZSH => generate::<Zsh, _>(&mut app, BIN_NAME, &mut stdout),
            PWRSH => generate::<PowerShell, _>(&mut app, BIN_NAME, &mut stdout),
            _ => unreachable!(),
        }
        exit(0);
    }

    let music_dir = PathBuf::from(matches.value_of("music-dir").unwrap());
    let output_dir = PathBuf::from(matches.value_of("output-dir").unwrap());
    let format = matches.value_of("format").unwrap_or("m3u");
    let extension = matches.value_of("output-file-extension").unwrap_or("");

    println!("indexing...");
    let (music_index, playlist_index) = index(&music_dir);

    println!("searching playlists...");
    let m3u_playlists = match_file_extension(&playlist_index, PLAYLIST_EXTENSION_M3U);

    println!("localizing songs...");
    let playlists: Vec<Playlist> = m3u_playlists
        .into_iter()
        .filter_map(|p| {
            let file_paths = m3u_playlist_paths(p);
            let name = p.file_stem().and_then(|s| s.to_str());

            name.map(|s| m3u_playlist(&music_index, &file_paths, s.to_string()))
        })
        .collect();

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
                match Extension::of(extension) {
                    Extension::Music => music_index.push(d.into_path()),
                    Extension::Playlist => playlist_index.push(d.into_path()),
                    Extension::Unknown => (),
                }
            }
        });

    (music_index, playlist_index)
}

fn m3u_playlist_paths(playlist_path: &Path) -> Vec<PathBuf> {
    let mut results: Vec<PathBuf> = Vec::new();
    if let Ok(contents) = std::fs::read_to_string(playlist_path) {
        for l in contents.lines() {
            if !l.starts_with("#EXT") {
                results.push(platform_path(l));
            }
        }
    }

    results
}

fn m3u_playlist<'a>(index: &'a [PathBuf], file_paths: &[PathBuf], name: String) -> Playlist<'a> {
    let songs = file_paths
        .iter()
        .filter_map(|p| match_file(index, p))
        .collect();

    Playlist::new(name, songs)
}

#[derive(Clone, Copy, Debug)]
enum Extension {
    Music,
    Playlist,
    Unknown,
}

impl Extension {
    #[inline]
    fn of(s: &OsStr) -> Extension {
        for e in MUSIC_EXTENSIONS.iter() {
            if s.eq(*e) {
                return Extension::Music;
            }
        }
        for e in PLAYLIST_EXTENSIONS.iter() {
            if s.eq(*e) {
                return Extension::Playlist;
            }
        }

        Extension::Unknown
    }
}

#[derive(Debug, Default, Clone)]
struct FileMatch<'a> {
    extension_matches: bool,
    matching_components: usize,
    path: Option<&'a Path>,
}

#[inline]
fn match_file<'index>(index: &'index [PathBuf], file_path: &PathBuf) -> Option<&'index Path> {
    let mut best_result = FileMatch::default();

    for local_path in index {
        match (
            file_path.file_stem(),
            file_path.extension(),
            local_path.file_stem(),
            local_path.extension(),
        ) {
            (Some(ls), Some(le), Some(s), Some(e)) => {
                if ls == s {
                    let mut fm = FileMatch {
                        path: Some(&local_path),
                        ..Default::default()
                    };
                    let local_components = local_path.components().rev().skip(1);
                    let components = file_path.components().rev().skip(1);
                    for (i, (lc, c)) in local_components.zip(components).enumerate() {
                        if lc != c {
                            fm.matching_components = i;
                            break;
                        }
                    }

                    fm.extension_matches = le == e;

                    if best_result.matching_components < fm.matching_components {
                        best_result = fm;
                    } else if best_result.matching_components == fm.matching_components {
                        if !best_result.extension_matches && fm.extension_matches {
                            best_result = fm;
                        }
                    }
                }
            }
            _ => continue,
        }
    }

    best_result.path
}

fn match_file_extension<'index>(index: &'index [PathBuf], extension: &str) -> Vec<&'index Path> {
    let mut results: Vec<&Path> = Vec::new();

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
