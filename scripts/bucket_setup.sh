#!/bin/sh

set -e
/usr/bin/mc alias set myminio http://s3storage:9000 ${MINIO_ROOT_USER} ${MINIO_ROOT_PASSWORD}
/usr/bin/mc mb myminio/${MINIO_BUCKET_NAME}
/usr/bin/mc policy set public myminio/${MINIO_BUCKET_NAME}
exit 0
