#!/usr/bin/env python3
"""Consistent online SQLite + verified original-audio backup; no secrets printed."""
import argparse
from contextlib import closing, contextmanager
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


@contextmanager
def connection(path, mode):
    from urllib.parse import quote
    database = sqlite3.connect(f"file:{quote(str(path.resolve()))}?mode={mode}", uri=True)
    try:
        yield database
    finally:
        database.close()


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
    result, records, references = {}, {}, []
    with connection(path, 'ro') as db:
        for row_key, text in db.execute("SELECT key,json FROM metadata WHERE key >= 'asset.record.' AND key < 'asset.record/'"):
            record = json.loads(text)
            key, size, asset_id = record['content_hash'], record['byte_length'], record['asset_id']
            if (not isinstance(key, str) or not re.fullmatch('[0-9a-f]{64}', key)
                    or type(size) is not int or not 0 < size <= 32 * 1024 * 1024
                    or not isinstance(asset_id, str) or row_key != 'asset.record.' + asset_id):
                raise ValueError('Invalid audio registry in database snapshot')
            if key in result and result[key] != size:
                raise ValueError('Conflicting original audio byte lengths in snapshot')
            result[key], records[asset_id] = size, key
        for (text,) in db.execute('SELECT json FROM events'):
            fact = json.loads(text)['fact']
            if fact['kind'] == 'recording_attached':
                references.append((fact['data']['asset_id'], fact['data']['asset']['sha256']))
            elif fact['kind'] == 'speech_transcribed':
                references.append((fact['data']['asset_id'], fact['data']['audio_sha256']))
    pending = {asset_id for asset_id, _ in references if asset_id not in records}
    if pending:
        # Facts may intentionally sync before (or without) recordings. This is an
        # incomplete audio replica, not proof that the immutable facts are corrupt.
        raise ValueError(f'Audio backup incomplete: {len(pending)} referenced recording(s) have not completed recording sync. Sync originals from their originating devices, then retry into a new destination; saved facts were kept')
    if any(records[asset_id] != key for asset_id, key in references):
        raise ValueError('Audio registry hash contradicts a recording/transcription reference in the snapshot')
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
    if manifest.get('format') != 'rufren-relay-sqlite' or type(manifest.get('version')) is not int or manifest.get('version') not in (1, 2):
        raise ValueError('Unsupported backup manifest format')
    if manifest['sha256'] != digest(source):
        raise ValueError('Backup SHA-256 does not match its manifest')
    actual = verify(source)
    for key in ['schema', 'event_count', 'cursor', 'relay']:
        if key not in manifest or type(manifest[key]) is not type(actual[key]) or manifest[key] != actual[key]:
            raise ValueError('Backup manifest metadata does not match its SQLite snapshot')
    required = required_assets(source)
    listed = manifest.get('assets', {})
    if (not isinstance(listed, dict) or any(type(size) is not int for size in listed.values()) or listed != required):
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
        with connection(source, 'ro') as original, closing(sqlite3.connect(destination)) as copied:
            # Anchor one read transaction: facts and registry come from the same
            # WAL snapshot while the live relay may keep appending transactions.
            original.execute('BEGIN')
            original.execute('SELECT COUNT(*) FROM sqlite_master').fetchone()
            original.backup(copied)
            copied.commit()
            # Finalize a self-contained transfer file before computing its digest.
            # A connection context manager alone does not close a Python connection.
            copied.execute('PRAGMA wal_checkpoint(TRUNCATE)').fetchone()
            copied.execute('PRAGMA journal_mode=DELETE').fetchone()
        with destination.open('rb') as finalized:
            os.fsync(finalized.fileno())
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
