cd ..
docker compose down
yes | docker volume prune
docker compose build --no-cache
docker compose up --force-recreate
