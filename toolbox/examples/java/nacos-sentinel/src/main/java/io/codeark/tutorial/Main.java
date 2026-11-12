package io.codeark.tutorial;

import com.alibaba.csp.sentinel.Entry;
import com.alibaba.csp.sentinel.SphU;
import com.alibaba.csp.sentinel.slots.block.BlockException;
import com.alibaba.csp.sentinel.slots.block.RuleConstant;
import com.alibaba.csp.sentinel.slots.block.flow.FlowRule;
import com.alibaba.csp.sentinel.slots.block.flow.FlowRuleManager;
import com.alibaba.nacos.api.NacosFactory;
import com.sun.net.httpserver.HttpServer;
import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.*;
import java.time.Duration;
import java.util.*;

public final class Main {
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          String name = scope.name();
          System.setProperty("project.name", name);
          System.setProperty(
              "csp.sentinel.dashboard.server",
              scope.host("CODEARK_SENTINEL_HOST") + ":" + scope.port("CODEARK_SENTINEL_PORT"));
          Properties properties = new Properties();
          properties.setProperty(
              "serverAddr",
              scope.host("CODEARK_NACOS_HOST") + ":" + scope.port("CODEARK_NACOS_PORT"));
          properties.setProperty("username", scope.required("CODEARK_NACOS_USERNAME"));
          properties.setProperty("password", scope.required("CODEARK_NACOS_PASSWORD"));
          var config = NacosFactory.createConfigService(properties);
          var naming = NacosFactory.createNamingService(properties);
          var server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
          var rule = new FlowRule(name);
          rule.setGrade(RuleConstant.FLOW_GRADE_QPS);
          rule.setCount(1);
          FlowRuleManager.loadRules(List.of(rule));
          server.createContext(
              "/tutorial",
              exchange -> {
                int status = 200;
                Entry entry = null;
                try {
                  entry = SphU.entry(name);
                } catch (BlockException blocked) {
                  status = 429;
                } finally {
                  if (entry != null) entry.exit();
                }
                byte[] body = (status == 200 ? "ok" : "limited").getBytes();
                exchange.sendResponseHeaders(status, body.length);
                exchange.getResponseBody().write(body);
                exchange.close();
              });
          server.start();
          int port = server.getAddress().getPort();
          boolean registered = false, published = false;
          try {
            if (config.getConfig(name, "CODEARK_TUTORIAL", 3000) != null
                || !naming.getAllInstances(name, "CODEARK_TUTORIAL").isEmpty())
              throw new IllegalStateException("Teaching name already exists; choose a new run ID");
            published = config.publishConfig(name, "CODEARK_TUTORIAL", "codeark=true");
            if (!published) throw new IllegalStateException("Config publish failed");
            String value = null;
            for (int i = 0; i < 10; i++) {
              value = config.getConfig(name, "CODEARK_TUTORIAL", 1000);
              if ("codeark=true".equals(value)) break;
              Thread.sleep(200);
            }
            if (!"codeark=true".equals(value))
              throw new IllegalStateException("Config read mismatch");
            System.out.println("NACOS_CONFIG_OK");
            naming.registerInstance(name, "CODEARK_TUTORIAL", scope.host("CODEARK_APP_HOST"), port);
            registered = true;
            if (naming.getAllInstances(name, "CODEARK_TUTORIAL").isEmpty())
              throw new IllegalStateException("Registration not visible");
            System.out.println("NACOS_REGISTRATION_OK");
            var http = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(3)).build();
            int blocked = 0;
            for (int i = 0; i < 20; i++) {
              var response =
                  http.send(
                      HttpRequest.newBuilder(URI.create("http://127.0.0.1:" + port + "/tutorial"))
                          .timeout(Duration.ofSeconds(3))
                          .GET()
                          .build(),
                      HttpResponse.BodyHandlers.discarding());
              if (response.statusCode() == 429) blocked++;
            }
            if (blocked == 0) throw new IllegalStateException("No request was limited");
            System.out.println("SENTINEL_HTTP_429_OK");
          } finally {
            server.stop(0);
            if (registered)
              naming.deregisterInstance(
                  name, "CODEARK_TUTORIAL", scope.host("CODEARK_APP_HOST"), port);
            if (published) config.removeConfig(name, "CODEARK_TUTORIAL");
            naming.shutDown();
            config.shutDown();
            FlowRuleManager.loadRules(List.of());
          }
        });
  }
}
