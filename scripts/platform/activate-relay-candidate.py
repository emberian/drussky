#!/usr/bin/env python3
"""Plan or execute an explicitly accepted SAME-protocol relay replacement.

Reuses the migration helper's ownership, coherent backup, TLS/status, preservation
and exact-history rollback lifecycle. Never restores a database or invokes inference.
Root acceptance must pin the previous immutable relay and actual CLI/UI evidence.
"""
import argparse
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location('relay_lifecycle', Path(__file__).with_name('cutover-sound-data6.py'))
shared = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shared)


def validate_candidate(path, native_path, _profile=None):
    accepted = json.loads(Path(path).read_text())
    protocol = accepted.get('data_protocol')
    if (accepted.get('accepted') is not True or accepted.get('activation_kind') != 'same-protocol'
            or type(protocol) is not int or protocol < 1):
        raise ValueError('Explicit same-protocol root acceptance is required')
    previous_path = Path(accepted['previous_accepted_relay_receipt'])
    previous = shared.pinned_json(previous_path, accepted['previous_accepted_relay_receipt_sha256'])
    expected = accepted['expected_previous_relay_sha256']
    old_binary = Path(previous['relay_executable']).resolve(strict=True)
    if (previous.get('accepted') is not True or type(previous.get('data_protocol')) is not int
            or previous['data_protocol'] != protocol or expected != previous['relay_executable_sha256']
            or old_binary.is_relative_to(shared.ROOT / 'target') or shared.prior.digest(old_binary) != expected):
        raise ValueError('Previous accepted immutable relay must have exactly the same protocol and pinned hash')
    binary = Path(accepted['relay_executable']).resolve(strict=True)
    if binary.is_relative_to(shared.ROOT / 'target') or shared.prior.digest(binary) != accepted['relay_executable_sha256']:
        raise ValueError('Candidate must be immutable and match its accepted SHA-256')
    shared.pinned_json(native_path, accepted['native_manifest_sha256'])
    native = shared.preflight.artifact(native_path, protocol, accepted['teacher_prompt_version'], accepted['dialogue_prompt_version'])
    source = shared.pinned_json(native['source_evidence'], accepted['source_evidence_sha256'])
    build = shared.pinned_json(accepted['relay_build_receipt'], accepted['relay_build_receipt_sha256'])
    sources = shared.shared_source_hashes(source['source_sha256'])
    if (not sources or accepted['relay_source_sha256'] != sources
            or shared.shared_source_hashes(build['source_sha256']) != sources
            or Path(build['binary']).resolve(strict=True) != binary
            or build.get('binary_sha256') != accepted['relay_executable_sha256']
            or type(build.get('event_version')) is not int or build['event_version'] != protocol
            or type(build.get('sync_version')) is not int or build['sync_version'] != protocol):
        raise ValueError('Actual relay build and native shared-source/protocol identities differ')
    for policy, source_key, build_key in [('teacher', 'server_prompt_source_sha256', 'teacher_source'),
                                           ('dialogue', 'dialogue_prompt_source_sha256', 'prompt_source')]:
        expected_source = accepted[policy + '_prompt_source_sha256']
        if (build.get(policy + '_policy') != accepted[policy + '_prompt_version']
                or source[source_key] != expected_source
                or build.get(build_key + '_sha256') != expected_source
                or shared.prior.digest(Path(build[build_key])) != expected_source):
            raise ValueError('Accepted policy identities differ from frozen source or actual relay build')
    workers = shared.worker_source_fingerprint()
    if (accepted.get('worker_sources_reviewed') is not True or accepted.get('worker_source_scope') != 'files-only'
            or not workers or accepted.get('worker_source_sha256') != workers
            or previous.get('worker_source_sha256') != workers):
        raise ValueError('Same-protocol activation requires unchanged reviewed worker files')
    captured_workers = {name: sha for name, sha in source['source_sha256'].items() if name.startswith('scripts/speech/')}
    if any(workers.get(name) != sha for name, sha in captured_workers.items()):
        raise ValueError('Native captured worker source differs from reviewed worker files')
    evidence = accepted.get('acceptance_evidence')
    if not isinstance(evidence, list) or not evidence:
        raise ValueError('Root must pin actual candidate acceptance evidence')
    for item in evidence:
        shared.pinned_json(item['path'], item['sha256'])
    return accepted, binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['plan', 'execute'], nargs='?', default='plan')
    for name in ['accepted-relay', 'native-manifest', 'output']:
        parser.add_argument('--' + name, type=Path)
    parser.add_argument('--expected-previous-sha256', required=True)
    args = parser.parse_args()
    if args.accepted_relay is None or args.native_manifest is None:
        parser.error('Explicit accepted relay receipt and native manifest are required')
    try:
        accepted, binary = validate_candidate(args.accepted_relay, args.native_manifest)
        if accepted['expected_previous_relay_sha256'] != args.expected_previous_sha256:
            raise ValueError('Selected previous relay hash differs from explicit command')
        if args.action == 'plan':
            print(json.dumps({'action': 'plan-only-no-service-change', 'data_protocol': accepted['data_protocol'],
                'candidate_executable': str(binary), 'expected_previous_sha256': args.expected_previous_sha256,
                'rollback_rule': 'Restart the previous immutable service only for exact unchanged logical history; never restore DB over new history',
                'model_calls': 0}))
            return
        if args.output is None:
            parser.error('Execution requires a unique new output directory')
        previous_path = Path(accepted['previous_accepted_relay_receipt'])
        previous = shared.pinned_json(previous_path, accepted['previous_accepted_relay_receipt_sha256'])
        args.previous_activation = previous_path
        args.rollback_executable = Path(previous['relay_executable'])
        args.expected_previous_executable = args.rollback_executable.resolve(strict=True)
        args.transition_profile = 'same-protocol'
        protocol = accepted['data_protocol']
        shared.execute(args, candidate_validator=validate_candidate, transition_spec={'from': protocol, 'to': protocol})
    except (ValueError, KeyError, FileNotFoundError, json.JSONDecodeError):
        raise SystemExit('Activation refused: incomplete or mismatched accepted receipts; no credentials printed.') from None


if __name__ == '__main__':
    main()
