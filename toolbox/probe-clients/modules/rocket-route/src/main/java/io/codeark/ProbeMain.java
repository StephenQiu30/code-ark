package io.codeark;

import com.google.gson.JsonObject;
import org.apache.rocketmq.tools.admin.DefaultMQAdminExt;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        var client = new DefaultMQAdminExt(4000);
        client.setNamesrvAddr("127.0.0.1:" + Probe.port(request));
        client.setInstanceName("codeark-" + java.util.UUID.randomUUID());
        try {
            client.start();
            var info = client.examineBrokerClusterInfo();
            return info.getBrokerAddrTable().values().stream().anyMatch(b -> !b.getBrokerAddrs().isEmpty());
        } finally {
            client.shutdown();
        }
    }
}
