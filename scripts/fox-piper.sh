#!/bin/bash

AUDIO_PLAYER=$(command -v fox-reader)
if [ -z "$AUDIO_PLAYER" ]; then
  echo "Error: fox-reader is not installed or not in PATH." >&2
  exit 1
fi

RATE_INT=${RATE%.*}
DATA=$(echo "$DATA" | sed ':a;N;$!ba;s/\n/ /g')

$AUDIO_PLAYER --cli --voice "$VOICE" --text "$DATA" --speed "$RATE"
