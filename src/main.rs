extern crate walkdir;

use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Write};
use std::error::Error;

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
        let mut file = File::create(file_path).unwrap();

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

fn main() {
    let root_path = Path::new("/mnt/data/Music");
    let playlist_output_dir = Path::new("/home/tobi/.config/cmus/playlists/");
    let input_extension = "m3u";
    let index = index(root_path);

    let m3u_playlist_paths = match_file_extension(&index, input_extension);
    let mut m3u_playlists: Vec<Playlist> = Vec::new();

    for p in m3u_playlist_paths {
        let file_names = file_names_m3u(p);
        let playlist_name = String::from(p.file_stem().unwrap().to_str().unwrap());
        let mut playlist = playlist_m3u(&index, &file_names, playlist_name);
        playlist.write_to(playlist_output_dir);

        m3u_playlists.push(playlist);
    }
}

fn index(root_path: &Path) -> Vec<PathBuf> {
    let abs_root_path = root_path.canonicalize().unwrap();
    let mut index = Vec::new();

    for d in WalkDir::new(abs_root_path).into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().unwrap().is_file())
    {
        index.push(d.into_path())
    }

    index
}

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

fn file_names_m3u(playlist_file: &Path) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    let mut file = File::open(playlist_file).unwrap();
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

fn playlist_m3u(index: &Vec<PathBuf>, file_names: &Vec<String>, name: String) -> Playlist {
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