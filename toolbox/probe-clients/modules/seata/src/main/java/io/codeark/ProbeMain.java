package io.codeark;

import com.google.gson.JsonObject;
import io.seata.core.rpc.netty.TmNettyRemotingClient;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        String endpoint = "127.0.0.1:" + Probe.port(request);
        var client = TmNettyRemotingClient.getInstance("codeark-probe","codeark_readonly");
        try {
            client.init();
            return io.seata.core.rpc.netty.RegistrationProbe.registered(client, endpoint);
        } finally {
            client.destroy();
        }
    }
}
