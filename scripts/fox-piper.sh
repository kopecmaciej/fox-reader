#!/bin/bash

AUDIO_PLAYER=$(command -v fox-reader)
if [ -z "$AUDIO_PLAYER" ]; then
  echo "Error: fox-reader is not installed or not in PATH." >&2
  exit 1
fi

# Convert Firefox speed range (-75 to 75) to target range (0.5 to 2)
convert_firefox_speed() {
  local firefox_speed=$1
  
  if (( $(echo "$firefox_speed < -75" | bc -l) )); then
    firefox_speed=-75
  elif (( $(echo "$firefox_speed > 75" | bc -l) )); then
    firefox_speed=75
  fi
  
  local converted_speed=$(echo "scale=2; 0.01 * $firefox_speed + 1.25" | bc -l)
}

# Check if we're being called from Firefox and convert speed accordingly
if [ -n "$MOZ_CRASHREPORTER_DATA_DIRECTORY" ]; then
  CONVERTED_RATE=$(convert_firefox_speed "$RATE")
  IS_FIREFOX=true
else
  CONVERTED_RATE="$RATE"
  IS_FIREFOX=false
fi

RATE_INT=${CONVERTED_RATE%.*}
DATA=$(echo "$DATA" | sed ':a;N;$!ba;s/\n/ /g')

## If rate is 0 then set it to 1, we need to check for 0.00
if [ -z "$CONVERTED_RATE" ] || (( $(echo "$CONVERTED_RATE <= 0.00" | bc -l) )) || [ "$RATE_INT" -eq 0 ]; then
  CONVERTED_RATE=1
fi

{
  $AUDIO_PLAYER --cli --voice "$VOICE" --text "$DATA" --speed "$CONVERTED_RATE"
}
