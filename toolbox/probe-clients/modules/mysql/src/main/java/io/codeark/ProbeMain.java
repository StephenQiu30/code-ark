package io.codeark;

import com.google.gson.JsonObject;
import java.sql.*;
import java.util.*;

public final class ProbeMain {
    public static void main(String[] args) { Probe.run(ProbeMain::check); }

    private static boolean check(JsonObject input) throws Exception {
        String purpose = Probe.value(input,"purpose");
        String db = Probe.value(input,"database");
        if (!db.matches("[A-Za-z0-9_]{0,64}")) return false;
        Properties options = new Properties();
        options.setProperty("user",Probe.value(input,"username"));
        options.setProperty("password",Probe.value(input,"password"));
        options.setProperty("allowPublicKeyRetrieval","true");
        options.setProperty("connectTimeout","3000");
        options.setProperty("socketTimeout","4000");
        String url = "jdbc:mysql://127.0.0.1:" + Probe.port(input) + "/" + db;
        try (Connection c = DriverManager.getConnection(url,options); Statement s = c.createStatement()) {
            c.setReadOnly(true);
            if (!value(s,"SELECT @@server_uuid").equals(Probe.value(input,"serverUuid"))) return false;
            return switch (purpose) {
                case "xxjob" -> verifyXxl(s);
                case "seata" -> schema(s, "seata") && "4".equals(value(s,"SELECT COUNT(*) FROM distributed_lock WHERE lock_key IN ('AsyncCommitting','RetryCommitting','RetryRollbacking','TxTimeoutCheck')"));
                case "canal" -> verifyReplication(s);
                default -> false;
            };
        }
    }
    private static String value(Statement s,String sql) throws SQLException {
        try (ResultSet rs=s.executeQuery(sql)) { return rs.next() ? rs.getString(1) : ""; }
    }
    private static boolean metadata(Statement s, String name, String query, int columns) throws Exception {
        Set<String> missing = new HashSet<>();
        try (var raw = Objects.requireNonNull(ProbeMain.class.getResourceAsStream("/"+name+".tsv"));
             var input = new java.io.BufferedReader(new java.io.InputStreamReader(raw,java.nio.charset.StandardCharsets.UTF_8))) {
            input.lines().forEach(missing::add);
        }
        if (missing.isEmpty()) return false;
        try (ResultSet rows = s.executeQuery(query)) {
            while (rows.next()) {
                StringJoiner record = new StringJoiner("\t");
                for (int n=1;n<=columns;n++) record.add(rows.getString(n));
                missing.remove(record.toString());
            }
        }
        return missing.isEmpty();
    }
    private static boolean schema(Statement s, String service) throws Exception {
        return metadata(s, service+"-columns", "SELECT TABLE_NAME,COLUMN_NAME,LOWER(COLUMN_TYPE),IS_NULLABLE FROM information_schema.columns WHERE TABLE_SCHEMA=DATABASE()", 4)
                && metadata(s, service+"-indexes", "SELECT TABLE_NAME,INDEX_NAME,SEQ_IN_INDEX,COLUMN_NAME,NON_UNIQUE FROM information_schema.statistics WHERE TABLE_SCHEMA=DATABASE()", 5);
    }
    private static boolean verifyXxl(Statement s) throws Exception {
        return schema(s, "xxjob") && "1".equals(value(s,
                "SELECT (SELECT COUNT(*) FROM xxl_job_group WHERE id IN (1,2))=2 "
                + "AND (SELECT COUNT(*) FROM xxl_job_info WHERE (id=1 AND job_group=1) OR (id IN (2,3,4) AND job_group=2))=4 "
                + "AND EXISTS(SELECT 1 FROM xxl_job_user WHERE id=1 AND role=1) "
                + "AND EXISTS(SELECT 1 FROM xxl_job_lock WHERE lock_name='schedule_lock')"));
    }
    private static boolean verifyReplication(Statement s) throws SQLException {
        if (!"1".equals(value(s,"SELECT @@log_bin=1 AND @@binlog_format='ROW' AND @@server_id>0"))) return false;
        boolean slave=false,client=false;
        try (ResultSet rs=s.executeQuery("SHOW GRANTS FOR CURRENT_USER")) {
            while (rs.next()) {
                String grants=rs.getString(1).toUpperCase(Locale.ROOT);
                if (!grants.contains(" ON *.* TO ")) continue;
                slave |= grants.contains("REPLICATION SLAVE") || grants.contains("ALL PRIVILEGES");
                client |= grants.contains("REPLICATION CLIENT") || grants.contains("ALL PRIVILEGES");
            }
        }
        return client && slave && "1".equals(value(s,"SELECT EXISTS(SELECT 1 FROM information_schema.processlist WHERE USER=SUBSTRING_INDEX(CURRENT_USER(),'@',1) AND COMMAND LIKE 'Binlog Dump%')"));
    }
}
