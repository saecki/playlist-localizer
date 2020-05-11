#!/bin/sh

cargo build --release
sudo cp target/release/playlist_localizer /usr/local/bin

case "$SHELL" in
    *zsh)
	echo "creating a completion script for zsh"
	sudo /usr/local/bin/playlist_localizer -g "zsh" -o /usr/share/zsh/site-functions
	;;
    *bash)
	echo "creating a completion script for bash"
	sudo /usr/local/bin/playlist_localizer -g "bash" -o /etc/bash_completion.d/
	;;
    *)
	echo "create a completion script for your shell manually"
	;;
esac

