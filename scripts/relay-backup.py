#!/usr/bin/env python3
"""Consistent online SQLite + verified original-audio backup; no secrets printed."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import sys
from datetime import datetime, timezone


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def connection(path, mode):
    from urllib.parse import quote
    return sqlite3.connect(f"file:{quote(str(path.resolve()))}?mode={mode}", uri=True)


def verify(path):
    with connection(path, 'ro') as db:
        if db.execute('PRAGMA integrity_check').fetchone()[0] != 'ok':
            raise ValueError('SQLite integrity check failed')
        if db.execute('PRAGMA user_version').fetchone()[0] != 1:
            raise ValueError('Unsupported physical database schema')
        relay = db.execute("SELECT json FROM metadata WHERE key='relay'").fetchone()
        count, cursor = db.execute('SELECT COUNT(*), COALESCE(MAX(seq),0) FROM events').fetchone()
        return {'schema': 1, 'event_count': count, 'cursor': cursor,
                'relay': json.loads(relay[0]) if relay else None}


def required_assets(path):
    result = {}
    with connection(path, 'ro') as db:
        for (text,) in db.execute("SELECT json FROM metadata WHERE key >= 'asset.record.' AND key < 'asset.record/'"):
            record = json.loads(text)
            key, size = record['content_hash'], record['byte_length']
            if not isinstance(key, str) or not re.fullmatch('[0-9a-f]{64}', key) or not isinstance(size, int) or not 0 < size <= 32 * 1024 * 1024:
                raise ValueError('Invalid audio registry in database snapshot')
            if key in result and result[key] != size:
                raise ValueError('Conflicting original audio byte lengths in snapshot')
            result[key] = size
    return dict(sorted(result.items()))


def audio_folder(path):
    return path.with_suffix(path.suffix + '.assets')


def verify_assets(folder, required):
    for key, size in required.items():
        path = folder / f'{key}.blob'
        if path.is_symlink() or not path.is_file() or path.stat().st_size != size or digest(path) != key:
            raise ValueError('An original audio asset is missing or failed its SHA-256/length check')


def copy_assets(source, destination, required):
    if not required:
        return
    if destination.exists():
        raise ValueError('Destination audio directory already exists; refusing to overwrite')
    verify_assets(source, required)
    destination.mkdir(mode=0o700, parents=True)
    for key in required:
        with (source / f'{key}.blob').open('rb') as original:
            descriptor = os.open(destination / f'{key}.blob', os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(descriptor, 'wb') as copied:
                while chunk := original.read(256 * 1024):
                    copied.write(chunk)
                copied.flush()
                os.fsync(copied.fileno())
    verify_assets(destination, required)


def verify_manifest(source, assets):
    path = source.with_suffix(source.suffix + '.json')
    if not path.exists():
        raise ValueError('Restore/verified backup requires the matching .json manifest')
    manifest = json.loads(path.read_text())
    if manifest.get('format') != 'rufren-relay-sqlite' or manifest.get('version') not in (1, 2):
        raise ValueError('Unsupported backup manifest format')
    if manifest['sha256'] != digest(source):
        raise ValueError('Backup SHA-256 does not match its manifest')
    required = required_assets(source)
    listed = manifest.get('assets', {})
    if listed != required:
        raise ValueError('Backup manifest audio list does not match the SQLite snapshot')
    verify_assets(assets, required)
    return required


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['backup', 'verify', 'restore'])
    parser.add_argument('source', type=Path)
    parser.add_argument('destination', type=Path, nargs='?')
    parser.add_argument('--assets', type=Path, help='Source original-audio directory; defaults to SOURCE.sqlite.assets')
    args = parser.parse_args()
    source = args.source
    source_assets = args.assets or audio_folder(source)
    if not source.is_file():
        parser.error('source database does not exist')
    if args.command == 'verify':
        required = verify_manifest(source, source_assets)
        details = verify(source)
        details['audio_blobs'] = len(required)
        print(json.dumps(details, sort_keys=True))
        return
    if args.destination is None:
        parser.error('backup and restore require a new destination path')
    destination = args.destination
    if destination.exists() or destination.with_suffix(destination.suffix + '.json').exists() or audio_folder(destination).exists():
        parser.error('destination, manifest, or audio directory already exists; refusing to overwrite')
    if args.command == 'restore':
        verify_manifest(source, source_assets)
    destination.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(destination, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    os.close(descriptor)
    try:
        with connection(source, 'ro') as original, sqlite3.connect(destination) as copied:
            original.backup(copied)
        details = verify(destination)
        required = required_assets(destination)
        # Snapshot first, immutable bytes second: every manifest link refers to an already committed blob.
        copy_assets(source_assets, audio_folder(destination), required)
        details.update(format='rufren-relay-sqlite', version=2, assets=required,
                       created_at=datetime.now(timezone.utc).isoformat(), sha256=digest(destination))
        metadata = destination.with_suffix(destination.suffix + '.json')
        with metadata.open('x') as output:
            os.chmod(metadata, 0o600)
            json.dump(details, output, indent=2)
            output.write('\n')
            output.flush()
            os.fsync(output.fileno())
        print(f'{args.command.capitalize()} verified: {details["event_count"]} events, cursor {details["cursor"]}, {len(required)} audio blobs')
    except Exception:
        print('Incomplete destination retained; inspect it before retrying with a new path.', file=sys.stderr)
        raise


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, sqlite3.Error, KeyError, TypeError) as error:
        print(f'Backup failed: {error}', file=sys.stderr)
        sys.exit(1)
