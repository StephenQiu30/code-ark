package io.codeark;

import com.google.gson.JsonObject;
import org.apache.rocketmq.client.apis.ClientConfiguration;
import org.apache.rocketmq.client.apis.ClientServiceProvider;
import java.time.Duration;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        var config = ClientConfiguration.newBuilder().setEndpoints("127.0.0.1:" + Probe.port(request))
                .setRequestTimeout(Duration.ofSeconds(4)).build();
        // Building a producer with a predeclared system topic waits for route discovery.
        // No messages or topics are created by the probe.
        try (var producer = ClientServiceProvider.loadService().newProducerBuilder()
                .setClientConfiguration(config).setTopics("TBW102").build()) {
            return producer != null;
        }
    }
}
