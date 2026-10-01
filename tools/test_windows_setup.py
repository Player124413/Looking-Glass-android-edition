"""Exercise the guided installer's worker using local synthetic data and HTTP."""
from contextlib import contextmanager
import hashlib
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import threading
import time
import unittest
import uuid

from test_setup import ROOT, make_data

TEST_ROOT = ROOT / 'private' / ('guided-setup-tests-' + uuid.uuid4().hex)


@contextmanager
def server(payload, slow=False):
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_GET(self):
            self.send_response(200)
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            try:
                for i in range(0, len(payload), 1024):
                    self.wfile.write(payload[i:i + 1024])
                    self.wfile.flush()
                    if slow:
                        time.sleep(0.02)
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                pass
    srv = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=srv.serve_forever, daemon=True)
    thread.start()
    try:
        yield f'http://127.0.0.1:{srv.server_port}/fixture.7z'
    finally:
        srv.shutdown()
        srv.server_close()


@unittest.skipUnless(os.name == 'nt', 'Windows installer')
class GuidedSetupTests(unittest.TestCase):
    def setUp(self):
        self.root = TEST_ROOT / self._testMethodName / 'Jeu & preview [test]'
        (self.root / 'tools').mkdir(parents=True)
        shutil.copyfile(ROOT / 'Setup.cmd', self.root / 'Setup.cmd')
        self.script = (ROOT / 'tools/windows_setup.ps1').read_text(encoding='utf-8')
        self.data = self.root.parent / 'Données & source'
        make_data(self.data)
        self.job = self.root / 'private/job'
        self.job.mkdir(parents=True)
        self.config = self.root / 'private/data-path.txt'
        self.config.write_text('old-data-path\n')
        self.save = self.root / 'private/saves/keep.txt'
        self.save.parent.mkdir()
        self.save.write_text('keep my progress')

    def start(self, source='', download=False, script=None):
        # Only the test copy can use loopback HTTP and synthetic download hashes.
        path = self.root / 'tools/windows_setup.ps1'
        path.write_text(script or self.script, encoding='utf-8')
        jobfile = self.job / 'job.json'
        jobfile.write_text(json.dumps(dict(directory=str(self.job), source=str(source), download=download)))
        return subprocess.Popen(['powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', str(path),
                                 '-Mode', 'Worker', '-JobFile', str(jobfile)], cwd=self.root,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def finish(self, proc, expected):
        stdout, stderr = proc.communicate(timeout=30)
        self.assertEqual(proc.returncode, expected, (stdout + stderr).decode(errors='replace'))
        self.assertEqual(self.save.read_text(), 'keep my progress')

    def test_folder_worker_handles_literal_paths(self):
        self.finish(self.start(self.data), 0)
        self.assertEqual(Path(self.config.read_text(encoding='utf-8').strip()), self.data / 'Alice1/bin/base')

    def test_invalid_input_keeps_previous_config(self):
        self.finish(self.start(self.root / 'missing.7z'), 1)
        self.assertEqual(self.config.read_text(), 'old-data-path\n')

    def test_cancellation_before_work_keeps_previous_config(self):
        (self.job / 'cancel').write_text('cancel')
        self.finish(self.start(self.data), 1)
        self.assertEqual(self.config.read_text(), 'old-data-path\n')

    def download_script(self, url, payload, wrong_hash=False):
        s = self.script.replace('https://archive.org/download/alice_202106/Alice1_2011_vanilla.7z', url)
        s = s.replace('977811158L', str(len(payload)) + 'L')
        s = s.replace('b253bb2c9c875f1838a711d0fe4f12f4bc2c6207db3baaE6431af962eca8705d',
                      '0' * 64 if wrong_hash else hashlib.sha256(payload).hexdigest())
        return s.replace("$response.ResponseUri.Scheme -ne 'https'", "$response.ResponseUri.Scheme -ne 'http'")

    def test_failed_integrity_check_does_not_publish_download(self):
        payload = b'synthetic bad archive' * 1024
        with server(payload) as url:
            self.finish(self.start(download=True, script=self.download_script(url, payload, wrong_hash=True)), 1)
        state = json.loads((self.job / 'status.json').read_text())
        self.assertIn('integrity', state['message'])
        self.assertEqual(self.config.read_text(), 'old-data-path\n')
        self.assertFalse(list((self.root / 'private/downloads').iterdir()))

    def test_download_cancellation_keeps_previous_install(self):
        payload = b'x' * (1024 * 1024)
        with server(payload, slow=True) as url:
            proc = self.start(download=True, script=self.download_script(url, payload))
            deadline = time.monotonic() + 10
            while not list((self.root / 'private/downloads').glob('*.partial')):
                if proc.poll() is not None or time.monotonic() > deadline:
                    self.fail('Worker did not begin the synthetic download')
                time.sleep(0.05)
            (self.job / 'cancel').write_text('cancel')
            self.finish(proc, 1)
        self.assertEqual(self.config.read_text(), 'old-data-path\n')
        self.assertFalse(list((self.root / 'private/downloads').iterdir()))

    def test_download_and_import(self):
        extractor = shutil.which('7z') or str(Path(os.environ.get('ProgramFiles', 'C:/Program Files')) / '7-Zip/7z.exe')
        if not Path(extractor).is_file():
            self.skipTest('7-Zip needed to create the synthetic fixture')
        archive = self.root.parent / 'fixture.7z'
        subprocess.run([extractor, 'a', '-t7z', str(archive), 'Alice1'], cwd=self.data, capture_output=True, check=True)
        payload = archive.read_bytes()
        with server(payload) as url:
            self.finish(self.start(download=True, script=self.download_script(url, payload)), 0)
        self.assertTrue((self.root / self.config.read_text().strip() / 'pak0.pk3').is_file())
        self.assertEqual((self.root / 'private/downloads/Alice1_2011_vanilla.7z').read_bytes(), payload)


if __name__ == '__main__':
    unittest.main()
