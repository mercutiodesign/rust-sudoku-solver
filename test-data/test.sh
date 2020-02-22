#!/bin/sh
for f in "$(dirname "$0")"/*.ss; do
    "$1" <"$f"
done
