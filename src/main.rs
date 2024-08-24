use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;

use clap::{crate_authors, crate_version, value_parser, Arg, ColorChoice, Command, ValueHint};
use clap_complete::generate;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use walkdir::WalkDir;

use crate::playlist::{Playlist, PlaylistFormat};

mod metadata;
mod playlist;

const BIN_NAME: &str = "playlist-localizer";

const MUSIC_EXTENSIONS: [&str; 7] = ["aac", "flac", "m4a", "m4b", "mp3", "ogg", "opus"];
const PLAYLIST_EXTENSIONS: [&str; 1] = ["m3u"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shell {
    Bash,
    Elvish,
    Fish,
    Pwrsh,
    Zsh,
}

impl FromStr for Shell {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bash" => Ok(Shell::Bash),
            "elvish" => Ok(Shell::Elvish),
            "fish" => Ok(Shell::Fish),
            "powershell" => Ok(Shell::Pwrsh),
            "zsh" => Ok(Shell::Zsh),
            _ => Err("Unknown shell"),
        }
    }
}

fn main() {
    let mut app = Command::new("playlist localizer")
        .color(ColorChoice::Auto)
        .version(crate_version!())
        .author(crate_authors!())
        .about("Finds the local paths to your playlists' songs.")
        .arg(
            Arg::new("music-dir")
                .short('m')
                .long("music-dir")
                .help("The directory which will be searched for playlists and music files")
                .num_args(1)
                .required_unless_present("generate-completion")
                .conflicts_with("generate-completion")
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("output-dir")
                .short('o')
                .long("output-dir")
                .help("The output directory which files will be written to")
                .num_args(1)
                .required_unless_present("generate-completion")
                .conflicts_with("generate-completion")
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .help("The wanted output format")
                .num_args(0..1)
                .default_value("m3u")
                .value_parser(value_parser!(PlaylistFormat)),
        )
        .arg(
            Arg::new("output-file-extension")
                .short('e')
                .long("output-file-extension")
                .value_name("extension")
                .help("The file extension of the output playlist files")
                .num_args(1),
        )
        .arg(
            Arg::new("generate-completion")
                .short('g')
                .long("generate-completion")
                .value_name("shell")
                .help("Generates a completion script for the specified shell")
                .conflicts_with("music-dir")
                .num_args(1)
                .value_parser(value_parser!(Shell)),
        );

    let matches = app.clone().get_matches();

    let generate_completion = matches.get_one("generate-completion");

    if let Some(shell) = generate_completion {
        let mut stdout = std::io::stdout();
        match shell {
            Shell::Bash => generate(Bash, &mut app, BIN_NAME, &mut stdout),
            Shell::Elvish => generate(Elvish, &mut app, BIN_NAME, &mut stdout),
            Shell::Fish => generate(Fish, &mut app, BIN_NAME, &mut stdout),
            Shell::Zsh => generate(Zsh, &mut app, BIN_NAME, &mut stdout),
            Shell::Pwrsh => generate(PowerShell, &mut app, BIN_NAME, &mut stdout),
        }
        exit(0);
    }

    let music_dir = matches.get_one::<String>("music-dir").unwrap();
    let output_dir = matches.get_one::<String>("output-dir").unwrap();
    let format = matches.get_one("format").copied().unwrap();
    let extension = matches.get_one("output-file-extension").unwrap_or(&"");

    println!("indexing...");
    let (music_index, playlist_index) = index(music_dir.as_ref());

    println!("localizing songs...");
    let playlists: Vec<Playlist> = playlist_index
        .iter()
        .filter_map(|p| {
            let file_paths = m3u_playlist_paths(p);
            let name = p.file_stem().and_then(|s| s.to_str());

            name.map(|s| m3u_playlist(&music_index, &file_paths, s.to_string()))
        })
        .collect();

    println!("writing playlists...");
    for mut p in playlists {
        p.write_to(output_dir.as_ref(), format, extension);
    }

    println!("done");
}

fn index(music_dir: &Path) -> (HashMap<OsString, Vec<PathBuf>>, Vec<PathBuf>) {
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
    let mut music_index = HashMap::<OsString, Vec<PathBuf>>::new();
    let mut playlist_index = Vec::new();

    let iter = WalkDir::new(abs_music_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| match e.metadata() {
            Ok(m) => m.is_file(),
            Err(_e) => false,
        });
    for d in iter {
        let path = d.into_path();
        if let Some(extension) = path.extension() {
            let Some(file_stem) = path.file_stem() else {
                continue;
            };
            match Extension::of(extension) {
                Extension::Music => match music_index.entry(file_stem.to_owned()) {
                    Entry::Occupied(songs) => {
                        songs.into_mut().push(path);
                    }
                    Entry::Vacant(v) => {
                        v.insert(vec![path]);
                    }
                },
                Extension::Playlist => playlist_index.push(path),
                Extension::Unknown => (),
            }
        }
    }

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

fn m3u_playlist<'a>(
    index: &'a HashMap<OsString, Vec<PathBuf>>,
    file_paths: &[PathBuf],
    name: String,
) -> Playlist<'a> {
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
            if s == *e {
                return Extension::Music;
            }
        }
        for e in PLAYLIST_EXTENSIONS.iter() {
            if s == *e {
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
fn match_file<'index>(
    index: &'index HashMap<OsString, Vec<PathBuf>>,
    file_path: &Path,
) -> Option<&'index Path> {
    let file_stem = file_path.file_stem()?;
    let local_songs = index.get(file_stem)?;

    let mut best_match = FileMatch::default();
    for local_path in local_songs.iter() {
        let (Some(local_extension), Some(file_extension)) =
            (local_path.extension(), file_path.extension())
        else {
            continue;
        };

        let mut file_match = FileMatch {
            path: Some(local_path),
            ..Default::default()
        };
        let local_components = local_path.components().rev().skip(1);
        let file_components = file_path.components().rev().skip(1);
        for (i, (local_comp, file_comp)) in local_components.zip(file_components).enumerate() {
            if local_comp != file_comp {
                file_match.matching_components = i;
                break;
            }
        }

        file_match.extension_matches = file_extension == local_extension;

        if best_match.matching_components < file_match.matching_components
            || best_match.matching_components == file_match.matching_components
                && !best_match.extension_matches
                && file_match.extension_matches
        {
            best_match = file_match;
        }
    }

    best_match.path
}

#[cfg(not(target_os = "windows"))]
fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    path.canonicalize()
}

#[cfg(target_os = "windows")]
fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    let string = path.canonicalize()?.display().to_string();

    Ok(PathBuf::from(string.replace("\\\\?\\", "")))
}

#[cfg(not(target_os = "windows"))]
fn platform_path(string: &str) -> PathBuf {
    let path = string.replace('\\', "/");
    PathBuf::from(path)
}

#[cfg(target_os = "windows")]
fn platform_path(string: &str) -> PathBuf {
    let path = string.replace('/', "\\");
    PathBuf::from(path)
}
