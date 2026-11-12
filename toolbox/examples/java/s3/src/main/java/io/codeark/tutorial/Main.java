package io.codeark.tutorial;

import io.minio.*;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;

public final class Main {
  public static void main(String[] args) {
    Scope.run(
        scope -> {
          String bucket = "codeark-tutorial-" + scope.runId();
          String object = "tutorial.txt";
          var client =
              MinioClient.builder()
                  .endpoint(scope.http("CODEARK_S3_ENDPOINT").toString())
                  .credentials(
                      scope.required("CODEARK_S3_ACCESS_KEY"),
                      scope.required("CODEARK_S3_SECRET_KEY"))
                  .build();
          if (client.bucketExists(BucketExistsArgs.builder().bucket(bucket).build()))
            throw new IllegalStateException("Teaching bucket already exists; use a new run ID");
          client.makeBucket(MakeBucketArgs.builder().bucket(bucket).build());
          boolean written = false;
          try {
            byte[] data = "codeark tutorial".getBytes(StandardCharsets.UTF_8);
            client.putObject(
                PutObjectArgs.builder().bucket(bucket).object(object).stream(
                        new ByteArrayInputStream(data), (long) data.length, -1L)
                    .build());
            written = true;
            try (var result =
                client.getObject(GetObjectArgs.builder().bucket(bucket).object(object).build())) {
              if (!"codeark tutorial"
                  .equals(new String(result.readAllBytes(), StandardCharsets.UTF_8)))
                throw new IllegalStateException("Object mismatch");
            }
            System.out.println("S3_WRITE_READ_OK");
          } finally {
            if (written)
              client.removeObject(RemoveObjectArgs.builder().bucket(bucket).object(object).build());
            client.removeBucket(RemoveBucketArgs.builder().bucket(bucket).build());
          }
        });
  }
}
