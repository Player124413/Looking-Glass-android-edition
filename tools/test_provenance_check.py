"""Keep the identifier exception narrower than ordinary quoted prose."""
import unittest
from provenance_check import without_asset_identifiers


class AssetIdentifiers(unittest.TestCase):
    def test_only_an_exact_mounted_path_is_exempt(self):
        paths = {"sound/demo/example.wav"}
        self.assertEqual(
            without_asset_identifiers('cue("sound/demo/example.wav") trailing words', paths),
            'cue("<asset-identifier>") trailing words',
        )
        for line in (
            'cue("sound/demo/missing.wav")',
            '"A sentence mentions sound/demo/example.wav here"',
            '"An ordinary sentence with several matching tokens"',
        ):
            self.assertEqual(without_asset_identifiers(line, paths), line)


if __name__ == "__main__":
    unittest.main()
