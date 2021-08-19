#!/bin/sh

cargo install --path .

case "$SHELL" in
    *zsh)
    echo "creating a completion script for zsh"
    ~/.cargo/bin/playlist-localizer -g "zsh" > ~/.config/zsh/functions/_playlist-localizer
    ;;
    *)
    echo "create a completion script for your shell manually by running 'playlist-localizer --generate-completion <shell>'"
    ;;
esac

