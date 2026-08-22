## 2026-06-11 - Windows Path Normalization Bypass in Archive Extraction Safety Checks

**Vulnerability:** Zip Slip / Path Traversal validation (`archive_entry_is_safe`) checked path components using `c.trim_end_matches(' ') == ".."`. However, Windows Win32 API / `bsdtar` normalizes path components by stripping trailing spaces AND trailing dots (`.`), causing path components like `.. .` or `...` to bypass the `trim_end_matches(' ')` check while resolving to parent directory traversal (`..`) on extraction.

**Learning:** Trimming only spaces when validating relative path components on Windows is insufficient because Windows file APIs strip both trailing dots and trailing spaces from path components during path resolution.

**Prevention:** When checking path safety for Windows extraction tools, check whether components consist solely of dots and spaces (other than a single `.`) or if `c.trim_end_matches([' ', '.']) == ".."`.
