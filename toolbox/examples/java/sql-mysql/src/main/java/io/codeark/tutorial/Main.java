package io.codeark.tutorial;

import java.sql.DriverManager;

public final class Main {
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          String table = scope.name();
          String url =
              "jdbc:mysql://"
                  + scope.host("CODEARK_SQL_HOST")
                  + ":"
                  + scope.port("CODEARK_SQL_PORT")
                  + "/"
                  + scope.database()
                  + "?connectTimeout=5000&socketTimeout=5000";
          try (var connection =
                  DriverManager.getConnection(
                      url,
                      scope.required("CODEARK_SQL_USERNAME"),
                      scope.required("CODEARK_SQL_PASSWORD"));
              var statement = connection.createStatement()) {
            statement.executeUpdate(
                "CREATE TABLE " + table + " (id INTEGER PRIMARY KEY, note VARCHAR(64))");
            try {
              statement.executeUpdate("INSERT INTO " + table + " VALUES (1,'codeark tutorial')");
              try (var result =
                  statement.executeQuery("SELECT note FROM " + table + " WHERE id=1")) {
                if (!result.next() || !"codeark tutorial".equals(result.getString(1)))
                  throw new IllegalStateException("Query mismatch");
              }
              System.out.println("MYSQL_QUERY_OK");
            } finally {
              statement.executeUpdate("DROP TABLE " + table);
            }
          }
        });
  }
}
