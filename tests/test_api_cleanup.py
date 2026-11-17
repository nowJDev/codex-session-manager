# 격리 API 검증 루트의 Windows 읽기 전용 파일 정리와 소유 경계를 검증한다.
import errno
import importlib.util
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location(
    "csm_api_check", Path(__file__).resolve().parents[1] / "scripts/check-codex-api.py"
)
API_CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(API_CHECK)


class ApiCleanupTests(unittest.TestCase):
    def test_removes_owned_windows_read_only_pack_file(self):
        with tempfile.TemporaryDirectory(prefix="csm-api-") as directory:
            root = Path(directory)
            pack = root / ".codex/.tmp/plugins-clone/.git/objects/pack/fixture.idx"
            pack.parent.mkdir(parents=True)
            pack.write_bytes(b"generated pack index")
            pack.chmod(stat.S_IREAD)
            self.assertFalse(pack.stat().st_mode & stat.S_IWRITE)
            try:
                API_CHECK.clean(root)
            except PermissionError as error:
                self.fail(f"Generated read-only file must be cleaned: {error}")
            self.assertFalse(root.exists())

    def test_refuses_external_symlink_or_direct_external_path(self):
        with tempfile.TemporaryDirectory(prefix="csm-api-") as directory:
            with tempfile.TemporaryDirectory(prefix="csm-outside-") as outside_directory:
                root = Path(directory)
                outside = Path(outside_directory) / "preserve.txt"
                outside.write_bytes(b"outside fixture")
                original_mode = outside.stat().st_mode
                link = root / "external-link"
                try:
                    link.symlink_to(outside)
                except OSError as error:
                    if os.name != "nt" or error.winerror not in (5, 1314):
                        raise
                    with self.assertRaises(ValueError):
                        API_CHECK.owned_path(root, outside)
                else:
                    with self.assertRaises(ValueError):
                        API_CHECK.clean(root)
                    self.assertTrue(link.is_symlink())
                self.assertEqual(outside.read_bytes(), b"outside fixture")
                self.assertEqual(outside.stat().st_mode, original_mode)

    def test_propagates_original_permission_error_for_writable_owned_file(self):
        with tempfile.TemporaryDirectory(prefix="csm-api-") as directory:
            root = Path(directory)
            file = root / "protected-by-acl.txt"
            file.write_bytes(b"preserve on ACL denial")
            original_mode = file.stat().st_mode
            denied = PermissionError(errno.EACCES, "simulated ACL denial", str(file))
            # 실제 ACL 변경은 수행하지 않고 OS unlink 경계의 거부만 재현한다.
            with mock.patch.object(Path, "unlink", side_effect=denied):
                with self.assertRaises(PermissionError) as raised:
                    API_CHECK.clean(root)
            self.assertIs(raised.exception, denied)
            self.assertEqual(file.read_bytes(), b"preserve on ACL denial")
            self.assertEqual(file.stat().st_mode, original_mode)


if __name__ == "__main__":
    unittest.main()
