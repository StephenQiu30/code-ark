package io.codeark;

import com.google.gson.JsonObject;
import io.minio.MinioClient;

public final class ProbeMain {
    public static void main(String[] args) {
        Probe.run(ProbeMain::check);
    }
    private static boolean check(JsonObject request) throws Exception {
        try (var client = MinioClient.builder().endpoint("http://127.0.0.1:" + Probe.port(request))
                .credentials(Probe.value(request,"username"),Probe.value(request,"password")).build()) {
            return client.listBuckets() != null;
        }
    }
}
