docker compose -f docker-compose-full.yml --profile backup run --rm backup
$date = ".\backups\" + (Get-Date).ToString("yyyy-MM-dd-hh-mm-ss") + ".db"
Move-Item -Path ".\backups\latest.db" -Destination $date
