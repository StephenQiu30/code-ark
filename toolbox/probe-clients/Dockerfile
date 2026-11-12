# Build context: toolbox/probe-clients. The release builder supplies the platform's locked
# Maven/Temurin 17 manifest from toolbox/image-builds/xxjob/recipe.json.
ARG BUILDER_IMAGE
FROM ${BUILDER_IMAGE} AS build
WORKDIR /src
COPY pom.xml pom.xml
COPY common common
COPY modules modules
RUN mvn -B -ntp package && mkdir /bundle && \
    for module in modules/*; do \
      dest="/bundle/${module#modules/}"; mkdir "$dest" && \
      cp "$module/target/probe.jar" "$dest/" && cp -R "$module/target/lib" "$dest/" || exit 1; \
    done

# Export / with BuildKit --output type=local,dest=.... The desktop runs the jars on the
# independently verified registry JRE. It checks the artifact lock before every launch.
FROM scratch AS artifacts
COPY --from=build /bundle/ /
