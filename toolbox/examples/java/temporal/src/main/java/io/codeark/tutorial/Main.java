package io.codeark.tutorial;

import io.temporal.activity.ActivityInterface;
import io.temporal.activity.ActivityMethod;
import io.temporal.activity.ActivityOptions;
import io.temporal.client.*;
import io.temporal.serviceclient.*;
import io.temporal.worker.*;
import io.temporal.workflow.*;
import java.time.Duration;

public final class Main {
  @WorkflowInterface
  public interface Greeting {
    @WorkflowMethod
    String run();
  }

  @ActivityInterface
  public interface GreetingActivity {
    @ActivityMethod
    String hello();
  }

  public static class GreetingActivityImpl implements GreetingActivity {
    public String hello() {
      return "Hello, CodeArk";
    }
  }

  public static class GreetingImpl implements Greeting {
    public String run() {
      return Workflow.newActivityStub(
              GreetingActivity.class,
              ActivityOptions.newBuilder().setStartToCloseTimeout(Duration.ofSeconds(5)).build())
          .hello();
    }
  }

  public static void main(String[] args) {
    Scope.run(
        scope -> {
          var service =
              WorkflowServiceStubs.newServiceStubs(
                  WorkflowServiceStubsOptions.newBuilder()
                      .setTarget(
                          scope.host("CODEARK_TEMPORAL_HOST")
                              + ":"
                              + scope.port("CODEARK_TEMPORAL_PORT"))
                      .build());
          var client =
              WorkflowClient.newInstance(
                  service,
                  WorkflowClientOptions.newBuilder()
                      .setNamespace(scope.required("CODEARK_TEMPORAL_NAMESPACE"))
                      .build());
          var factory = WorkerFactory.newInstance(client);
          String queue = scope.name();
          var worker = factory.newWorker(queue);
          worker.registerWorkflowImplementationTypes(GreetingImpl.class);
          worker.registerActivitiesImplementations(new GreetingActivityImpl());
          try {
            factory.start();
            var workflow =
                client.newWorkflowStub(
                    Greeting.class,
                    WorkflowOptions.newBuilder()
                        .setWorkflowId(scope.name())
                        .setTaskQueue(queue)
                        .setWorkflowRunTimeout(Duration.ofSeconds(30))
                        .build());
            if (!"Hello, CodeArk".equals(workflow.run()))
              throw new IllegalStateException("Workflow output mismatch");
            System.out.println("TEMPORAL_WORKER_OK");
          } finally {
            factory.shutdown();
            service.shutdownNow();
          }
        });
  }
}
