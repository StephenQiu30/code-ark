package io.codeark.tutorial;

import java.net.URI;
import java.util.Map;

/** Teaching scope only. Never infer endpoints or credentials from repository defaults. */
public final class Scope {
  private final Map<String, String> env;

  public Scope(Map<String, String> env) {
    this.env = Map.copyOf(env);
  }

  public String required(String key) {
    String value = env.get(key);
    if (value == null || value.isBlank())
      throw new IllegalArgumentException("Missing teaching input");
    return value;
  }

  public void confirm() {
    if (!"YES".equals(required("CODEARK_TUTORIAL_CONFIRM")))
      throw new IllegalArgumentException("Teaching consent required");
    runId();
  }

  public String credential(String key) {
    String value = env.get(key);
    if (value == null)
      throw new IllegalArgumentException(
          "Credential input must be explicit; empty is allowed only when intentionally configured");
    return value;
  }

  public String runId() {
    String id = required("CODEARK_RUN_ID");
    if (!id.matches("[a-z0-9]{8,24}")) throw new IllegalArgumentException("Invalid run ID");
    return id;
  }

  public String name() {
    return "codeark_tutorial_" + runId();
  }

  public String host(String key) {
    String host = required(key);
    if (host.equals("127.0.0.1") || host.equals("localhost")) return host;
    if ("container".equals(env.get("CODEARK_ENDPOINT_MODE"))
        && !required("CODEARK_DOCKER_NETWORK").isBlank()
        && host.matches("[a-z][a-z0-9_-]{0,62}")) return host;
    throw new IllegalArgumentException(
        "Only explicit loopback or joined container-network endpoints are allowed");
  }

  public int port(String key) {
    int value = Integer.parseInt(required(key));
    if (value < 1 || value > 65535) throw new IllegalArgumentException("Invalid port");
    return value;
  }

  public URI http(String key) {
    URI uri = URI.create(required(key));
    if (!"http".equals(uri.getScheme()) || uri.getUserInfo() != null || uri.getHost() == null)
      throw new IllegalArgumentException("Invalid local HTTP endpoint");
    String host = uri.getHost();
    if (!(host.equals("127.0.0.1")
        || host.equals("localhost")
        || ("container".equals(env.get("CODEARK_ENDPOINT_MODE"))
            && host.matches("[a-z][a-z0-9_-]{0,62}")
            && !required("CODEARK_DOCKER_NETWORK").isBlank())))
      throw new IllegalArgumentException("Remote endpoint refused");
    return uri;
  }

  public String database() {
    String database = required("CODEARK_SQL_DATABASE");
    if (!database.equals(name()))
      throw new IllegalArgumentException("Dedicated teaching database must match run ID");
    return database;
  }

  public static void run(Checked action) {
    try {
      Scope scope = new Scope(System.getenv());
      scope.confirm();
      action.run(scope);
    } catch (Exception error) {
      System.err.println("EXAMPLE_FAILED " + error.getClass().getSimpleName());
      System.exit(1);
    }
  }

  @FunctionalInterface
  public interface Checked {
    void run(Scope scope) throws Exception;
  }
}
