#!/usr/bin/env bash
# Put a `graphyn` binary on PATH.
#
# A release download takes seconds; building from source takes minutes. The
# default is therefore a download, with `version: source` available for
# repositories that want to run their own tree — which is how Graphyn tests
# this action against itself.
set -euo pipefail

version="${GRAPHYN_VERSION:-latest}"

if [ "$version" = "source" ]; then
  echo "Building graphyn from the checked-out tree."
  cargo build --locked --release -p graphyn-cli
  echo "$PWD/target/release" >> "$GITHUB_PATH"
  exit 0
fi

os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Linux-x86_64)  asset="graphyn-x86_64-unknown-linux-gnu.tar.gz" ;;
  Darwin-arm64)  asset="graphyn-aarch64-apple-darwin.tar.gz" ;;
  *)
    # Named rather than guessed. A wrong asset would download, fail to execute,
    # and look like a Graphyn bug rather than an unsupported runner.
    echo "::error::No prebuilt graphyn for $os-$arch. Use 'version: source', or run this action on a supported runner." >&2
    exit 1
    ;;
esac

if [ "$version" = "latest" ]; then
  url="https://github.com/JeelGajera/graphyn/releases/latest/download/${asset}"
else
  url="https://github.com/JeelGajera/graphyn/releases/download/${version}/${asset}"
fi

dest="${RUNNER_TEMP:-/tmp}/graphyn-bin"
mkdir -p "$dest"

echo "Downloading $url"
if ! curl -fsSL --retry 3 --retry-delay 2 -o "$dest/graphyn.tar.gz" "$url"; then
  echo "::error::Could not download graphyn $version for $os-$arch from $url. Check that the release exists, or use 'version: source'." >&2
  exit 1
fi

tar -xzf "$dest/graphyn.tar.gz" -C "$dest"
chmod +x "$dest/graphyn"
"$dest/graphyn" --version
echo "$dest" >> "$GITHUB_PATH"
