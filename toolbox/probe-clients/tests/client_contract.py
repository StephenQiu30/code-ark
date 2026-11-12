"""Exercise each packaged SDK and an authenticated S3 exchange against local fixtures."""
import http.server
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[1]
SECRET = "secret-MUST-STAY-PRIVATE"


def invoke(module, **values):
    body = dict(probeId="probe", componentId="component", port=1, password=SECRET, username="test", database="test", purpose="seata")
    body.update(values)
    path = ROOT / "modules" / module / "target"
    classpath = os.pathsep.join(str(p) for p in [path/"probe.jar", path/"lib/*"])
    with tempfile.TemporaryDirectory() as private:
        result = subprocess.run(
            ["java", "-Xmx128m", "-XX:ActiveProcessorCount=2", f"-Duser.home={private}", "-cp", classpath, "io.codeark.ProbeMain"],
            input=json.dumps(body).encode(), cwd=private, capture_output=True, timeout=25
        )
    if SECRET.encode() in result.stdout + result.stderr:
        raise AssertionError("Private input leaked to child process output")
    if result.returncode:
        raise AssertionError(f"Client process failed with status {result.returncode}")
    return json.loads(result.stdout)


class ClientContract(unittest.TestCase):
    def test_each_client_rejects_refused_connection_without_leaking_input(self):
        for module in sorted((ROOT / "modules").iterdir()):
            with self.subTest(module=module.name):
                data = invoke(module.name)
                self.assertEqual((data["probeId"],data["componentId"],data["success"],data["errorCode"]),("probe","component",False,"ProtocolUnavailable"))
                self.assertGreater(data["sampledAt"],0)

    def test_s3_signed_list_buckets_and_denied_credentials(self):
        signatures = []
        class S3(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                auth = self.headers.get("Authorization","")
                signatures.append(auth.startswith("AWS4-HMAC-SHA256") and "Credential=test/" in auth)
                if not signatures[-1]:
                    self.send_error(403)
                    return
                body = b'<ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Owner><ID>test</ID><DisplayName>test</DisplayName></Owner><Buckets/></ListAllMyBucketsResult>'
                self.send_response(200)
                self.send_header("Content-Type","application/xml")
                self.send_header("Content-Length",str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            def log_message(self,*args):
                pass
        with http.server.HTTPServer(("127.0.0.1",0), S3) as api:
            worker = threading.Thread(target=api.serve_forever,daemon=True)
            worker.start()
            try:
                self.assertTrue(invoke("s3",port=api.server_port)["success"])
                self.assertFalse(invoke("s3",port=api.server_port,username="wrong-user")["success"])
                self.assertIn(True, signatures)
                self.assertIn(False, signatures)
            finally:
                api.shutdown()
                worker.join()

if __name__=="__main__":
    unittest.main()
