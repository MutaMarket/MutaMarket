#!/usr/bin/env sh
# Rolls the production stack to the images tagged with the given commit
# sha: records it as IMAGE_TAG in .env (so a later manual `up -d` keeps
# the same version), pulls, recreates the changed containers and waits
# for the api to answer. Run by the deploy workflow over ssh after it
# has moved this checkout to the same commit; usable by hand for a
# rollback: `deploy/deploy.sh <older sha>`.
set -eu
cd "$(dirname "$0")/.."

tag="${1:?usage: deploy/deploy.sh <image tag>}"
compose="docker compose -f docker-compose.yml -f docker-compose.prod.yml"

if grep -q '^IMAGE_TAG=' .env; then
  sed -i "s|^IMAGE_TAG=.*|IMAGE_TAG=$tag|" .env
else
  printf '\nIMAGE_TAG=%s\n' "$tag" >> .env
fi

$compose pull --quiet
$compose up -d --remove-orphans

# The api loads every estimator model before it listens; give it time.
health_attempts=60
origin="$(sed -n 's/^STACK_ORIGIN=//p' .env)"
i=0
until curl -fsS --max-time 5 "$origin/api/health" > /dev/null; do
  i=$((i + 1))
  if [ "$i" -ge "$health_attempts" ]; then
    echo "api did not become healthy at $origin/api/health" >&2
    $compose logs --tail 50 api >&2
    exit 1
  fi
  sleep 5
done
echo "deployed $tag"

# Superseded sha-tagged images would otherwise accumulate on the disk.
docker image prune -f > /dev/null
