#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

if [[ $# -lt 1 ]]; then
  echo "usage: $0 SCREENSHOT_DIR [MIN_WIDTH] [MIN_HEIGHT]" >&2
  exit 2
fi

screenshot_dir="$1"
min_width="${2:-1000}"
min_height="${3:-700}"

required=(
  "01_choose.png"
  "02_make_ready.png"
  "03_generating_ideas.png"
  "04_generated_ideas.png"
  "05_selected_comparison.png"
  "06_focus_handles.png"
  "07_generating_handle_ideas.png"
  "08_handle_ideas.png"
  "09_focus_vents.png"
  "10_pack_drawer.png"
  "11_export_drawer.png"
)

hash_names=()
hash_values=()

image_width() {
  sips -g pixelWidth "$1" 2>/dev/null | awk '/pixelWidth/ { print $2 }'
}

image_height() {
  sips -g pixelHeight "$1" 2>/dev/null | awk '/pixelHeight/ { print $2 }'
}

image_hash() {
  shasum -a 256 "$1" | awk '{ print $1 }'
}

for name in "${required[@]}"; do
  path="$screenshot_dir/$name"
  if [[ ! -f "$path" ]]; then
    echo "Missing screenshot: $name" >&2
    exit 1
  fi

  width="$(image_width "$path")"
  height="$(image_height "$path")"
  if [[ -z "$width" || -z "$height" ]]; then
    echo "Could not inspect screenshot dimensions: $name" >&2
    exit 1
  fi
  if (( width < min_width || height < min_height )); then
    echo "Screenshot is too small: $name is ${width}x${height}" >&2
    exit 1
  fi
  hash_names+=("$name")
  hash_values+=("$(image_hash "$path")")
  printf '%s %sx%s %s\n' "$name" "$width" "$height" "${hash_values[$((${#hash_values[@]} - 1))]}"
done

hash_for() {
  needle="$1"
  index=0
  for name in "${hash_names[@]}"; do
    if [[ "$name" == "$needle" ]]; then
      printf '%s\n' "${hash_values[$index]}"
      return 0
    fi
    index=$((index + 1))
  done
  return 1
}

pairs=(
  "03_generating_ideas.png 02_make_ready.png"
  "04_generated_ideas.png 03_generating_ideas.png"
  "05_selected_comparison.png 04_generated_ideas.png"
  "06_focus_handles.png 05_selected_comparison.png"
  "07_generating_handle_ideas.png 06_focus_handles.png"
  "08_handle_ideas.png 06_focus_handles.png"
  "09_focus_vents.png 08_handle_ideas.png"
  "10_pack_drawer.png 09_focus_vents.png"
  "11_export_drawer.png 10_pack_drawer.png"
)

for pair in "${pairs[@]}"; do
  read -r left right <<<"$pair"
  left_hash="$(hash_for "$left")"
  right_hash="$(hash_for "$right")"
  if [[ "$left_hash" == "$right_hash" ]]; then
    echo "Screenshots should differ but are identical: $left and $right" >&2
    exit 1
  fi
done

echo "Make Canvas screenshot sanity passed."
