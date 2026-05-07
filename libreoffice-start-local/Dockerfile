FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    HOME=/tmp/libreoffice-home

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
      ca-certificates \
      fontconfig \
      fonts-dejavu \
      fonts-liberation \
      fonts-noto-cjk \
      libreoffice \
      libreoffice-l10n-zh-cn \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /data

ENTRYPOINT ["soffice", "--headless", "--nologo", "--nofirststartwizard", "--nodefault", "--nolockcheck"]
CMD ["--version"]
