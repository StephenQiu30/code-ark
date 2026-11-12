ARG ES_VERSION=9.3.0
FROM docker.elastic.co/elasticsearch/elasticsearch:${ES_VERSION}

ARG IK_VERSION=9.3.0
# Install IK Analysis plugin if not already present
RUN if [ ! -d "/usr/share/elasticsearch/plugins/analysis-ik" ]; then \
      bin/elasticsearch-plugin install --batch https://release.infinilabs.com/analysis-ik/stable/elasticsearch-analysis-ik-${IK_VERSION}.zip; \
    else \
      echo "IK Analysis plugin already installed, skipping..."; \
    fi
