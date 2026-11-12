import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
spec=importlib.util.spec_from_file_location("gpu",Path(__file__).parents[1]/"src/ai_gpu_install.py")
gpu=importlib.util.module_from_spec(spec);spec.loader.exec_module(gpu)
class RepairTests(unittest.TestCase):
 def setUp(self):
  self.temporary=tempfile.TemporaryDirectory(dir=Path(__file__).parents[1]/"target")
  self.addCleanup(self.temporary.cleanup)
  self.root=Path(self.temporary.name)/"ai-gpu-runtime";self.root.mkdir()
  self.dll=self.root/"original.dll";self.dll.write_bytes(b"original")
 def test_valid_runtime_skips_install_and_occupancy_checks(self):
  with patch.object(gpu,"validate",return_value=True),patch.object(gpu,"ensure_unoccupied") as occupied,patch.object(gpu.subprocess,"run") as install:
   gpu.repair(self.root)
  occupied.assert_not_called();install.assert_not_called()
  self.assertEqual(self.dll.read_bytes(),b"original")
  self.assertTrue((self.root/"ready").is_file())
 def test_busy_runtime_is_not_overwritten_or_downloaded(self):
  with patch.object(gpu,"validate",return_value=False),patch.object(gpu,"ensure_unoccupied",side_effect=RuntimeError(gpu.BUSY)),patch.object(gpu.subprocess,"run") as install:
   with self.assertRaisesRegex(RuntimeError,gpu.BUSY):gpu.repair(self.root)
  install.assert_not_called();self.assertEqual(self.dll.read_bytes(),b"original")
 def test_failed_staging_keeps_original_files(self):
  with patch.object(gpu,"validate",return_value=False),patch.object(gpu,"ensure_unoccupied"),patch.object(gpu.subprocess,"run"):
   with self.assertRaisesRegex(RuntimeError,"staging validation failed"):gpu.repair(self.root)
  self.assertEqual(self.dll.read_bytes(),b"original")
