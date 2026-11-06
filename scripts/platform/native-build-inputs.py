#!/usr/bin/env python3
"""Collect native source inputs, including literal Rust includes outside crates/.

Generated OUT_DIR assets are not literal paths; the caller separately binds
build-environment and CA identity. No build or process action is performed.
"""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
LITERAL_INCLUDE = re.compile(r'include_(?:bytes|str)!\s*\(\s*("(?:[^"\\]|\\.)*")\s*[,)]')


def collect(root=ROOT):
    root = Path(root).resolve()
    paths = {root / 'Cargo.toml', root / 'Cargo.lock'}
    paths.update(p for p in (root / '.cargo').rglob('*') if p.is_file())
    crates = {p for p in (root / 'crates').rglob('*')
              if p.is_file() and 'target' not in p.relative_to(root / 'crates').parts}
    paths.update(crates)
    literal_includes = []
    for source in sorted(p for p in crates if p.suffix == '.rs'):
        text = source.read_text()
        for match in LITERAL_INCLUDE.finditer(text):
            relative = json.loads(match.group(1))
            included = (source.parent / relative).resolve(strict=True)
            if not included.is_relative_to(root) or not included.is_file():
                raise ValueError('Literal Rust include must resolve to a repository file: ' + str(source))
            paths.add(included)
            literal_includes.append({'source': str(source.relative_to(root)),
                'line': text.count('\n', 0, match.start()) + 1,
                'file': str(included.relative_to(root))})
    for name in ['build-native.sh', 'finalize-apple-bundle.py', 'sign-macos-local.sh',
                 'check-apple-bundle.py', 'native-build-inputs.py']:
        paths.add(root / 'scripts/platform' / name)
    return {'source_sha256': {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in sorted(paths)}, 'literal_rust_includes': literal_includes}


if __name__ == '__main__':
    print(json.dumps(collect(), indent=2))
