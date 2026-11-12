package io.codeark.tutorial;

import com.rabbitmq.client.ConnectionFactory;
import java.nio.charset.StandardCharsets;

public final class Main {
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          var factory = new ConnectionFactory();
          factory.setHost(scope.host("CODEARK_MQ_HOST"));
          factory.setPort(scope.port("CODEARK_MQ_PORT"));
          factory.setUsername(scope.required("CODEARK_MQ_USERNAME"));
          factory.setPassword(scope.required("CODEARK_MQ_PASSWORD"));
          factory.setVirtualHost(scope.required("CODEARK_MQ_VHOST"));
          factory.setConnectionTimeout(5000);
          try (var connection = factory.newConnection();
              var channel = connection.createChannel()) {
            String queue = channel.queueDeclare().getQueue();
            byte[] payload = ("codeark-" + scope.runId()).getBytes(StandardCharsets.UTF_8);
            try {
              channel.basicPublish("", queue, null, payload);
              var message = channel.basicGet(queue, false);
              if (message == null || !java.util.Arrays.equals(payload, message.getBody()))
                throw new IllegalStateException("Message mismatch");
              channel.basicAck(message.getEnvelope().getDeliveryTag(), false);
              System.out.println("AMQP_SEND_RECEIVE_OK");
            } finally {
              channel.queueDelete(queue);
            }
          }
        });
  }
}
