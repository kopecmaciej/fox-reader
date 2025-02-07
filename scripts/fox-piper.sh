#!/bin/bash

RATE_INT=${RATE%.*}
VOLUME=${VOLUME:-100}

# Some hacks to work with audio backends properly
if [ "$RATE_INT" -lt 1 ]; then
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 100}")
elif [ "$RATE_INT" -eq 0 ]; then
  RATE=1
else
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 75}")
fi

PROCESSOR=$AUDIO_BACKEND


# Process the text and pipe to the audio processor
echo "$DATA" | sed -z 's/\n/ /g' | $PIPER_PATH -q -m "$VOICE_PATH/$VOICE" -f - | eval "$PROCESSOR"
