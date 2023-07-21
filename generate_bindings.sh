#!/bin/bash


# ----- Configure this before running -----
VIMBA_INCLUDE_DIR="$HOME/Vimba_6_0/VimbaC/Include"
VIMBA_LIBRARY_DIR="$HOME/Vimba_6_0/VimbaC/DynamicLib/x86_64bit"
# -----------------------------------------


if ! HEADER_PATH=$(realpath -q "$VIMBA_INCLUDE_DIR/VimbaC.h"); then
    echo "VimbaC.h in directory '$VIMBA_INCLUDE_DIR' couldn't be found" >&2
    exit 1
fi

echo "$VIMBA_LIBRARY_DIR" > libdir

bindgen "$HEADER_PATH" \
    --default-enum-style moduleconsts \
    --with-derive-partialeq \
    --with-derive-default \
    --distrust-clang-mangling \
    --raw-line "#![allow(non_upper_case_globals,non_snake_case)]" \
    --raw-line "#![allow(non_camel_case_types,dead_code)]" \
    -o "src/vimba_sys.rs"
