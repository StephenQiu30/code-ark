package io.codeark.tutorial;

import com.xxl.job.core.context.XxlJobHelper;
import com.xxl.job.core.executor.impl.XxlJobSimpleExecutor;
import com.xxl.job.core.handler.annotation.XxlJob;
import java.nio.file.Files;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

public final class Main {
  private static final CountDownLatch executed = new CountDownLatch(1);

  public static final class Jobs {
    @XxlJob("codearkTutorial")
    public void tutorial() {
      XxlJobHelper.handleSuccess("CODEARK_XXL_TASK_OK");
      System.out.println("CODEARK_XXL_TASK_OK");
      executed.countDown();
    }
  }

  public static void main(String[] args) {
    Scope.run(
        scope -> {
          if (!"container".equals(System.getenv("CODEARK_ENDPOINT_MODE")))
            throw new IllegalArgumentException(
                "Executor must join actual admin Docker network; host loopback cannot be reached"
                    + " from the admin container");
          var executor = new XxlJobSimpleExecutor();
          executor.setAdminAddresses(scope.http("CODEARK_XXL_ADMIN_ENDPOINT").toString());
          executor.setAppname(scope.name());
          executor.setAccessToken(scope.required("CODEARK_XXL_EXECUTOR_TOKEN"));
          executor.setIp(scope.host("CODEARK_XXL_EXECUTOR_HOST"));
          executor.setPort(scope.port("CODEARK_XXL_EXECUTOR_PORT"));
          executor.setGlueEnabled(false);
          var logs = Files.createTempDirectory("codeark-xxl-tutorial-");
          executor.setLogPath(logs.toString());
          executor.setLogRetentionDays(1);
          executor.setXxlJobBeanList(List.of(new Jobs()));
          try {
            executor.start();
            System.out.println("XXL_EXECUTOR_WAITING_FOR_EXPLICIT_ADMIN_TRIGGER");
            if (!executed.await(120, TimeUnit.SECONDS))
              throw new IllegalStateException(
                  "Admin did not execute the tutorial job within120seconds");
          } finally {
            executor.destroy();
            System.out.println("XXL_TEACHING_LOGS_RETAINED_LOCALLY");
          }
        });
  }
}
