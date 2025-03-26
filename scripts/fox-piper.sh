#!/bin/bash

AUDIO_PLAYER=$(command -v fox-reader)
if [ -z "$AUDIO_PLAYER" ]; then
  echo "Error: fox-reader is not installed or not in PATH." >&2
  exit 1
fi

RATE_INT=${RATE%.*}
DATA=$(echo "$DATA" | sed ':a;N;$!ba;s/\n/ /g')

# Rate calculation for firefox compatibility
# Comment it out if you don't want to any changes to rate
if [ "$RATE_INT" -lt 1 ]; then
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 100}")
elif [ "$RATE_INT" -eq 0 ]; then
  RATE=1
else
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 75}")
fi

$AUDIO_PLAYER --cli --model "$VOICE_PATH/$VOICE" --text "$DATA" --rate "$RATE"
