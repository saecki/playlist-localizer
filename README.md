# playlist_localizer
A commandline utility for finding the local paths of your playlists' songs.

## Usage
```
USAGE:
  playlist_localizer [OPTIONS] --music-dir <music-dir> --output-dir <output-dir>

FLAGS:
  -h, --help       Prints help information
  -V, --version    Prints version information

OPTIONS:
  -f, --format <format>                      The wanted output format [possible values: m3u, extm3u]
  -g, --generate-completion <shell>          Generates a completion script for the specified shell
                                             [possible values: zsh, bash, fish, powershell, elvish]
  -m, --music-dir <music-dir>                The directory which will be searched for playlists and music files
  -o, --output-dir <output-dir>              The directory which the playlists will be written to
  -e, --output-file-extension <extension>    The file extension of the output playlist files
```