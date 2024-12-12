#!/bin/bash

# Convert RATE to an integer for comparisons
RATE_INT=${RATE%.*}

# Convert RATE to a new range
if [ "$RATE_INT" -lt 1 ]; then
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 100}")
elif [ "$RATE_INT" -eq 0 ]; then
  RATE=1
else
  # To use with firefox and other tools RATE need to be divided
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 75}")
fi

echo "$DATA, $VOICE, $RATE" >>/tmp/piper-reader.log

# Run the command using the provided arguments
echo "$DATA" | sed -z 's/\n/ /g' | piper-tts -q -m "$HOME/.local/share/piper-reader/voices/$VOICE" -f - | mpv --speed="$RATE" --volume=100 --no-terminal --keep-open=no -
