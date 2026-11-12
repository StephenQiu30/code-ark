package io.codeark.tutorial;
import com.typesafe.config.ConfigFactory;
import org.junit.jupiter.api.Test;
import java.nio.file.Files;
import java.util.Map;
import static org.junit.jupiter.api.Assertions.*;
class ConfigurationTest {
 @Test void generatedConfigurationUsesActualCoordinatorAndParsesBeforeConnecting()throws Exception {
  var directory=Main.createConfiguration(new Scope(Map.of("CODEARK_SEATA_HOST","127.0.0.1","CODEARK_SEATA_PORT","18091")));
  try {
   var registry=ConfigFactory.parseFile(directory.resolve("registry.conf").toFile());
   var client=ConfigFactory.parseFile(directory.resolve("file.conf").toFile());
   assertEquals("file",registry.getString("registry.type"));
   assertEquals("127.0.0.1:18091",client.getString("service.default.grouplist"));
   assertEquals("default",client.getString("service.vgroupMapping.codeark_tutorial"));
  } finally {Files.deleteIfExists(directory.resolve("registry.conf"));Files.deleteIfExists(directory.resolve("file.conf"));Files.deleteIfExists(directory);}
 }
}
