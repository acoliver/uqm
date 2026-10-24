#!/usr/bin/env bash
set -euo pipefail

# Generate locally with: update_homebrew_tap.sh vX.Y.Z SHA256 > uqm.rb
# Publishing is opt-in and only used by the stable release workflow.
if [[ $# -lt 2 || $# -gt 3 || ! "$1" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ || ! "$2" =~ ^[a-f0-9]{64}$ ]]; then
  echo "usage: $0 vX.Y.Z SHA256 [--publish]" >&2
  exit 1
fi
version="${1#v}"
formula() {
  cat <<EOF
class Uqm < Formula
  desc "Ur-Quan Masters space adventure"
  homepage "https://github.com/acoliver/uqm"
  url "https://github.com/acoliver/uqm/releases/download/$1/uqm-$1-aarch64-apple-darwin.tar.gz"
  sha256 "$2"
  license all_of: ["GPL-2.0-or-later", "CC-BY-NC-SA-2.5"]

  depends_on arch: :arm64
  depends_on :macos

  def install
    app = prefix/"UQM.app"
    app.install "Contents"
    bin.install_symlink app/"Contents/MacOS/uqm" => "uqm"
  end

  test do
    assert_equal version.to_s, shell_output("#{bin}/uqm --version").strip
    assert_match "Content OK:", shell_output("#{bin}/uqm --check-content")
  end
end
EOF
}
if [[ $# -eq 2 ]]; then
  formula "$1" "$2"
  exit
fi
[[ "$3" == --publish ]] || { echo "expected --publish" >&2; exit 1; }
: "${HOMEBREW_TAP_GITHUB_TOKEN:?Missing tap token}"
export GH_TOKEN="${HOMEBREW_TAP_GITHUB_TOKEN}"
sha=$(gh api 'repos/acoliver/homebrew-tap/git/trees/main?recursive=1' \
  --jq '.tree[] | select(.path == "Formula/uqm.rb") | .sha')
content=$(formula "$1" "$2" | base64 | tr -d '\n')
export TAP_SHA="$sha" TAP_CONTENT="$content" TAP_VERSION="$version"
python3 - <<'PYBODY' | gh api repos/acoliver/homebrew-tap/contents/Formula/uqm.rb --method PUT --input -
import json, os
body = {"message": "uqm " + os.environ["TAP_VERSION"], "content": os.environ["TAP_CONTENT"]}
if os.environ["TAP_SHA"]:
    body["sha"] = os.environ["TAP_SHA"]
print(json.dumps(body))
PYBODY
