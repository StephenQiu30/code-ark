# Code Ark: Ready-to-Run Docker Compose Development Environments

[![GitHub stars](https://img.shields.io/github/stars/StephenQiu30/code-ark?style=flat-square&logo=github)](https://github.com/StephenQiu30/code-ark)
[![GitHub last commit](https://img.shields.io/github/last-commit/StephenQiu30/code-ark?style=flat-square)](https://github.com/StephenQiu30/code-ark/commits/main)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](./LICENSE)
[![Docker Compose](https://img.shields.io/badge/Docker-Compose-2496ED?style=flat-square&logo=docker&logoColor=white)](https://docs.docker.com/compose/)

**Code Ark** is an open-source collection of ready-to-run Docker Compose
environments for local development and integration testing. Start Elasticsearch,
Kafka, MySQL, PostgreSQL, Redis, RabbitMQ, MinIO, Nacos, RocketMQ, Prometheus,
Grafana, and other infrastructure without installing each service manually.

[中文文档](./README.md) · [Service catalog](#docker-service-catalog) ·
[Quick start](#quick-start) · [Contributing](./CONTRIBUTING.md)

## Why Code Ark?

- **Ready to run:** each stack includes a working `docker-compose.yml`.
- **Start only what you need:** services are organized as independent projects.
- **Explicit configuration:** public settings are documented in `.env.example` files.
- **Persistent local data:** each service documents its volumes and cleanup behavior.
- **Cross-platform:** works with Linux, macOS Docker Desktop, and Windows WSL 2.
- **Team friendly:** consistent commands make developer onboarding reproducible.

> Code Ark is designed for local development and testing. The default
> configurations are not production-ready.

## Docker service catalog

| Category | Service | Directory | Default ports |
| --- | --- | --- | --- |
| Search and logs | Elasticsearch, Kibana, Logstash | [`elastic-start-local`](./elastic-start-local/README.md) | `9200`, `5601`, `5044` |
| Event streaming | Kafka and Kafka UI | [`kafka-start-local`](./kafka-start-local/README.md) | `9092`, `19000` |
| Document processing | LibreOffice headless | [`libreoffice-start-local`](./libreoffice-start-local/README.md) | — |
| Object storage | MinIO | [`minio-start-local`](./minio-start-local/README.md) | `9000`, `9001` |
| Observability | Prometheus and Grafana | [`monitoring-start-local`](./monitoring-start-local/README.md) | `19090`, `13000` |
| Database | MySQL 8 | [`mysql-start-lcoal`](./mysql-start-lcoal/README.md) | `3306` |
| Service discovery | Nacos | [`nacos-start-local`](./nacos-start-local/README.md) | `8840`, `8848`, `9848-9850` |
| OCR | PaddleOCR API | [`ocr-start-local`](./ocr-start-local/README.md) | `18868` |
| Database | PostgreSQL | [`pgsql-start-local`](./pgsql-start-local/README.md) | `5432` |
| Message broker | RabbitMQ | [`rabbitmq-start-lcoal`](./rabbitmq-start-lcoal/README.md) | `5672`, `15672` |
| Cache | Redis | [`redis-start-local`](./redis-start-local/README.md) | `6379` |
| Message queue | RocketMQ | [`rocketmq-start-local`](./rocketmq-start-local/README.md) | `15876`, `15911`, `18180` |
| Distributed transaction | Seata | [`seata-start-local`](./seata-start-local/README.md) | `7091`, `8091` |
| Traffic protection | Sentinel | [`sentinel-start-local`](./sentinel-start-local/README.md) | `8858`, `8719` |
| Job scheduling | XXL-Job | [`xxjob-start-local`](./xxjob-start-local/README.md) | `18081` |

The historical directory names `mysql-start-lcoal` and `rabbitmq-start-lcoal`
are retained for compatibility.

## Quick start

### Requirements

- Docker Engine or Docker Desktop
- Docker Compose v2
- Git

```bash
git clone https://github.com/StephenQiu30/code-ark.git
cd code-ark
```

Start PostgreSQL locally:

```bash
cd pgsql-start-local
cp .env.example .env
docker compose up -d
docker compose ps
```

Start the complete Elasticsearch, Kibana, and Logstash stack:

```bash
cd elastic-start-local
cp .env.example .env
./start.sh
```

Open the README inside a service directory for its ports, credentials, resource
settings, and data persistence instructions.

## Common commands

Run these commands inside a service directory:

```bash
docker compose up -d
docker compose ps
docker compose logs -f
docker compose stop
docker compose down
docker compose down -v
```

`docker compose down -v` deletes named volumes and their local data.

## Configuration and security

1. Copy `.env.example` to `.env` in the selected service directory.
2. Adjust image versions, ports, credentials, and memory limits when needed.
3. Never commit a real `.env`, password, token, or API key.
4. Treat these stacks as development defaults, not hardened production deployments.

## Frequently asked questions

### Can I run only one service?

Yes. Each directory is an independent Docker Compose project, so you can start
only the infrastructure required by your application.

### How do I run Elasticsearch and Kibana locally with Docker?

Enter `elastic-start-local`, copy `.env.example` to `.env`, and run `./start.sh`.
The script checks Docker memory, builds the IK analysis plugin, and waits for the
stack to become healthy.

### Is local data preserved when containers stop?

Yes. `docker compose stop` and `docker compose down` preserve named volumes.
Adding `-v` deletes those volumes.

### Can I use these files in production?

Not as-is. Production deployments require proper secret management, TLS,
backups, access controls, monitoring, high availability, and capacity planning.

## Contributing

Contributions for new development stacks, documentation, and fixes are welcome.
Read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a pull request.

If Code Ark helps your workflow, consider starring the repository so other
developers can discover it.

## License

Code Ark is available under the [MIT License](./LICENSE).
