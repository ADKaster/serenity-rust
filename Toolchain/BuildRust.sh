#!/usr/bin/env bash
set -eo pipefail

# === CONFIGURATION AND SETUP ===

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "$DIR"

PREFIX="$DIR/Local/rust/"
BUILD="$DIR/../Build/"

SED="sed"
REALPATH="realpath"
NPROC="nproc"

SYSTEM_NAME="$(uname -s)"

if [ "$SYSTEM_NAME" = "OpenBSD" ]; then
    REALPATH="readlink -f"
    NPROC="sysctl -n hw.ncpuonline"
    export CC=egcc
    export CXX=eg++
    export LDFLAGS=-Wl,-z,notext
elif [ "$SYSTEM_NAME" = "FreeBSD" ]; then
    NPROC="sysctl -n hw.ncpu"
elif [ "$SYSTEM_NAME" = "Darwin" ]; then
    NPROC="sysctl -n hw.ncpu"
    REALPATH="grealpath"  # GNU coreutils
    SED="gsed"            # GNU sed
fi

if [ -z "$MAKEJOBS" ]; then
    MAKEJOBS=$($NPROC)
fi

if [ ! -d "$BUILD" ]; then
    mkdir -p "$BUILD"
fi
BUILD=$($REALPATH "$BUILD")

echo PREFIX is "$PREFIX"

mkdir -p "$DIR/Tarballs"

### Grab latest rust fork
pushd Tarballs

# Use a shallow clone. if you want to unshallow, ``git fetch --unshallow``
[ ! -d rust ] && git clone git@github.com:awesomekling/rust.git --depth 50

cd rust

git pull

#### Build rust for serenity

# Generate config.toml with proper absolute paths for the current serenity tree
"$SED" "s|@SERENITY_TOOLCHAIN_ROOT@|${DIR}|g" "$DIR/Rust/config.toml.in" > "$DIR/Tarballs/rust/config.toml"

export DESTDIR="$PREFIX"

python3 ./x.py install -i --stage 1 --target x86_64-unknown-serenity compiler/rustc library/std cargo rustfmt

# Make sure we have proc_macros available for host with a matching version, in case the developer doesn't have
# a nightly installed that they keep up to date. If adding more/different hosts here, update config.toml.in [build.target]
python3 ./x.py install -i --stage 1 --target x86_64-unknown-linux-gnu library/std

popd
