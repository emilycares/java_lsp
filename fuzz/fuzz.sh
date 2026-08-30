#!/usr/bin/env bash

export FUZZ_TIME=900
# export FUZZ_TIME=300

targets=(
    parser_class
    parser_jimage
    parser_module
    parser_java
    parser_java_a
    ast_to_class
    cfc
)
for i in "${targets[@]}"; do
    cargo-fuzz build "$i"
    cargo-fuzz run "$i" -- -max_total_time="$FUZZ_TIME"
done
