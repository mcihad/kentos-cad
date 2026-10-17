"""The SDK and the host agreeing on command versions (docs/adr/0443 §3)."""

import unittest

from kentos import cad
from kentos.cad import NotRunHere, compatibility, require


class Versions(unittest.TestCase):
    def test_the_sdk_and_its_native_host_agree(self) -> None:
        c = compatibility()
        self.assertTrue(c.ok, c)
        self.assertEqual(c.extra, [])

    def test_a_host_of_another_version_is_told_before_any_call(self) -> None:
        host = [(w.id, w.version) for w in cad.COMMANDS.values() if isinstance(w, cad.LocalCommand)]
        host = [(i, v + 1) if i == "cad.forms.submit" else (i, v) for i, v in host if i != "cad.schema.describe"]
        host.append(("cad.gelecek.komut", 1))
        c = compatibility(host)
        self.assertFalse(c.ok)
        self.assertEqual(c.missing, ["cad.schema.describe"])
        self.assertEqual(c.newer, {"cad.forms.submit": (1, 2)})
        self.assertEqual(c.extra, ["cad.gelecek.komut"])
        with self.assertRaises(NotRunHere) as e:
            require("cad.forms.submit", "cad.schema.describe", host=host)
        self.assertEqual(e.exception.code, "unknown_version")
        self.assertIn("cad.forms.submit (SDK v1, KentOS v2)", e.exception.message)
        self.assertIn("cad.schema.describe (bu sürüm çalıştırmıyor)", e.exception.message)
        require("cad.forms.validate", host=host)
        with self.assertRaises(NotRunHere):
            require("cad.yok")

    def test_the_form_profile_is_pinned(self) -> None:
        self.assertEqual(cad.FORM_PROFILE, "kentos.forms/1")


if __name__ == "__main__":
    unittest.main()
