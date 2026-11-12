package io.codeark.tutorial;

import java.net.http.*;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.Base64;

public final class Main {
  private static HttpResponse<String> send(
      HttpClient client, Scope scope, String path, String method, String body) throws Exception {
    var endpoint = scope.http("CODEARK_HTTP_ENDPOINT");
    var request =
        HttpRequest.newBuilder(endpoint.resolve(path))
            .timeout(Duration.ofSeconds(10))
            .header("Content-Type", "application/json");
    String username = System.getenv("CODEARK_HTTP_USERNAME"),
        password = System.getenv("CODEARK_HTTP_PASSWORD");
    if (username != null && password != null)
      request.header(
          "Authorization",
          "Basic "
              + Base64.getEncoder()
                  .encodeToString((username + ":" + password).getBytes(StandardCharsets.UTF_8)));
    request.method(
        method,
        body == null
            ? HttpRequest.BodyPublishers.noBody()
            : HttpRequest.BodyPublishers.ofString(body));
    return client.send(request.build(), HttpResponse.BodyHandlers.ofString());
  }

  public static void main(String[] args) {
    Scope.run(
        scope -> {
          var client = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(5)).build();
          if (args.length != 1) throw new IllegalArgumentException("Choose elastic or prometheus");
          if ("prometheus".equals(args[0])) {
            var result = send(client, scope, "/api/v1/query?query=vector%281%29", "GET", null);
            if (result.statusCode() != 200
                || !result.body().contains("\"success\"")
                || !result.body().contains("\"1\""))
              throw new IllegalStateException("Metrics query failed");
            System.out.println("PROMETHEUS_QUERY_OK");
          } else if ("elastic".equals(args[0])) {
            String index = scope.name();
            boolean created = false;
            try {
              var creation = send(client, scope, "/" + index, "PUT", "{}");
              if (creation.statusCode() != 200)
                throw new IllegalStateException("Index creation failed");
              created = true;
              var put =
                  send(
                      client,
                      scope,
                      "/" + index + "/_doc/1?refresh=wait_for",
                      "PUT",
                      "{\"message\":\"codeark\"}");
              if (put.statusCode() != 201) throw new IllegalStateException("Index write failed");
              var query =
                  send(
                      client,
                      scope,
                      "/" + index + "/_search",
                      "POST",
                      "{\"query\":{\"match\":{\"message\":\"codeark\"}}}");
              if (query.statusCode() != 200 || !query.body().contains("codeark"))
                throw new IllegalStateException("Search mismatch");
              System.out.println("ELASTIC_QUERY_OK");
            } finally {
              if (created) {
                var cleanup = send(client, scope, "/" + index, "DELETE", null);
                if (cleanup.statusCode() != 200)
                  throw new IllegalStateException("Teaching index cleanup failed");
              }
            }
          } else throw new IllegalArgumentException("Unknown query mode");
        });
  }
}
