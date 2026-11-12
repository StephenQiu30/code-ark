ARG BUILDER_IMAGE
ARG RUNTIME_IMAGE
FROM ${BUILDER_IMAGE} AS build
ARG SOURCE_URL
ARG SOURCE_SHA256
WORKDIR /source
RUN curl --fail --location --silent --show-error "$SOURCE_URL" -o /tmp/source.tar.gz \
    && printf '%s  /tmp/source.tar.gz\n' "$SOURCE_SHA256" | sha256sum --check --strict \
    && tar -xzf /tmp/source.tar.gz --strip-components=1
ENV CGO_ENABLED=0 GOMAXPROCS=2 GOMEMLIMIT=1GiB GOFLAGS=-p=2
RUN --mount=type=cache,target=/go/pkg/mod --mount=type=cache,target=/root/.cache/go-build \
    attempts=0; until go mod download; do attempts=$((attempts + 1)); [ "$attempts" -lt 3 ] || exit 1; sleep 2; done; \
    go build -trimpath -ldflags='-s -w' -o /out/minio .

FROM ${RUNTIME_IMAGE}
COPY --from=build /out/minio /usr/local/bin/minio
COPY --from=build /source/LICENSE /usr/share/licenses/minio/LICENSE
USER 1000:1000
EXPOSE 9000 9001
ENTRYPOINT ["/usr/local/bin/minio"]
CMD ["server", "/data", "--console-address", ":9001"]
