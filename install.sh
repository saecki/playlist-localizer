#!/bin/sh

cargo build --release
sudo cp target/release/playlist-localizer /usr/local/bin

case "$SHELL" in
    *bash)
    echo "creating a completion script for bash"
    /usr/local/bin/playlist-localizer -g "bash" | sudo tee /etc/bash_completion.d/playlist-localizer > /dev/null
    ;;
    *zsh)
    echo "creating a completion script for zsh"
    /usr/local/bin/playlist-localizer -g "zsh" | sudo tee /usr/share/zsh/site-functions/_playlist-localizer > /dev/null
    ;;
    *)
    echo "create a completion script for your shell manually by running 'playlist-localizer --generate-completion <shell>'"
    ;;
esac

