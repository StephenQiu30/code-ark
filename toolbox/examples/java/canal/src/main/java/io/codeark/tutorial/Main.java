package io.codeark.tutorial;

import com.alibaba.otter.canal.client.impl.SimpleCanalConnector;
import java.net.InetSocketAddress;
import java.time.Duration;
import java.util.concurrent.TimeUnit;

public final class Main {
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          String destination = scope.required("CODEARK_CANAL_DESTINATION");
          if (!destination.equals(scope.name()))
            throw new IllegalArgumentException("Dedicated tutorial destination required");
          var client =
              new SimpleCanalConnector(
                  new InetSocketAddress(
                      scope.host("CODEARK_CANAL_HOST"), scope.port("CODEARK_CANAL_PORT")),
                  scope.credential("CODEARK_CANAL_USERNAME"),
                  scope.credential("CODEARK_CANAL_PASSWORD"),
                  destination);
          client.setRollbackOnConnect(false);
          client.setRollbackOnDisConnect(false);
          client.setSoTimeout(5000);
          client.connect();
          try {
            client.subscribe(scope.database() + "\\..*");
            boolean seen = false;
            long deadline = System.nanoTime() + Duration.ofSeconds(30).toNanos();
            while (!seen && System.nanoTime() < deadline) {
              var message = client.getWithoutAck(100, 500L, TimeUnit.MILLISECONDS);
              if (message == null) continue;
              try {
                seen =
                    message.getEntries().stream()
                        .anyMatch(
                            entry -> scope.database().equals(entry.getHeader().getSchemaName()));
                if (message.getId() >= 0) client.ack(message.getId());
              } catch (Exception error) {
                if (message.getId() >= 0) client.rollback(message.getId());
                throw error;
              }
            }
            if (!seen) throw new IllegalStateException("No event in the dedicated teaching schema");
            System.out.println("CANAL_EVENT_OK");
          } finally {
            client.disconnect();
          }
        });
  }
}
