#!/bin/bash

find_audio_player() {
  local players=("mpv" "ffplay" "sox" "aplay" "paplay")
  for player in "${players[@]}"; do
    if command -v "$player" >/dev/null 2>&1; then
      echo "$player"
      return 0
    fi
  done
  echo "No supported audio player found. Please install one of: mpv, ffplay, aplay, paplay, or sox" >&2
  exit 1
}

AUDIO_PLAYER=$(find_audio_player)
RATE_INT=${RATE%.*}
VOLUME=${VOLUME:-100}

# Rate calculation for firefox compatibility
# Comment it out if you don't want to any changes to rate
if [ "$RATE_INT" -lt 1 ]; then
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 100}")
elif [ "$RATE_INT" -eq 0 ]; then
  RATE=1
else
  RATE=$(awk "BEGIN {printf \"%.2f\", ($RATE + 100) / 75}")
fi

build_audio_command() {
  case "$AUDIO_PLAYER" in
  "mpv")
    echo "mpv --speed=\"$RATE\" --volume=\"$VOLUME\" --no-terminal --keep-open=no -"
    ;;
  "ffplay")
    echo "ffplay -nodisp -autoexit -volume \"$VOLUME\" -"
    ;;
  "sox")
    if command -v paplay >/dev/null 2>&1; then
      echo "sox -q -r 22050 -b 16 -c 1 -e signed-integer -t raw - -t wav - tempo \"$RATE\" norm vol $(awk "BEGIN {printf \"%.2f\", $VOLUME / 100}") | paplay --format=s16le --rate=22050 --channels=1"
    elif command -v aplay >/dev/null 2>&1; then
      echo "sox -q -r 22050 -b 16 -c 1 -e signed-integer -t raw - -t wav - tempo \"$RATE\" norm vol $(awk "BEGIN {printf \"%.2f\", $VOLUME / 100}") | aplay -q -t raw -c 1 -r \$SAMPLE_RATE -f S16_LE -"
    else
      echo "When using sox, either aplay or paplay must be installed" >&2
      exit 1
    fi
    ;;
  "aplay")
    echo "aplay -q -t raw -c 1 -r \$SAMPLE_RATE -f S16_LE -"
    ;;
  "paplay")
    echo "paplay --volume=$(awk "BEGIN {printf \"%.0f\", $VOLUME * 655.35}") --channels=1 --rate=22050 --raw -"
    ;;
  esac
}

AUDIO_CMD=$(build_audio_command)

echo "$DATA" | sed -z 's/\n/ /g' | $PIPER_PATH -q -m "$VOICE_PATH/$VOICE" -f - | eval "$AUDIO_CMD"
