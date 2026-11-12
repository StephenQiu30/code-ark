package io.codeark.tutorial;

import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.Map;
import org.apache.rocketmq.client.apis.*;
import org.apache.rocketmq.client.apis.consumer.*;

public final class Main {
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          var provider = ClientServiceProvider.loadService();
          String topic = scope.name();
          var config =
              ClientConfiguration.newBuilder()
                  .setEndpoints(scope.host("CODEARK_MQ_HOST") + ":" + scope.port("CODEARK_MQ_PORT"))
                  .setRequestTimeout(Duration.ofSeconds(5))
                  .build();
          try (var consumer =
                  provider
                      .newSimpleConsumerBuilder()
                      .setClientConfiguration(config)
                      .setConsumerGroup(scope.name())
                      .setAwaitDuration(Duration.ofSeconds(3))
                      .setSubscriptionExpressions(
                          Map.of(topic, new FilterExpression("*", FilterExpressionType.TAG)))
                      .build();
              var producer =
                  provider
                      .newProducerBuilder()
                      .setClientConfiguration(config)
                      .setTopics(topic)
                      .build()) {
            String body = "codeark-" + scope.runId();
            producer.send(
                provider
                    .newMessageBuilder()
                    .setTopic(topic)
                    .setKeys(scope.runId())
                    .setBody(body.getBytes(StandardCharsets.UTF_8))
                    .build());
            boolean received = false;
            long deadline = System.nanoTime() + Duration.ofSeconds(20).toNanos();
            while (!received && System.nanoTime() < deadline) {
              for (var message : consumer.receive(1, Duration.ofSeconds(10))) {
                var buffer = message.getBody();
                byte[] bytes = new byte[buffer.remaining()];
                buffer.get(bytes);
                if (body.equals(new String(bytes, StandardCharsets.UTF_8))) {
                  consumer.ack(message);
                  received = true;
                }
              }
            }
            if (!received) throw new IllegalStateException("Message not received");
            System.out.println("ROCKETMQ_PROXY_SEND_RECEIVE_OK");
          }
        });
  }
}
