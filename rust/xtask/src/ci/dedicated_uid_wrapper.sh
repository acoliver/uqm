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
has_processes() {
    local status
    /usr/bin/pgrep -U "$uid" >/dev/null 2>&1
    status=$?
    case "$status" in
        0) return 0 ;;
        1) ;;
        *) return 2 ;;
    esac
    /usr/bin/pgrep -u "$uid" >/dev/null 2>&1
    status=$?
    case "$status" in
        0) return 0 ;;
        1) return 1 ;;
        *) return 2 ;;
    esac
}
has_processes
status=$?
case "$status" in
    0)
        echo "dedicated containment uid $uid was already in use" >&2
        exit 125
        ;;
    1) ;;
    *)
        echo "cannot inspect dedicated containment uid $uid" >&2
        exit 125
        ;;
esac
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
trap 'cleanup || true; exit 143' HUP INT TERM
trap 'cleanup || true' EXIT
/usr/bin/sudo -n -u "#$uid" -- /usr/bin/env -i "${env_args[@]}" "$@"
status=$?
if ! cleanup; then
    echo "dedicated containment uid $uid still owns processes after SIGKILL" >&2
    exit 125
fi
exit "$status"