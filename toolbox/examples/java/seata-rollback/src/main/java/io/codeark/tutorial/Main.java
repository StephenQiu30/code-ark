package io.codeark.tutorial;

import com.alibaba.druid.pool.DruidDataSource;
import io.seata.rm.RMClient;
import io.seata.rm.datasource.DataSourceProxy;
import io.seata.tm.TMClient;
import io.seata.tm.api.GlobalTransactionContext;
import java.nio.file.Files;
import java.sql.DriverManager;

public final class Main {
  static java.nio.file.Path createConfiguration(Scope scope) throws Exception {
    String endpoint = scope.host("CODEARK_SEATA_HOST")+":"+scope.port("CODEARK_SEATA_PORT");
    var directory=Files.createTempDirectory("codeark-seata-tutorial-");
    String configPath=directory.resolve("file.conf").toString().replace('\\','/');
    Files.writeString(directory.resolve("registry.conf"),"""
        registry {
          type = "file"
        }
        config {
          type = "file"
          file {
            name = "file:%s"
          }
        }
        """.formatted(configPath));
    Files.writeString(directory.resolve("file.conf"),"""
        service {
          vgroupMapping.codeark_tutorial = "default"
          default.grouplist = "%s"
          enableDegrade = false
          disableGlobalTransaction = false
        }
        client {
          rm.reportSuccessEnable = true
        }
        """.formatted(endpoint));
    return directory;
  }
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          String database = scope.database();
          String table = scope.name();
          var directory = createConfiguration(scope);
          var registry = directory.resolve("registry.conf");
          var config = directory.resolve("file.conf");
          System.setProperty("seata.config.name", "file:" + registry.toString().replace('\\', '/'));
          String url =
              "jdbc:mysql://"
                  + scope.host("CODEARK_SQL_HOST")
                  + ":"
                  + scope.port("CODEARK_SQL_PORT")
                  + "/"
                  + database
                  + "?connectTimeout=5000&socketTimeout=5000";
          String user = scope.required("CODEARK_SQL_USERNAME"),
              password = scope.required("CODEARK_SQL_PASSWORD");
          boolean verified = false;
          boolean tmStarted=false,rmStarted=false;
          var source = new DruidDataSource();
          source.setUrl(url);
          source.setUsername(user);
          source.setPassword(password);
          try (var direct = DriverManager.getConnection(url, user, password);
              var statement = direct.createStatement()) {
            statement.executeUpdate(
                "CREATE TABLE " + table + " (id INTEGER PRIMARY KEY,balance INTEGER NOT NULL)");
            statement.executeUpdate("INSERT INTO " + table + " VALUES(1,100)");
            statement.executeUpdate(
                "CREATE TABLE undo_log (branch_id BIGINT NOT NULL,xid VARCHAR(128) NOT NULL,context"
                    + " VARCHAR(128) NOT NULL,rollback_info LONGBLOB NOT NULL,log_status INT NOT"
                    + " NULL,log_created DATETIME(6) NOT NULL,log_modified DATETIME(6) NOT"
                    + " NULL,UNIQUE KEY ux_undo_log(xid,branch_id))");
            TMClient.init(scope.name(), "codeark_tutorial");
            tmStarted=true;
            RMClient.init(scope.name(), "codeark_tutorial");
            rmStarted=true;
            var proxy = new DataSourceProxy(source);
            var transaction = GlobalTransactionContext.getCurrentOrCreate();
            transaction.begin(30000, scope.name());
            try {
              try (var connection = proxy.getConnection();
                  var update =
                      connection.prepareStatement(
                          "UPDATE " + table + " SET balance=0 WHERE id=1")) {
                connection.setAutoCommit(false);
                update.executeUpdate();
                connection.commit();
              }
              transaction.rollback();
            } catch (Exception failure) {
              try {
                transaction.rollback();
              } catch (Exception ignored) {
              }
              throw failure;
            }
            try (var result =
                statement.executeQuery("SELECT balance FROM " + table + " WHERE id=1")) {
              if (!result.next() || result.getInt(1) != 100)
                throw new IllegalStateException("Rollback not verified");
            }
            verified = true;
            System.out.println("SEATA_ROLLBACK_OK");
          } finally {
            source.close();
            if(rmStarted)io.seata.core.rpc.netty.RmNettyRemotingClient.getInstance().destroy();
            if(tmStarted)io.seata.core.rpc.netty.TmNettyRemotingClient.getInstance().destroy();
            if (verified) {
              try (var direct = DriverManager.getConnection(url, user, password);
                  var statement = direct.createStatement()) {
                statement.executeUpdate("DROP TABLE " + table);
                statement.executeUpdate("DROP TABLE undo_log");
              }
            }
            Files.deleteIfExists(registry);
            Files.deleteIfExists(config);
            Files.deleteIfExists(directory);
          }
        });
  }
}
