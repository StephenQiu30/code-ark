# Sentinel 本地启动

> Sentinel 是阿里开源的流量防卫组件，用于流量控制、熔断降级、系统负载保护等

## 服务说明

- **镜像**: bladex/sentinel-dashboard:1.8.8
- **版本**: 1.8.8
- **默认账号/密码**: sentinel / sentinel

## 端口说明

| 服务 | 端口 | 说明 |
|------|:----:|------|
| Dashboard | 8858 | Sentinel 控制台 |
| API | 8719 | 客户端通信端口 |

## 快速启动

### 使用启动脚本（推荐）

```bash
./start.sh
```

### 使用 Docker Compose

```bash
docker compose up -d
```

## 访问控制台

启动成功后，访问 http://localhost:8858

默认账号密码：
- 用户名: `sentinel`
- 密码: `sentinel`

## 常用命令

```bash
# 查看日志
docker compose logs -f

# 停止服务
docker compose down

# 停止并删除数据
docker compose down -v
rm -rf ./sentinel

# 重启服务
docker compose restart
```

## 客户端接入

在 Spring Cloud/Boot 项目中添加依赖：

```xml
<dependency>
    <groupId>com.alibaba.cloud</groupId>
    <artifactId>spring-cloud-starter-alibaba-sentinel</artifactId>
</dependency>
```

配置文件：

```yaml
spring:
  cloud:
    sentinel:
      transport:
        dashboard: localhost:8858
        port: 8719
      eager: true
```

## 核心功能

- **流量控制**: 限制 QPS、并发线程数
- **熔断降级**: 慢调用比例、异常比例、异常数
- **系统规则**: CPU、RT、线程数、入口 QPS
- **授权规则**: 黑白名单控制
- **流控规则**: 基于调用关系的流量控制

## 数据持久化

当前配置为 **单机模式**，规则仅保存在内存中，重启后丢失。

如需规则持久化，可以使用以下方式：

1. **推模式**: 结合 Nacos/Apollo 实现配置中心
2. **拉模式**: 结合数据库/文件实现持久化
3. **企业版**: Sentinel 企业版支持多种数据源

## 相关链接

- [官方文档](https://sentinelguard.io/zh-cn/)
- [GitHub](https://github.com/alibaba/Sentinel)
- [Spring Cloud Alibaba](https://github.com/alibaba/spring-cloud-alibaba/wiki/Sentinel)