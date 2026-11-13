package io.codeark;

import apache.rocketmq.v2.ClientType;
import apache.rocketmq.v2.Code;
import apache.rocketmq.v2.TelemetryCommand;
import com.google.gson.JsonObject;
import org.apache.rocketmq.client.apis.ClientConfiguration;
import org.apache.rocketmq.client.apis.ClientServiceProvider;
import org.apache.rocketmq.client.java.impl.ClientImpl;
import org.apache.rocketmq.shaded.io.grpc.stub.StreamObserver;
import java.time.Duration;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        var config = ClientConfiguration.newBuilder().setEndpoints("127.0.0.1:" + Probe.port(request))
                // This owned local preset exposes the Proxy's plaintext HTTP/2 listener.
                .enableSsl(false).setRequestTimeout(Duration.ofSeconds(4)).build();
        // Proxy 5.5 rejects system topics, including TBW102. Verify a real SDK
        // settings exchange and heartbeat without creating topics or sending messages.
        // The separate NameServer probe verifies registered broker routes.
        try (var producer = ClientServiceProvider.loadService().newProducerBuilder()
                .setClientConfiguration(config).build()) {
            var client = (ClientImpl) producer;
            var settings = new CompletableFuture<Boolean>();
            var stream = client.telemetry(client.getEndpoints(), new StreamObserver<TelemetryCommand>() {
                public void onNext(TelemetryCommand command) {
                    settings.complete(command.hasSettings()
                            && command.getStatus().getCode() == Code.OK
                            && command.getSettings().getClientType() == ClientType.PRODUCER
                            && command.getSettings().hasPublishing());
                }
                public void onError(Throwable error) { settings.complete(false); }
                public void onCompleted() { settings.complete(false); }
            });
            try {
                stream.onNext(client.settingsCommand());
                if (!settings.get(4, TimeUnit.SECONDS)) return false;
                var heartbeat = client.getClientManager().heartbeat(client.getEndpoints(),
                        client.wrapHeartbeatRequest(), Duration.ofSeconds(4)).get(4, TimeUnit.SECONDS);
                return heartbeat.getStatus().getCode() == Code.OK;
            } finally {
                try {
                    // Empty-topic clients have no route endpoints for the SDK's
                    // automatic termination. Remove this exact ephemeral session.
                    client.getClientManager().notifyClientTermination(client.getEndpoints(),
                            client.wrapNotifyClientTerminationRequest(), Duration.ofSeconds(2))
                            .get(2, TimeUnit.SECONDS);
                } finally {
                    stream.onCompleted();
                }
            }
        }
    }
}
