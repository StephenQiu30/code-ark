package io.codeark;

import com.google.gson.JsonObject;
import com.alibaba.nacos.api.NacosFactory;
import java.util.Properties;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        Properties properties = new Properties();
        properties.setProperty("serverAddr","127.0.0.1:" + Probe.port(request));
        properties.setProperty("username",Probe.value(request,"username"));
        properties.setProperty("password",Probe.value(request,"password"));
        properties.setProperty("enableRemoteSyncConfig","true");
        properties.setProperty("enableRemoteSyncConfigOnStartup","true");
        var client = NacosFactory.createConfigService(properties);
        try {
            client.getConfig("codeark-readiness","DEFAULT_GROUP",4000);
            return "UP".equals(client.getServerStatus());
        } finally {
            client.shutDown();
        }
    }
}
