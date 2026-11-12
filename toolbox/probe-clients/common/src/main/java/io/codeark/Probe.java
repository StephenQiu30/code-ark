package io.codeark;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.OutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.*;

public final class Probe {
    public interface Check { boolean run(JsonObject input) throws Exception; }
    private Probe() {}
    public static int port(JsonObject input) {
        int port = input.get("port").getAsInt();
        if (port < 1 || port > 65535) throw new IllegalArgumentException();
        return port;
    }
    public static String value(JsonObject input, String key) {
        return input.get(key).getAsString();
    }
    public static void run(Check check) {
        PrintStream response = System.out;
        // Some third-party clients print server replies and credentials, even on stdout.
        PrintStream quiet = new PrintStream(OutputStream.nullOutputStream());
        System.setOut(quiet);
        System.setErr(quiet);
        String probe = "", component = "";
        boolean success = false;
        String failure = "ProtocolUnavailable";
        ExecutorService worker = Executors.newSingleThreadExecutor();
        try {
            byte[] bytes = System.in.readNBytes(16385);
            if (bytes.length > 16384) throw new IllegalArgumentException();
            JsonObject input = JsonParser.parseString(new String(bytes, StandardCharsets.UTF_8)).getAsJsonObject();
            probe = id(input,"probeId");
            component = id(input,"componentId");
            success = worker.submit(() -> check.run(input)).get(12, TimeUnit.SECONDS);
        } catch (ExecutionException execution) {
            if (execution.getCause() instanceof LinkageError) failure = "ProbeClientIncompatible";
        } catch (LinkageError incompatible) {
            failure = "ProbeClientIncompatible";
        } catch (Exception ignored) {
            // Emit only fixed machine-readable codes. The process owns its client threads.
        } finally {
            worker.shutdownNow();
        }
        JsonObject result = new JsonObject();
        result.addProperty("probeId", probe);
        result.addProperty("componentId", component);
        result.addProperty("success", success);
        result.addProperty("errorCode", success ? null : failure);
        result.addProperty("warningCode", (String)null);
        result.addProperty("sampledAt", System.currentTimeMillis());
        Gson gson = new GsonBuilder().serializeNulls().create();
        response.println(gson.toJson(result));
        response.flush();
        Runtime.getRuntime().halt(0);
    }
    private static String id(JsonObject obj, String key) {
        String id = value(obj,key);
        if (!id.matches("[A-Za-z0-9_-]{1,96}")) throw new IllegalArgumentException();
        return id;
    }
}
