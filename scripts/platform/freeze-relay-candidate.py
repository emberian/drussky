#!/usr/bin/env python3
"""Build and freeze a relay candidate for the existing acceptance validators.

Run only in the root-granted Cargo slot. No service, native app, signing, device
or inference action is performed. Failed candidates and logs are retained.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


collector = load('native_inputs', 'native-build-inputs.py')
lifecycle = load('relay_lifecycle', 'cutover-sound-data6.py')


def digest(path):
    return lifecycle.prior.digest(path)


def capture():
    complete = collector.collect(ROOT)
    selected = lifecycle.shared_source_hashes(complete['source_sha256'])
    # Include literal assets referenced by relay-dependency sources, including
    # external test fixtures, without freezing unrelated changing app sources.
    for item in complete['literal_rust_includes']:
        if item['source'] in selected:
            selected[item['file']] = complete['source_sha256'][item['file']]
    for name, sha in complete['source_sha256'].items():
        if name.startswith('.cargo/'):
            selected[name] = sha
    for filename in ['freeze-relay-candidate.py', 'native-build-inputs.py']:
        path = Path(__file__).with_name(filename)
        selected[str(path.relative_to(ROOT))] = digest(path)
    return selected


def constant(path, name, pattern):
    match = re.search(name + pattern, path.read_text())
    if not match:
        raise ValueError('Expected release constant was not found')
    return match.group(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('name', help='Unique directory name under .local/')
    parser.add_argument('--expected-sync-version', type=int, required=True)
    parser.add_argument('--expected-teacher-prompt', required=True)
    parser.add_argument('--expected-dialogue-prompt', required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,79}', args.name):
        parser.error('Use a unique lowercase directory name')
    output = ROOT / '.local' / args.name
    if output.exists():
        parser.error('Candidate already exists; preserve it and choose a new name')
    if os.environ.get('CARGO_BUILD_TARGET') or (os.environ.get('CARGO_TARGET_DIR') and Path(os.environ['CARGO_TARGET_DIR']).resolve() != ROOT / 'target'):
        parser.error('Use the native repository target directory for this relay build')
    teacher = ROOT / 'crates/rufren-tutor/src/teacher.rs'
    dialogue = ROOT / 'crates/rufren-tutor/src/dialogue.rs'
    core = ROOT / 'crates/rufren-core/src/lib.rs'
    versions = [int(constant(core, name, r':\s*u32\s*=\s*(\d+);')) for name in ['EVENT_VERSION', 'SYNC_VERSION']]
    policies = [constant(teacher, 'TEACHER_PROMPT_VERSION', r':\s*&str\s*=\s*"([^"]+)"'),
                constant(dialogue, 'PROMPT_VERSION', r':\s*&str\s*=\s*"([^"]+)"')]
    if versions != [args.expected_sync_version] * 2 or policies != [args.expected_teacher_prompt, args.expected_dialogue_prompt]:
        parser.error('Current source versions differ from the explicitly requested candidate')
    before = capture()
    output.mkdir(mode=0o700)
    log = output / 'build.log'
    start = time.monotonic()
    with log.open('x') as stream:
        result = subprocess.run(['cargo', 'build', '--locked', '-p', 'rufren-relay', '-j', '2'],
                                cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
    elapsed = time.monotonic() - start
    receipt = {'built_at': datetime.now(timezone.utc).isoformat(),
               'source_sha256': before, 'source_scope': 'Relay workspace dependencies and their literal includes; app/native changes tracked separately',
               'event_version': versions[0], 'sync_version': versions[1],
               'teacher_policy': policies[0], 'dialogue_policy': policies[1],
               'build_elapsed_seconds': elapsed, 'build_log': str(log),
               'source_hashes_unchanged': before == capture()}
    receipt_path = output / 'receipt.json'
    if result.returncode or not receipt['source_hashes_unchanged']:
        receipt['failed'] = True
        lifecycle.prior.private_write(receipt_path, receipt)
        raise SystemExit('Relay build failed or inputs changed; inspect retained candidate log/receipt. No service was changed.')
    binary = output / 'rufren-relay'
    shutil.copy2(ROOT / 'target/debug/rufren-relay', binary)
    binary.chmod(0o500)
    for source, filename in [(teacher, 'teacher.rs'), (dialogue, 'dialogue.rs')]:
        shutil.copyfile(source, output / filename)
        (output / filename).chmod(0o400)
    if before != capture() or digest(teacher) != digest(output / 'teacher.rs') or digest(dialogue) != digest(output / 'dialogue.rs'):
        receipt.update(failed=True, source_hashes_unchanged=False)
        lifecycle.prior.private_write(receipt_path, receipt)
        raise SystemExit('Sources changed during freeze; candidate retained, no service changed.')
    receipt.update(binary=str(binary), binary_sha256=digest(binary),
                   teacher_source=str(output / 'teacher.rs'), teacher_source_sha256=digest(output / 'teacher.rs'),
                   prompt_source=str(output / 'dialogue.rs'), prompt_source_sha256=digest(output / 'dialogue.rs'))
    lifecycle.prior.private_write(receipt_path, receipt)
    print(json.dumps({'receipt': str(receipt_path), 'binary': str(binary),
                      'binary_sha256': receipt['binary_sha256'], 'source_inputs': len(before),
                      'event_version': versions[0], 'teacher_policy': policies[0], 'dialogue_policy': policies[1],
                      'service_actions': 'none'}))


if __name__ == '__main__':
    main()
