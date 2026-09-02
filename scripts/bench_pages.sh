#!/bin/zsh
# Times the module-listing endpoints against a full legacy import as one
# heavy real account, then lists the Postgres statements that crossed the
# slow-log threshold while they ran. Run the API locally first
# (`SCHEDULER_ENABLED=false cargo run --release`) and mint a session for
# the account:
#
#   insert into sessions (token, user_id, active_character_id, expires_at)
#   values ('bench', <user_id>, <character_id>, now() + interval '1 day');
#
# Enable the slow log with
#   alter system set log_min_duration_statement = '100ms'; select pg_reload_conf();
# (reset with `= -1`). The defaults below are the heaviest rows of the
# 2026-09 snapshot; override any of them through the environment.
export PATH=/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin:$PATH
BASE=${BASE:-http://127.0.0.1:3000}
SESSION=${SESSION:-bench}
TYPE=${TYPE:-49738}
MODULE=${MODULE:-1055457735166}
LOCATION=${LOCATION:-jita-iv-moon-4-caldari-navy-assembly-plant-60003760}
CHARACTER=${CHARACTER:-sithus-asques-94211409}
COLLECTION=${COLLECTION:-all-for-sale-OwEj2oRLK8ec1KNH}
PG_CONTAINER=${PG_CONTAINER:-mutamarket-postgres}

START=$(date -u +%Y-%m-%dT%H:%M:%S)
ENDPOINTS=(
  "/api/module-cards"
  "/api/module-cards/type/$TYPE"
  "/api/module-cards/type/$TYPE/sort/value/desc"
  "/api/module-cards/type/$TYPE/sort/price/asc"
  "/api/module-cards/type/$TYPE/goldbar"
  "/api/module-cards/type/$TYPE?unlisted=true"
  "/api/modules/type/$TYPE"
  "/api/module-page/x-$MODULE"
  "/api/module-page/x-$MODULE/similar"
  "/api/historic-sales-cards/type/$TYPE"
  "/api/personal/page"
  "/api/personal/modules"
  "/api/personal/modules?q=type/$TYPE"
  "/api/locations"
  "/api/locations/$LOCATION"
  "/api/characters/$CHARACTER"
  "/api/collections/$COLLECTION"
  "/api/sell/page"
  "/api/sell/modules"
  "/api/sell/locations"
)
printf '%-70s %8s %6s %8s\n' endpoint ms status bytes
for endpoint in $ENDPOINTS; do
  out=$(curl -s -o /dev/null -w '%{time_total} %{http_code} %{size_download}' \
    -H "Cookie: mm_session=$SESSION" "$BASE$endpoint")
  t=${out%% *}; rest=${out#* }; code=${rest%% *}; size=${rest#* }
  printf '%-70s %8.0f %6s %8s\n' "$endpoint" $((t*1000)) "$code" "$size"
done

echo
echo "--- postgres statements over the slow-log threshold since $START"
docker logs --since "$START" "$PG_CONTAINER" 2>&1 \
  | grep -A6 'duration:' | grep -v '^--$\|DETAIL:' \
  | sed 's/^.*LOG:  duration: /duration: /' | cut -c1-200
