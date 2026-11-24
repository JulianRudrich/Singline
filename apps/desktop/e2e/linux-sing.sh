#!/usr/bin/env bash
# End-to-end check of Sing to find on Linux: instead of the microphone the app
# hears a recording of four lines of "Morning Harbor" (SINGLINE_MIC_FILE),
# transcribes it with a real Whisper model, asks a pretend Genius which song
# it is and the pretend LRCLIB for the lyrics. The check then compares every
# line the overlay shows with where the singing really was, and saves
# screenshots of the listening and the found state.
#
# A second round sings the same lines offline: Singline quits as at logout,
# starts again without a Genius token, with Genius and LRCLIB out of reach,
# and has to find the song among the songs heard in the first round, on the
# device.
#
# The recording is synthesized with espeak-ng (sudo apt install espeak-ng):
# line k starts 1 + 4k seconds into the file, so the first sung line ("I can
# hear the morning…", at 12 s in the song) is heard 1 s after listening
# starts. Any Whisper model works; the small tiny one is enough:
#   mkdir -p ~/.cache/singline-e2e && curl -L -o ~/.cache/singline-e2e/ggml-tiny.bin \
#     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin
# SINGLINE_E2E_MODEL points at another model file. Without a model or
# espeak-ng the check is skipped.
#
# Needs what linux-smoke.sh needs. Build first:
#   pnpm --filter singline-desktop tauri build --debug --no-bundle
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
APP="${APP:-$ROOT/target/debug/singline}"
OUT="${OUT:-$ROOT/target/e2e-sing}"
PORT="${PORT:-18766}"
GENIUS_PORT="${GENIUS_PORT:-18767}"
MODEL="${SINGLINE_E2E_MODEL:-${XDG_CACHE_HOME:-$HOME/.cache}/singline-e2e/ggml-tiny.bin}"
HERE="$ROOT/apps/desktop/e2e"

if [ ! -s "$MODEL" ]; then
  echo "SKIPPED: no Whisper model at $MODEL (see the top of $0 for how to get one)"
  exit 0
fi
if ! command -v espeak-ng > /dev/null; then
  echo "SKIPPED: espeak-ng is not installed, so the sung lines cannot be synthesized"
  exit 0
fi

rm -rf "$OUT" && mkdir -p "$OUT/data"
# Not the first start: no welcome window over the overlay.
echo '{"welcomed": true}' > "$OUT/data/settings.json"
python3 "$HERE/sung_audio.py" "$OUT/sung.wav" 1.0 4.0 \
  "I can hear the morning calling from the mountain" \
  "Every window opens when the swallows come home" \
  "Hold the lantern higher so the travelers find us" \
  "We will sing until the valley answers back"
printf '{ "geniusToken": "e2e-token" }\n' > "$OUT/data/sing.json"

export WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1 NO_AT_BRIDGE=1
export LANG=en_US.UTF-8 SINGLINE_DEBUG=1
export SINGLINE_DATA_DIR="$OUT/data" SINGLINE_LRCLIB_URL="http://127.0.0.1:$PORT"
export SINGLINE_GENIUS_URL="http://127.0.0.1:$GENIUS_PORT" SINGLINE_MIC_FILE="$OUT/sung.wav" SINGLINE_WHISPER_MODEL="$MODEL"
export APP OUT PORT GENIUS_PORT HERE

timeout 240 dbus-run-session -- xvfb-run -a -s "-screen 0 1280x800x24" bash -c '
  set -u
  xsetroot -solid "#3b5a73" 2>/dev/null || true
  # Starts the app as $app. Every line it prints goes to the file $1, with
  # the wall-clock time it arrived.
  start_app() {
    "$APP" > >(python3 -u -c "
import sys, time
for line in sys.stdin:
    print(f\"{time.time():.3f} {line}\", end=\"\", flush=True)
" > "$1") 2>&1 &
    app=$!
  }
  # Waits until the file $2 holds $1, for $3 quarter seconds at most.
  wait_for() {
    for _ in $(seq "$3"); do
      grep -q "$1" "$2" 2>/dev/null && return 0
      sleep 0.25
    done
    return 1
  }
  # Quits as the system does at logout, and waits until the app is gone.
  quit_app() {
    kill -TERM "$app" 2>/dev/null
    for _ in $(seq 40); do
      kill -0 "$app" 2>/dev/null || return 0
      sleep 0.25
    done
    kill -KILL "$app" 2>/dev/null
  }

  python3 "$HERE/mock_lrclib.py" "$PORT" 2> "$OUT/lrclib.log" &
  lrclib=$!
  python3 "$HERE/mock_genius.py" "$GENIUS_PORT" 2> "$OUT/genius.log" &
  genius=$!
  sleep 1
  start_app "$OUT/app.log"
  wait_for "sing: listening" "$OUT/app.log" 120
  sleep 2.5
  import -window root "$OUT/listening.png" || true
  wait_for "sing: found" "$OUT/app.log" 160
  sleep 2
  import -window root "$OUT/found.png" || true
  sleep 16
  quit_app
  kill "$lrclib" "$genius" 2>/dev/null
  wait "$lrclib" "$genius" 2>/dev/null

  # Offline: no token, and LRCLIB is out of reach. Genius is there only to
  # count, and must not be asked.
  rm -f "$SINGLINE_DATA_DIR/sing.json"
  export SINGLINE_LRCLIB_URL="http://127.0.0.1:9"
  python3 "$HERE/mock_genius.py" "$GENIUS_PORT" 2> "$OUT/genius-offline.log" &
  sleep 1
  start_app "$OUT/app-offline.log"
  wait_for "sing: found" "$OUT/app-offline.log" 200
  sleep 10
  quit_app
  kill $(jobs -p) 2>/dev/null || true
' 2>&1 | grep -v "fd limit" || true

python3 - "$OUT" <<'PY'
import json, pathlib, re, sqlite3, sys

out = pathlib.Path(sys.argv[1])
failures = []

# The song as the pretend LRCLIB has it: a line every four seconds.
harbor = [
    "", "The river carries every name we gave the summer", "Paper boats are sailing past the silent harbor",
    "I can hear the morning calling from the mountain", "Every window opens when the swallows come home",
    "Hold the lantern higher so the travelers find us", "We will sing until the valley answers back",
    "Golden fields are waiting on the other side", "Nobody remembers how the story started",
    "But the morning keeps on calling from the mountain", "And the river carries every name we gave", "",
]
def line_at(position):
    index = max(0, min(len(harbor) - 1, int(position // 4)))
    return harbor[index]

# The first sung line is at 12 s in the song and 1 s into the recording.
OFFSET = 11.0

def read(name):
    path = out / name
    return path.read_text() if path.exists() else ""

def requests(name):
    return [json.loads(line) for line in read(name).splitlines() if line.startswith("{")]

def check_round(name, log):
    """Sing to find found the sung song, the overlay followed the singing and the app quit cleanly."""
    problems = []
    started = re.search(r"^([0-9.]+) sing: listening to the file", log, re.M)
    found = re.search(r'^([0-9.]+) sing: found "([^"]+)" by "([^"]+)"', log, re.M)
    if not started:
        problems.append("Sing to find never started listening")
    if not found:
        problems.append("Sing to find found no song")
    elif (found.group(2), found.group(3)) != ("Morning Harbor", "Example Artist"):
        problems.append(f"found {found.group(2)!r} by {found.group(3)!r} instead of the sung song")
    if "SIGTERM: quitting" not in log:
        problems.append("the app did not quit cleanly on SIGTERM")
    errors = []
    reports = [(float(at), float(position), text) for at, position, text in re.findall(r"^([0-9.]+) overlay-line (-?[0-9.]+) (.*)$", log, re.M)]
    if len(reports) < 3:
        problems.append(f"the overlay showed {len(reports)} line(s); expected at least 3")
    for at, position, text in reports:
        # The overlay shows a long line in two halves.
        expected = line_at(position + 0.05)
        if text not in expected or (text == "") != (expected == ""):
            problems.append(f"at {position} s the overlay showed {text!r}, expected {expected!r}")
        if started:
            sung = OFFSET + at - float(started.group(1))
            errors.append(position - sung)
            if abs(position - sung) > 1.5:
                problems.append(f"the overlay was at {position:.2f} s while the singing was at {sung:.2f} s")
    print(f"{name}: overlay lines: {len(reports)}")
    for text in re.findall(r"^[0-9.]+ sing: heard (\".*\")", log, re.M)[:6]:
        print(f"  heard {text}")
    if found:
        print(f"  found {found.group(2)} by {found.group(3)} {float(found.group(1)) - float(started.group(1)) if started else 0:.1f} s after listening started")
    for (at, position, text), error in zip(reports, errors):
        print(f"  {position:7.2f} s  {error:+.2f} s  {text}")
    if errors:
        print(f"  timing error: mean {sum(abs(e) for e in errors) / len(errors):.2f} s, largest {max(abs(e) for e in errors):.2f} s")
    failures.extend(f"{name}: {problem}" for problem in problems)

# Round 1: Genius names the candidates, LRCLIB has their lyrics.
log = read("app.log")
check_round("online", log)
genius = requests("genius.log")
if not genius:
    failures.append("online: the app never asked Genius")
for request in genius:
    if request["authorization"] != "Bearer e2e-token":
        failures.append(f"online: Genius was asked without the token: {request['authorization']!r}")
    if set(request["query"]) != {"q"}:
        failures.append(f"online: Genius got more than the words: {request['query']}")
lrclib = requests("lrclib.log")
asked = {r["query"].get("track_name") for r in lrclib if r["path"] == "/api/search"}
if not {"City Morning", "Morning Harbor"} <= asked:
    failures.append(f"online: LRCLIB was not asked for both candidates: {sorted(asked)}")
print(f"  Genius requests: {len(genius)}, LRCLIB requests: {len(lrclib)}")

# Round 2: no token and nothing to reach, so the song must come from the
# songs heard in round 1.
offline = read("app-offline.log")
check_round("offline", offline)
if "of the songs you heard fit" not in offline:
    failures.append("offline: the songs heard before were not searched")
asked = requests("genius-offline.log")
if asked:
    failures.append(f"offline: Genius was asked {len(asked)} time(s) for a song heard before")

# Each round's song went into the listening history when the app quit.
database = out / "data" / "singline.sqlite"
listens = []
if database.exists():
    with sqlite3.connect(database) as connection:
        listens = connection.execute("SELECT title, artist, service FROM listens").fetchall()
sung = [listen for listen in listens if listen == ("Morning Harbor", "Example Artist", "microphone")]
if len(sung) < 2:
    failures.append(f"the listening history holds {len(sung)} sung song(s), expected one for each round: {listens}")

if failures:
    print("FAILED:\n  " + "\n  ".join(failures))
    print("--- app.log ---\n" + log[-3000:])
    print("--- app-offline.log ---\n" + offline[-3000:])
    sys.exit(1)
print(f"OK: Sing to find found the song online and offline and followed the singing. Screenshots: {out / 'listening.png'}, {out / 'found.png'}")
PY
