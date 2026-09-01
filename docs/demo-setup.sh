#!/usr/bin/env bash
#
# Scratch environment for docs/demo.tape.
#
# The TUI's sidebar lists every container, image, volume and network on the
# daemon it is pointed at, so recording against a real machine would put
# whatever happens to be running that day into the GIF. Instead this brings up
# a throwaway Docker-in-Docker daemon on tcp://127.0.0.1:12375 and starts a
# small Compose project inside it. The recording sees exactly five services and
# nothing else, and tearing down is a single `docker rm` of the dind container.
#
#   docs/demo-setup.sh up     # idempotent; blocks until the project is healthy
#   docs/demo-setup.sh down   # removes the dind container and its volume
#
set -euo pipefail

# The tape exports DOCKER_HOST for the recording, and this script is called
# from inside it. Every "outer" docker call below has to reach the real daemon,
# so drop the inherited endpoint — a stray DOCKER_HOST here would nest a second
# dind inside the scratch daemon and put it in the sidebar.
unset DOCKER_HOST DOCKER_CONTEXT DOCKER_TLS_VERIFY DOCKER_CERT_PATH

DIND_NAME=rustydocker-demo-dind
DIND_PORT=12375
DEMO_HOST="tcp://127.0.0.1:${DIND_PORT}"
DEMO_DIR=/tmp/rustydocker-demo
PROJECT=rustydocker-demo

# Images the demo project needs. postgres and alpine are copied over from the
# host daemon when they are already there; the rest are pulled inside dind.
SEED_IMAGES=(postgres:17-alpine alpine:3)
PULL_IMAGES=(redis:7-alpine nginx:alpine)

ddocker() { docker -H "$DEMO_HOST" "$@"; }

write_compose() {
  mkdir -p "$DEMO_DIR"
  cat > "$DEMO_DIR/compose.yml" <<'YAML'
services:
  db:
    image: postgres:17-alpine
    environment:
      POSTGRES_PASSWORD: demo
      POSTGRES_DB: shop
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 3s
      retries: 5

  cache:
    image: redis:7-alpine
    command: ["redis-server", "--save", "", "--appendonly", "no"]

  api:
    image: alpine:3
    depends_on:
      db:
        condition: service_healthy
      cache:
        condition: service_started
    environment:
      DATABASE_URL: postgres://postgres:demo@db:5432/shop
      REDIS_URL: redis://cache:6379
      LOG_LEVEL: info
    command:
      - sh
      - -c
      - |
        i=0
        while :; do
          i=$$((i+1))
          case $$((i % 5)) in
            0) p=/v1/orders ;;
            1) p=/v1/orders/8821 ;;
            2) p=/v1/customers ;;
            3) p=/healthz ;;
            *) p=/v1/cart/items ;;
          esac
          # No timestamp of our own — the TUI already prefixes each line with
          # the Docker one, and a second clock made every entry wrap onto two
          # rows.
          printf 'INFO  GET %-16s 200 %sms\n' "$$p" "$$(( (i * 7) % 40 + 3 ))"
          if [ $$((i % 3)) -eq 0 ]; then
            printf 'WARN  cache miss for key order:%s\n' "$$((i * 131))"
          fi
          # Real traffic and real work, so the Stats tab has non-zero
          # network and CPU deltas to draw instead of flat bars.
          wget -q -O /dev/null http://web/blob.bin 2>/dev/null || true
          n=0
          while [ $$n -lt 25000 ]; do n=$$((n+1)); done
          sleep 2
        done

  worker:
    image: alpine:3
    depends_on:
      db:
        condition: service_healthy
      cache:
        condition: service_started
    command:
      - sh
      - -c
      - |
        i=0
        while :; do
          i=$$((i+1))
          printf 'INFO  job invoice.render #%s done in %sms\n' "$$((4200+i))" "$$(( (i * 13) % 90 + 20 ))"
          sleep 2
        done

  web:
    image: nginx:alpine
    depends_on:
      - api
    ports:
      - "8080:80"
    command:
      - sh
      - -c
      - |
        head -c 262144 /dev/urandom > /usr/share/nginx/html/blob.bin
        exec nginx -g 'daemon off;'
YAML
}

up() {
  write_compose

  if ! docker ps --format '{{.Names}}' | grep -qx "$DIND_NAME"; then
    docker rm -f "$DIND_NAME" >/dev/null 2>&1 || true
    docker run -d --name "$DIND_NAME" --privileged \
      -e DOCKER_TLS_CERTDIR= \
      -p "127.0.0.1:${DIND_PORT}:2375" \
      docker:dind --host=tcp://0.0.0.0:2375 >/dev/null
  fi

  # Wait for the inner daemon to accept connections.
  for _ in $(seq 1 60); do
    ddocker info >/dev/null 2>&1 && break
    sleep 1
  done
  ddocker info >/dev/null

  for img in "${SEED_IMAGES[@]}"; do
    ddocker image inspect "$img" >/dev/null 2>&1 && continue
    if docker image inspect "$img" >/dev/null 2>&1; then
      docker save "$img" | ddocker load >/dev/null
    else
      ddocker pull "$img" >/dev/null
    fi
  done
  for img in "${PULL_IMAGES[@]}"; do
    ddocker image inspect "$img" >/dev/null 2>&1 || ddocker pull "$img" >/dev/null
  done

  DOCKER_HOST="$DEMO_HOST" docker compose \
    -f "$DEMO_DIR/compose.yml" -p "$PROJECT" up -d --wait </dev/null >/dev/null

  # The Logs pane is about 30 rows tall. Wait for the api service to build up a
  # full screen of backlog, otherwise a cold start records against a mostly
  # empty pane with only two or three matches for the search to highlight.
  until [ "$(ddocker logs "${PROJECT}-api-1" 2>/dev/null | wc -l)" -ge 35 ]; do
    sleep 2
  done
  echo "ready: $PROJECT on $DEMO_HOST"
}

down() {
  docker rm -f -v "$DIND_NAME" >/dev/null 2>&1 || true
  rm -rf "$DEMO_DIR"
}

case "${1:-up}" in
  up) up ;;
  down) down ;;
  *) echo "usage: $0 [up|down]" >&2; exit 2 ;;
esac
