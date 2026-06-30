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
  "box_direct_make_ready.png"
  "box_property_edit.png"
  "flat_panel_direct_make_ready.png"
  "flat_panel_property_edit.png"
  "pack_drawer.png"
  "export_drawer.png"
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
  "box_property_edit.png box_direct_make_ready.png"
  "flat_panel_direct_make_ready.png box_direct_make_ready.png"
  "flat_panel_property_edit.png flat_panel_direct_make_ready.png"
  "pack_drawer.png box_property_edit.png"
  "export_drawer.png pack_drawer.png"
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
