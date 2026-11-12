package io.codeark.tutorial;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Map;
import org.junit.jupiter.api.Test;

class ScopeTest {
  @Test
  void rejectsRemoteTargetBeforeAClientCanConnect() {
    var scope = new Scope(Map.of("HOST", "example.com"));
    assertThrows(IllegalArgumentException.class, () -> scope.host("HOST"));
  }

  @Test
  void refusesBusinessDatabaseDespiteTeachingConsent() {
    var scope = new Scope(Map.of("CODEARK_RUN_ID", "test1234", "CODEARK_SQL_DATABASE", "business"));
    assertThrows(IllegalArgumentException.class, scope::database);
  }

  @Test
  void consentIsExplicit() {
    assertThrows(
        IllegalArgumentException.class,
        () ->
            new Scope(Map.of("CODEARK_TUTORIAL_CONFIRM", "NO", "CODEARK_RUN_ID", "test1234"))
                .confirm());
  }

  @Test
  void allowsOnlyTheCurrentDedicatedDatabase() {
    var scope =
        new Scope(
            Map.of(
                "CODEARK_RUN_ID", "test1234", "CODEARK_SQL_DATABASE", "codeark_tutorial_test1234"));
    assertEquals("codeark_tutorial_test1234", scope.database());
  }
}
