#!/usr/bin/env sh
# Guided first-time setup on a fresh machine: builds the api image, runs
# the setup program inside it with this checkout mounted (it checks the
# domain, verifies every credential with its provider and writes .env),
# then starts the production stack.
set -e
cd "$(dirname "$0")/.."

docker compose build api
docker compose run --rm --no-deps -it \
  --user "$(id -u):$(id -g)" \
  -v "$PWD:/host" -e SETUP_ENV_PATH=/host/.env \
  api setup

printf '\nStart the stack now? [Y/n] '
read -r answer
case "$answer" in
  n|N|no|NO) echo "Later: docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build"; exit 0 ;;
esac
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
echo
echo "The stack is starting; the SDE import runs first and the API answers on /api/health once seeded."
echo "Follow it with: docker compose logs -f api"
