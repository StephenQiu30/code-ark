package io.codeark.tutorial;

import java.time.Duration;
import java.util.*;
import java.util.concurrent.TimeUnit;
import org.apache.kafka.clients.admin.*;
import org.apache.kafka.clients.consumer.*;
import org.apache.kafka.clients.producer.*;
import org.apache.kafka.common.serialization.*;

public final class Main {
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          String topic = scope.name();
          String endpoint = scope.host("CODEARK_MQ_HOST") + ":" + scope.port("CODEARK_MQ_PORT");
          Properties adminConfig = new Properties();
          adminConfig.put("bootstrap.servers", endpoint);
          adminConfig.put("request.timeout.ms", "5000");
          try (var admin = AdminClient.create(adminConfig)) {
            boolean created = false;
            try {
              admin
                  .createTopics(List.of(new NewTopic(topic, 1, (short) 1)))
                  .all()
                  .get(10, TimeUnit.SECONDS);
              created = true;
              Properties producerConfig = new Properties();
              producerConfig.putAll(adminConfig);
              producerConfig.put("key.serializer", StringSerializer.class.getName());
              producerConfig.put("value.serializer", StringSerializer.class.getName());
              producerConfig.put("delivery.timeout.ms", "10000");
              try (var producer = new KafkaProducer<String, String>(producerConfig)) {
                producer
                    .send(new ProducerRecord<>(topic, scope.runId(), "codeark-" + scope.runId()))
                    .get(10, TimeUnit.SECONDS);
              }
              Properties consumerConfig = new Properties();
              consumerConfig.putAll(adminConfig);
              consumerConfig.put("group.id", scope.name());
              consumerConfig.put("key.deserializer", StringDeserializer.class.getName());
              consumerConfig.put("value.deserializer", StringDeserializer.class.getName());
              consumerConfig.put("auto.offset.reset", "earliest");
              consumerConfig.put("enable.auto.commit", "false");
              boolean received = false;
              try (var consumer = new KafkaConsumer<String, String>(consumerConfig)) {
                consumer.subscribe(List.of(topic));
                long deadline = System.nanoTime() + Duration.ofSeconds(20).toNanos();
                while (!received && System.nanoTime() < deadline) {
                  for (var record : consumer.poll(Duration.ofMillis(500)))
                    if (("codeark-" + scope.runId()).equals(record.value())) received = true;
                }
              }
              if (!received) throw new IllegalStateException("Message not received");
              System.out.println("KAFKA_SEND_RECEIVE_OK");
            } finally {
              if (created) admin.deleteTopics(List.of(topic)).all().get(10, TimeUnit.SECONDS);
            }
          }
        });
  }
}
