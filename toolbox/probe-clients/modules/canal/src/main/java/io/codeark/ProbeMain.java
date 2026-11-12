package io.codeark;

import com.google.gson.JsonObject;
import com.alibaba.otter.canal.client.impl.SimpleCanalConnector;
import java.net.InetSocketAddress;
import java.util.concurrent.TimeUnit;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        // Canal uses fixed client ID 1001. Never attach to the application's destination.
        var client = new SimpleCanalConnector(new InetSocketAddress("127.0.0.1", Probe.port(request)),
                "", "", "codeark_probe");
        client.setRollbackOnConnect(false);
        client.setRollbackOnDisConnect(false);
        client.setSoTimeout(4000);
        client.connect();
        try {
            client.subscribe();
            var message = client.getWithoutAck(1,500L,TimeUnit.MILLISECONDS);
            if (message == null) return false;
            if (message.getId() >= 0) client.rollback(message.getId());
            return true;
        } finally {
            client.disconnect();
        }
    }
}
