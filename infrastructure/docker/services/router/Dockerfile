# Pinned by its digest, which Dependabot updates, as the images of
# services/php/Dockerfile
FROM traefik:v3.7.14@sha256:575fa15b135078fe5e50aa847987d96dbddd7b093c172429618404df73f3fa7c

COPY traefik /etc/traefik

ARG PROJECT_NAME
RUN sed -i "s/{{ PROJECT_NAME }}/${PROJECT_NAME}/g" /etc/traefik/traefik.yaml

VOLUME [ "/etc/ssl/certs" ]
