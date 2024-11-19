#!/bin/bash

# Stop and remove container and image by name threads
docker stop threads
docker rm threads
docker rmi threads

# Build image
docker build -t threads .

# Run container
docker run -d --restart always -p 19998:19998 --name threads threads

# show logs
docker logs -f threads
