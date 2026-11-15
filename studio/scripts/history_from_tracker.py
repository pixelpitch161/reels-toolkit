"""One-time converter: the earlier episodes -> the app's starting matchup history.

Reads a CSV (left, right, winner, seed, date_made) transcribed from the Episodes sheet of
ballfight-matchup-tracker.xlsx and writes the TOML that bf-app ships as assets/history_start.toml.
Rows naming a retired fighter (GUITAR, SAX) are left out. Every row starts ticked as uploaded,
made at 12:00 UTC on its date.

    python -I history_from_tracker.py history_start.csv ../crates/bf-app/assets/history_start.toml
"""

import csv
import datetime
import sys

RETIRED = {"GUITAR", "SAX"}


def quote(text):
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main(src, dst):
    with open(src, newline="", encoding="utf-8") as f:
        rows = [r for r in csv.DictReader(f) if r["left"] not in RETIRED and r["right"] not in RETIRED]
    out = ["# The episodes made before BALLFIGHT Studio, from the matchup tracker.",
           "# Made by studio/scripts/history_from_tracker.py; copied in when no history exists.",
           f"next_id = {len(rows)}", ""]
    for i, r in enumerate(rows):
        day = datetime.date.fromisoformat(r["date_made"])
        made = int(datetime.datetime(day.year, day.month, day.day, 12, tzinfo=datetime.timezone.utc).timestamp())
        out += ["[[fights]]", f"id = {i}", f"fighters = [{quote(r['left'])}, {quote(r['right'])}]",
                f"seed = {int(r['seed'])}", f"winner = {quote(r['winner'])}", f"made = {made}",
                "uploaded = true", ""]
    with open(dst, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(out))
    print(f"{len(rows)} rows -> {dst}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
