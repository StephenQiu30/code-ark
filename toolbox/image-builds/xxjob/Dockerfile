ARG BUILDER_IMAGE
ARG RUNTIME_IMAGE
FROM ${BUILDER_IMAGE} AS build
ARG SOURCE_URL
ARG SOURCE_SHA256
ENV MAVEN_OPTS=-Xmx768m
WORKDIR /source
RUN curl --fail --location --silent --show-error "$SOURCE_URL" -o /tmp/source.tar.gz \
    && printf '%s  /tmp/source.tar.gz\n' "$SOURCE_SHA256" | sha256sum --check --strict \
    && tar -xzf /tmp/source.tar.gz --strip-components=1
RUN mvn --batch-mode --no-transfer-progress -pl xxl-job-admin -am package -DskipTests

FROM ${RUNTIME_IMAGE}
WORKDIR /app
COPY --from=build /source/xxl-job-admin/target/xxl-job-admin-*.jar /app/xxl-job-admin.jar
COPY --from=build /source/LICENSE /usr/share/licenses/xxl-job/LICENSE
COPY entrypoint.sh /app/entrypoint.sh
USER 1000:1000
EXPOSE 8080
ENTRYPOINT ["/bin/sh", "/app/entrypoint.sh"]
