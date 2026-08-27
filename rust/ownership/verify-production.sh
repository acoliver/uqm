#!/bin/sh
set -eu

: "${UQM_CI_SOURCE_ROOT:?UQM_CI_SOURCE_ROOT must be supplied by the trusted controller}"
: "${UQM_CI_CONTROLLER_EXECUTABLE:?UQM_CI_CONTROLLER_EXECUTABLE must be supplied by the trusted controller}"

"${UQM_CI_CONTROLLER_EXECUTABLE}" __ci-verify
"${UQM_CI_CONTROLLER_EXECUTABLE}" __ci-ownership-production
