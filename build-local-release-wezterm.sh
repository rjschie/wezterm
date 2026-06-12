#!/usr/bin/env bash
set -euo pipefail

# Commands based on the Git Action for building wezterm
# @see https://github.com/wezterm/wezterm/blob/main/.github/workflows/gen_macos.yml

export MACOSX_DEPLOYMENT_TARGET="10.12"

tmp_dir="/tmp"

rustup target add aarch64-apple-darwin

for p in wezterm wezterm-gui wezterm-mux-server strip-ansi-escapes; do
  cargo build --target aarch64-apple-darwin -p "$p" --release
done

# set -a
# source "$SCRIPT_DIR/.env"
# set +a
bash ci/deploy.sh
rm -rf $tmp_dir/WezTerm.app
mv ./WezTerm-macos-*/WezTerm.app $tmp_dir/
codesign --force --sign - "$tmp_dir/WezTerm.app"
open "$tmp_dir"
open "/Applications"

