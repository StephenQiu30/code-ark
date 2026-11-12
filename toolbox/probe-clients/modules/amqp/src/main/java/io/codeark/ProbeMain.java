package io.codeark;

import com.google.gson.JsonObject;
import com.rabbitmq.client.ConnectionFactory;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        ConnectionFactory client = new ConnectionFactory();
        client.setHost("127.0.0.1");
        client.setPort(Probe.port(request));
        client.setUsername(Probe.value(request,"username"));
        client.setPassword(Probe.value(request,"password"));
        client.setVirtualHost("/");
        client.setConnectionTimeout(3000);
        client.setHandshakeTimeout(3000);
        client.setAutomaticRecoveryEnabled(false);
        try (var conn = client.newConnection("codeark-readiness"); var channel = conn.createChannel()) {
            return channel.isOpen();
        }
    }
}
