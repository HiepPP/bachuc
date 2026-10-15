#!/bin/sh
# Samples CPU and memory of process groups for N seconds.
# Usage: measure.sh <seconds> <label>=<pgrep -f pattern | pids:1,2,3> [...]
# Example: measure.sh 60 gpui="BachucGpui.app/Contents/MacOS/bachuc-gpui" gpui_wk="pids:<WebKit helper pids>"
# The mermaid WKWebView runs in separate WebKit XPC processes; match them to the
# app by start time (`ps -o pid,lstart`) and pass them with pids: to count them.
# CPU% = delta of cumulative CPU time / wall time (100% = one core).
# Memory = summed RSS and summed phys_footprint (from `footprint`) at the end.
set -eu
secs=$1
shift

cpu_seconds() {
  # ps cputime is [[dd-]hh:]mm:ss.cc; sum for all pids.
  ps -o time= -p "$1" | awk '{
    n = split($1, a, ":"); s = 0
    for (i = 1; i <= n; i++) { v = a[i]; if (index(v, "-")) { split(v, d, "-"); v = d[1] * 24 + d[2] } s = s * 60 + v }
    t += s } END { printf "%.2f", t }'
}

for spec in "$@"; do
  label=${spec%%=*}
  pattern=${spec#*=}
  case $pattern in
    pids:*) pids=${pattern#pids:} ;;
    *) pids=$(pgrep -f "$pattern" | paste -sd, -) ;;
  esac
  eval "pids_$label=\$pids"
  eval "start_$label=\$(cpu_seconds \$pids)"
done
sleep "$secs"
printf "%-10s %5s %8s %9s %12s\n" label procs cpu% rss_mb footprint_mb
for spec in "$@"; do
  label=${spec%%=*}
  eval "pids=\$pids_$label; start=\$start_$label"
  end=$(cpu_seconds "$pids")
  rss=$(ps -o rss= -p "$pids" | awk '{t += $1} END {printf "%.0f", t / 1024}')
  fp=0
  for pid in $(echo "$pids" | tr , ' '); do
    v=$(footprint -p "$pid" 2>/dev/null | awk '/phys_footprint:/ {print $2, $3; exit}')
    fp=$(echo "$fp $v" | awk '{u = $3; x = $2; if (u == "KB") x /= 1024; if (u == "GB") x *= 1024; printf "%.0f", $1 + x}')
  done
  count=$(echo "$pids" | tr , '\n' | wc -l | tr -d ' ')
  cpu=$(echo "$start $end $secs" | awk '{printf "%.1f", ($2 - $1) / $3 * 100}')
  printf "%-10s %5s %8s %9s %12s\n" "$label" "$count" "$cpu" "$rss" "$fp"
done
