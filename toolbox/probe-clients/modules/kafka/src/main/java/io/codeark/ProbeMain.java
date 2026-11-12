package io.codeark;

import com.google.gson.JsonObject;
import java.util.Map;
import java.time.Duration;
import java.util.concurrent.TimeUnit;
import org.apache.kafka.clients.admin.Admin;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        try (var admin = Admin.create(Map.of("bootstrap.servers", "127.0.0.1:" + Probe.port(request),
                "default.api.timeout.ms",5000, "request.timeout.ms",3000))) {
            var cluster = admin.describeCluster();
            return !cluster.nodes().get(5,TimeUnit.SECONDS).isEmpty()
                    && !cluster.clusterId().get(5,TimeUnit.SECONDS).isBlank();
        }
    }
}
