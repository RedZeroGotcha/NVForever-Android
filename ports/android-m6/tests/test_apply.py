import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

SPEC = importlib.util.spec_from_file_location("m6_overlay", Path(__file__).parents[1] / "apply.py")
overlay = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(overlay)


def zip_bytes(name, data=b"example"):
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w") as archive:
        archive.writestr(name, data)
    return output.getvalue()


class ApplyTests(unittest.TestCase):
    def test_rejects_archive_hash_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "archive.zip"
            path.write_bytes(b"incorrect archive")
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                overlay.archive_bytes(path, "unused", "0" * 64)
            self.assertEqual(overlay.archive_bytes(path, "unused", hashlib.sha256(path.read_bytes()).hexdigest()), path.read_bytes())

    def test_rejects_unsupported_or_modified_base(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "Missing v0.5.1"):
                overlay.validate_base(root)
            file = root / "engine/Cargo.toml"
            file.parent.mkdir()
            file.write_text("modified")
            with self.assertRaisesRegex(ValueError, "differs"):
                overlay.validate_base(root)
            self.assertFalse((root / "android").exists())

    def test_result_check_rejects_silently_skipped_patch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            payload = b"patched native input"
            (root / "result-files.json").write_text(json.dumps({"input.rs": hashlib.sha256(payload).hexdigest()}))
            with patch.object(overlay, "HERE", root):
                with self.assertRaisesRegex(ValueError, "Reconstructed source differs"):
                    overlay.validate_result(root)
                (root / "input.rs").write_bytes(payload)
                overlay.validate_result(root)
                (root / "input.rs").write_bytes(b"unpatched native input")
                with self.assertRaisesRegex(ValueError, "Reconstructed source differs"):
                    overlay.validate_result(root)

    def test_rejects_escaping_paths_and_existing_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "tree"
            root.mkdir()
            (root / "link").symlink_to(root.parent, target_is_directory=True)
            for name in ["../outside", "/absolute", "C:/drive", "a\\b", "link/outside"]:
                with self.subTest(name=name), self.assertRaises(ValueError):
                    overlay.member_path(root, name)

    def test_validates_all_zip_members_before_writing(self):
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            archive.writestr("first.txt", b"must not be written")
            archive.writestr("../outside.txt", b"escape")
        with tempfile.TemporaryDirectory() as directory, patch.object(overlay, "validate_base"):
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "Unsafe archive"):
                overlay.apply(root, output.getvalue(), b"unused")
            self.assertEqual(list(root.iterdir()), [])

    def test_rejects_zip_symlinks_before_writing(self):
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            info = zipfile.ZipInfo("link")
            info.create_system = 3
            info.external_attr = 0o120777 << 16
            archive.writestr(info, "../outside")
        with tempfile.TemporaryDirectory() as directory, patch.object(overlay, "validate_base"):
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "symlink"):
                overlay.apply(root, output.getvalue(), b"unused")
            self.assertEqual(list(root.iterdir()), [])

    def test_rejects_tar_links_before_extracting_either_archive(self):
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode="w:gz") as archive:
            info = tarfile.TarInfo("winit-0.30.13/link")
            info.type = tarfile.SYMTYPE
            info.linkname = "../../outside"
            archive.addfile(info)
        with tempfile.TemporaryDirectory() as directory, patch.object(overlay, "validate_base"):
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "Unexpected winit"):
                overlay.apply(root, zip_bytes("first.txt"), output.getvalue())
            self.assertEqual(list(root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
