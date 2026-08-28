#!/usr/bin/env bash
uid=$1
shift
count=$1
shift
env_args=()
while [ "$count" -gt 0 ]; do
    env_args+=("$1")
    shift
    count=$((count - 1))
done
has_live_processes() {
    local selector=$1
    local pids
    local status
    local state
    pids="$(/usr/bin/pgrep "$selector" "$uid" 2>/dev/null)"
    status=$?
    case "$status" in
        0) ;;
        1) return 1 ;;
        *) return 2 ;;
    esac
    for pid in $pids; do
        if ! state="$(/bin/ps -o stat= -p "$pid" 2>/dev/null)"; then
            /bin/kill -0 "$pid" 2>/dev/null && return 2
            continue
        fi
        state="${state#"${state%%[![:space:]]*}"}"
        case "$state" in
            Z*|"") ;;
            *) return 0 ;;
        esac
    done
    return 1
}
has_processes() {
    local status
    has_live_processes -U
    status=$?
    case "$status" in
        0) return 0 ;;
        1) ;;
        *) return 2 ;;
    esac
    has_live_processes -u
}
cleanup() {
    local status
    trap - EXIT HUP INT TERM
    attempts=0
    while [ "$attempts" -lt 100 ]; do
        has_processes
        status=$?
        case "$status" in
            0) ;;
            1) return 0 ;;
            *) return 1 ;;
        esac
        /usr/bin/pkill -KILL -U "$uid" >/dev/null 2>&1 || true
        /usr/bin/pkill -KILL -u "$uid" >/dev/null 2>&1 || true
        /bin/sleep 0.02
        attempts=$((attempts + 1))
    done
    has_processes
    [ "$?" -eq 1 ]
}
if ! cleanup; then
    echo "dedicated containment uid $uid still owns processes before launch" >&2
    exit 125
fi
trap 'cleanup || true; exit 143' HUP INT TERM
trap 'cleanup || true' EXIT
/usr/bin/sudo -n -u "#$uid" -- /usr/bin/env -i "${env_args[@]}" "$@"
status=$?
if ! cleanup; then
    echo "dedicated containment uid $uid still owns processes after SIGKILL" >&2
    exit 125
fi
exit "$status"