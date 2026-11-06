#!/usr/bin/env python3
"""Construct a same-protocol acceptance after root reviews actual evidence.

Derives identities from immutable build/native receipts and runs the existing
acceptance validator before publishing a new private receipt. No build, service,
device, network or inference action. This does not assess editorial/UI quality.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import tempfile

spec = importlib.util.spec_from_file_location('relay_activation', Path(__file__).with_name('activate-relay-candidate.py'))
activation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(activation)
shared = activation.shared


def read(path):
    return json.loads(Path(path).read_text())


def construct(previous_path, build_path, native_path, evidence_paths, review_note):
    previous, build, native = read(previous_path), read(build_path), read(native_path)
    source_path = Path(native['source_evidence']).resolve(strict=True)
    source = read(source_path)
    protocol = native['data_protocol']
    if (previous.get('accepted') is not True or previous.get('data_protocol') != protocol
            or build.get('source_hashes_unchanged') is not True
            or source.get('source_hashes_unchanged') is not True):
        raise ValueError('Require accepted same-protocol predecessor and stable build/native sources')
    workers = shared.worker_source_fingerprint()
    if (not workers or previous.get('worker_source_scope') != 'files-only'
            or previous.get('worker_sources_reviewed') is not True
            or previous.get('worker_source_sha256') != workers):
        raise ValueError('Reviewed worker files changed; preserve the existing worker-review gate')
    for path in evidence_paths:
        read(path)  # Actual evidence must be valid JSON; root owns acceptance.
    return {
        'accepted': True, 'activation_kind': 'same-protocol', 'data_protocol': protocol,
        'physical_database_schema': native['physical_database_schema'],
        'previous_accepted_relay_receipt': str(previous_path),
        'previous_accepted_relay_receipt_sha256': shared.prior.digest(previous_path),
        'expected_previous_relay_sha256': previous['relay_executable_sha256'],
        'relay_executable': build['binary'], 'relay_executable_sha256': build['binary_sha256'],
        'native_manifest_sha256': shared.prior.digest(native_path),
        'source_evidence_sha256': shared.prior.digest(source_path),
        'relay_build_receipt': str(build_path), 'relay_build_receipt_sha256': shared.prior.digest(build_path),
        'relay_source_sha256': shared.shared_source_hashes(source['source_sha256']),
        'teacher_prompt_version': native['teacher_prompt_version'],
        'dialogue_prompt_version': native['dialogue_prompt_version'],
        'teacher_prompt_source_sha256': source['server_prompt_source_sha256'],
        'dialogue_prompt_source_sha256': source['dialogue_prompt_source_sha256'],
        'worker_sources_reviewed': True, 'worker_source_scope': 'files-only',
        'worker_source_sha256': workers, 'private_config_sha256': shared.prior.config_fingerprint(),
        'acceptance_evidence': [{'path': str(path), 'sha256': shared.prior.digest(path)} for path in evidence_paths],
        'reference_web_build': native['reference_web_build'], 'root_review': review_note,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['previous-accepted-relay', 'relay-build-receipt', 'native-manifest', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--evidence', type=Path, action='append', required=True)
    parser.add_argument('--review-note', required=True, help='Root acceptance scope and retained limitations')
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(shared.ROOT / '.local') or output.exists():
        parser.error('Select a new private output receipt under .local/')
    temporary = None
    try:
        value = construct(args.previous_accepted_relay.resolve(strict=True), args.relay_build_receipt.resolve(strict=True),
                          args.native_manifest.resolve(strict=True),
                          [path.resolve(strict=True) for path in args.evidence], args.review_note)
        with tempfile.NamedTemporaryFile(mode='w', dir=output.parent, prefix='.pending-acceptance-', delete=False) as stream:
            temporary = Path(stream.name)
            json.dump(value, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        activation.validate_candidate(temporary, args.native_manifest.resolve(strict=True))
        # Publish exclusively after validation; never overwrite old acceptance.
        os.link(temporary, output)
        print(json.dumps({'receipt': str(output), 'receipt_sha256': shared.prior.digest(output),
                          'data_protocol': value['data_protocol'], 'relay_sha256': value['relay_executable_sha256'],
                          'service_actions': 'none'}))
    except (ValueError, KeyError, OSError, json.JSONDecodeError):
        raise SystemExit('Acceptance construction refused; no output was published or service changed.') from None
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


if __name__ == '__main__':
    main()
