"""Unit tests for safe Hermes tool limit handling (stdlib only)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest import mock

PLUGIN_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PLUGIN_ROOT))

import suflyor  # noqa: E402


LIMIT_CASES = (
    ("missing", None, 10),
    ("zero", 0, 1),
    ("negative", -7, 1),
    ("too large", 51, 50),
    ("nonnumeric", "many", 10),
    ("boolean false", False, 10),
    ("boolean true", True, 10),
    ("in range", 24, 24),
    ("numeric string", "12", 12),
)


class NormalizeLimitTests(unittest.TestCase):
    def test_limit_is_normalized_to_safe_range(self) -> None:
        for label, value, expected in LIMIT_CASES:
            with self.subTest(label=label):
                self.assertEqual(suflyor._normalize_limit(value), expected)


class HandlerLimitTests(unittest.TestCase):
    def test_recent_sessions_sends_normalized_limit(self) -> None:
        for label, value, expected in LIMIT_CASES:
            args = {} if label == "missing" else {"limit": value}
            with self.subTest(label=label), mock.patch.object(
                suflyor, "_request", return_value={"sessions": []}
            ) as request:
                suflyor._h_recent(args)
                request.assert_called_once_with("GET", "/sessions", params={"limit": expected})

    def test_search_sends_normalized_limit(self) -> None:
        for label, value, expected in LIMIT_CASES:
            args = {"query": "needle"}
            if label != "missing":
                args["limit"] = value
            with self.subTest(label=label), mock.patch.object(
                suflyor, "_request", return_value={"hits": []}
            ) as request:
                suflyor._h_search(args)
                request.assert_called_once_with(
                    "GET", "/search", params={"q": "needle", "limit": expected}
                )


if __name__ == "__main__":
    unittest.main()
