#!/usr/bin/env bash

set -e

script_path=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd "${script_path}/.." || exit 1

if [ "$#" -eq "1" ]; then
    mapfile -t files < <(
        git ls-files -- \
            '*.rs'
    )
else
    files=()
    for file in "${@:2}"; do
        if [[ "${file}" == *".rs" ]]; then
            files+=("${file}")
        fi
    done
fi

if (( ${#files[@]} )); then
    TOOLCHAIN_DIR=Toolchain/Local/rust/bin
    RUSTFMT=$TOOLCHAIN_DIR/rustfmt

    if [ "$#" -gt "0" ] && [ "--overwrite-inplace" = "$1" ] ; then
        true # The only way to run this script.
    else
        # Note that this branch also covers --help, -h, -help, -?, etc.
        echo "USAGE: $0 --overwrite-inplace"
        echo "The argument is necessary to make you aware that this *will* overwrite your local files."
        exit 1
    fi

    echo "Using ${RUSTFMT}"

    "${RUSTFMT}" "${files[@]}"
    echo "Maybe some files have changed. Sorry, but rustfmt doesn't indicate what happened."
else
    echo "No .rs files to check."
fi
