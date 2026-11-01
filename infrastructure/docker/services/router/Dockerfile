# Pinned by its digest, which Dependabot updates, as the images of
# services/php/Dockerfile
FROM traefik:v3.7.15@sha256:0f6150202bc1fed72a7bd2107460eaa510e5b8a905f3fdb5b84f2c8619c33368

COPY traefik /etc/traefik

ARG PROJECT_NAME
RUN sed -i "s/{{ PROJECT_NAME }}/${PROJECT_NAME}/g" /etc/traefik/traefik.yaml

VOLUME [ "/etc/ssl/certs" ]
