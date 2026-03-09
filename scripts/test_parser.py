"""
DEPRECATED: test_parser.py used thermognosis.dataset.json_parser which has been
removed in the Rust-first migration (SPEC-IO-WALKER-01).

The equivalent Rust function is rust_core.py_validate_single_file(path, domain).
Example replacement:
    import rust_core
    result = rust_core.py_validate_single_file("dataset/raw/00000001.json", "samples")
    print(result)
"""
import sys
print(
    "DEPRECATED: This script requires thermognosis.dataset.json_parser which has been "
    "removed. Use rust_core.py_validate_single_file() instead.",
    file=sys.stderr,
)
sys.exit(1)
