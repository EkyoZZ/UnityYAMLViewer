"""Small deterministic downloads; no network or real model files."""
import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
import urllib.error
from unittest.mock import patch

spec=importlib.util.spec_from_file_location("download",Path(__file__).parents[1]/"src/translation_download.py")
download=importlib.util.module_from_spec(spec)
spec.loader.exec_module(download)

class Response(io.BytesIO):
    def __init__(self,content,status=200,headers=None):
        super().__init__(content)
        self.status=status
        self.headers=headers or {}

class DownloadTests(unittest.TestCase):
    def setUp(self):
        boundary=(Path(__file__).parents[1]/"target").resolve()
        self.temporary=tempfile.TemporaryDirectory(dir=boundary,prefix="download-test-")
        self.root=Path(self.temporary.name).resolve()
        assert self.root.is_relative_to(boundary)
        self.addCleanup(self.temporary.cleanup)
        self.output=contextlib.redirect_stdout(io.StringIO())
        self.output.__enter__()
        self.addCleanup(lambda:self.output.__exit__(None,None,None))
        self.data=b"verified fake archive"
        self.digest=hashlib.sha256(self.data).hexdigest()
    def download(self):
        return download.verified_archive(self.root,"test","https://example.invalid/model.zip",len(self.data),self.digest)
    def partial(self,data):
        directory=self.root/"downloads"
        directory.mkdir()
        (directory/"test.zip.part").write_bytes(data)
    def test_complete_archive_is_reused_offline(self):
        self.partial(self.data)
        with patch.object(download.urllib.request,"urlopen") as request:
            path=self.download()
            self.assertEqual(path.read_bytes(),self.data)
            self.download()
            request.assert_not_called()
    def test_resume_checks_range_and_hash(self):
        self.partial(self.data[:5])
        response=Response(self.data[5:],206,{"Content-Range":f"bytes 5-{len(self.data)-1}/{len(self.data)}"})
        with patch.object(download.urllib.request,"urlopen",return_value=response) as request:
            self.assertEqual(self.download().read_bytes(),self.data)
            self.assertEqual(request.call_args.args[0].get_header("Range"),"bytes=5-")
    def test_ignored_range_restarts_safely(self):
        self.partial(b"wrong prefix")
        with patch.object(download.urllib.request,"urlopen",return_value=Response(self.data)):
            self.assertEqual(self.download().read_bytes(),self.data)
    def test_timeout_retries_and_preserves_partial(self):
        self.partial(self.data[:5])
        with patch.object(download.urllib.request,"urlopen",side_effect=urllib.error.URLError("WinError 10060")) as request,patch.object(download.time,"sleep"):
            with self.assertRaisesRegex(RuntimeError,"Retry installation"):
                self.download()
            self.assertEqual(request.call_count,3)
        self.assertEqual((self.root/"downloads/test.zip.part").read_bytes(),self.data[:5])
    def test_bad_resume_response_is_rejected(self):
        self.partial(self.data[:5])
        with patch.object(download.urllib.request,"urlopen",return_value=Response(self.data,206,{"Content-Range":"bytes 0-19/20"})):
            with self.assertRaisesRegex(ValueError,"resume response"):
                self.download()
    def test_corrupt_archive_cannot_be_published(self):
        with patch.object(download.urllib.request,"urlopen",return_value=Response(b"x"*len(self.data))):
            with self.assertRaisesRegex(ValueError,"SHA-256"):
                self.download()
        self.assertFalse((self.root/"downloads/test.zip").exists())

if __name__=="__main__":unittest.main()
