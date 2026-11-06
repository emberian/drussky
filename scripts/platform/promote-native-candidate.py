#!/usr/bin/env python3
"""Select a staged pair with an accepted SAME-protocol running backend.

Reuses activate-relay-candidate.py acceptance validation. This command performs
strict read-only live checks, then updates artifact aliases/status only. It never
builds, signs, restarts services or operates a device/account.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys

import importlib.util


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


native = module('native_stage', 'stage-native-candidate.py')
activation_helper = module('relay_activation', 'activate-relay-candidate.py')
shared = activation_helper.shared


def validate_backend(ios_path, protocol, activation_path=None, accepted_relay_path=None):
    if bool(activation_path) == bool(accepted_relay_path):
        raise ValueError('Select exactly one activation receipt or accepted running relay')
    if accepted_relay_path:
        accepted, binary = activation_helper.validate_candidate(accepted_relay_path, ios_path)
        if (accepted['data_protocol'] != protocol
                or accepted['relay_executable_sha256'] != accepted['expected_previous_relay_sha256']
                or shared.prior.config_fingerprint() != accepted['private_config_sha256']):
            raise ValueError('Client-only promotion requires the same accepted backend and configuration')
        expected_pid = None
        provenance = {'mode': 'client-only', 'accepted_relay_receipt': str(Path(accepted_relay_path).resolve())}
    else:
        activation = native.read(activation_path)
        if (activation.get('accepted') is not True or activation.get('data_protocol') != protocol
                or activation.get('transition_profile') != 'same-protocol'
                or activation.get('stage') != f'active-data{protocol}-awaiting-root-alias-promotion'
                or Path(activation['native_candidate_receipt']).resolve() != ios_path
                or not activation.get('private_config_preserved') or not activation.get('artifact_aliases_preserved')):
            raise ValueError('Successful same-protocol activation must select this exact native pair')
        accepted, binary = activation_helper.validate_candidate(Path(activation['accepted_relay_receipt']), ios_path)
        if (activation['relay_executable_sha256'] != accepted['relay_executable_sha256']
                or shared.prior.config_fingerprint() != activation['config_hashes_before']
                or shared.worker_source_fingerprint() != activation['worker_source_sha256']
                or shared.alias_fingerprint() != activation['artifact_alias_hashes_before']):
            raise ValueError('Accepted relay configuration or alias identity changed')
        expected_pid = activation['relay_pid']
        provenance = {'mode': 'after-relay-activation',
                      'accepted_relay_cutover': str(Path(activation_path).resolve())}
    running = shared.services.service_status('relay')
    if (not running.get('running') or running.get('ownership') != 'recorded'
            or (expected_pid is not None and running['pid'] != expected_pid)
            or running['executable'] != str(binary)
            or running['recorded_sha256'] != accepted['relay_executable_sha256']
            or not running['on_disk_matches_receipt']):
        raise ValueError('Actual immutable relay process differs from the accepted backend')
    return accepted, {**provenance, 'relay_pid': running['pid']}


def promote(stage_path, activation_path, web_build, accepted_relay_path=None):
    staged = native.read(stage_path)
    if staged.get('stage') != 'staged' or staged.get('current_artifacts_and_aliases_preserved') is not True:
        raise ValueError('Candidate has no completed stable staging receipt')
    ios_path = Path(staged['ios_manifest']).resolve(strict=True)
    mac_path = Path(staged['macos_manifest']).resolve(strict=True)
    folder = Path(stage_path).resolve(strict=True).parent
    if ios_path.parent != folder or mac_path.parent != folder:
        raise ValueError('Candidate manifests must share their staging directory')
    for path, expected in [(ios_path, staged['ios_manifest_sha256']),
                           (mac_path, staged['macos_manifest_sha256']),
                           (Path(staged['source_receipt']), staged['source_receipt_sha256'])]:
        if native.digest(path) != expected:
            raise ValueError('Completed candidate receipt bytes changed')
    ios, mac = native.read(ios_path), native.read(mac_path)
    source = native.read(ios['source_evidence'])
    previous = source['preserved_active']
    protocol = ios['data_protocol']
    if protocol != previous['ios_receipt']['data_protocol'] or ios['reference_web_build'] != web_build:
        raise ValueError('Promotion must select the accepted web build and unchanged data protocol')
    native.verify_executable(ios)
    native.verify_executable(mac, True)
    native.verify_executable(previous['ios_receipt'])
    native.verify_executable(previous['macos_receipt'], True)
    native.assert_equal_maps(previous['alias_sha256'], native.alias_hashes(), 'Active artifact aliases')
    accepted, backend_provenance = validate_backend(ios_path, protocol, activation_path, accepted_relay_path)
    record_path = folder / 'promotion.json'
    if record_path.exists():
        raise ValueError('Candidate promotion is already recorded; inspect before retrying')
    checked = subprocess.run([sys.executable, str(native.ROOT / 'scripts/platform/check-delivery-artifact.py'),
        str(ios_path), '--expected-sync-version', str(protocol),
        '--expected-teacher-prompt', ios['teacher_prompt_version'],
        '--expected-dialogue-prompt', ios['dialogue_prompt_version'],
        '--bootstrap', str(native.TASK / 'bootstrap.json'), '--ca-cert', str(native.TASK / 'ca.pem')],
        cwd=native.ROOT, capture_output=True, text=True)
    if checked.returncode:
        raise ValueError('Strict live artifact/TLS compatibility preflight refused')
    preflight = json.loads(checked.stdout)
    if not preflight.get('artifact_verified') or not preflight.get('relay_verified'):
        raise ValueError('Strict live artifact/TLS compatibility preflight incomplete')
    capabilities = shared.authenticated_statuses(protocol)
    status = native.read(native.TASK / 'status.json')
    # Selection does not install the candidate or invalidate an accepted Apple agreement.
    agreement_accepted = status.get('apple_agreement_accepted') is True
    selected_stage = 'selected-native-update-awaiting-install' if agreement_accepted else 'waiting-for-user-apple-agreement'
    status.update(stage=selected_stage, updated_at=native.now(),
        artifact=ios['app'], frozen_manifest=str(ios_path),
        native_mac_artifact_receipt=str(mac_path), active_relay_protocol=protocol,
        device_artifact_protocol=protocol, teacher_prompt_version=ios['teacher_prompt_version'],
        dialogue_prompt_version=ios['dialogue_prompt_version'], physical_database_schema=1,
        rust_device_build='succeeded-' + staged['candidate'], native_mac_build='succeeded-' + staged['candidate'],
        content_revision=ios['content_lineage'], lesson_count=ios['lesson_count'], candidate_promoted=True,
        candidate_artifact=ios['app'], candidate_frozen_manifest=str(ios_path),
        candidate_native_mac_artifact_receipt=str(mac_path), candidate_data_protocol=protocol,
        candidate_teacher_prompt=ios['teacher_prompt_version'], candidate_dialogue_prompt=ios['dialogue_prompt_version'],
        relay_protocol_verified_at=native.now(),
        candidate_device_installed=False,
        error=None if agreement_accepted else 'Accepted native/server pairing selected. Apple agreement acceptance is still required before signing.')
    native.write(folder / 'live-preflight.json', preflight)
    native.write(native.TASK / f'frozen-artifact-v{protocol}.json', ios)
    native.write(native.TASK / 'frozen-artifact.json', ios)
    native.write(native.TASK / 'status.json', status)
    preserved = {name: sha for name, sha in previous['alias_sha256'].items()
                 if name not in ['frozen-artifact.json', f'frozen-artifact-v{protocol}.json']}
    for name, sha in preserved.items():
        if native.digest(native.TASK / name) != sha:
            raise RuntimeError('Historical artifact alias changed unexpectedly')
    record = {'promoted_at': native.now(), 'candidate': staged['candidate'], 'data_protocol': protocol,
              'reference_web_build': web_build, 'teacher_prompt_version': ios['teacher_prompt_version'],
              'dialogue_prompt_version': ios['dialogue_prompt_version'],
              'native_receipt': str(Path(stage_path).resolve()), **backend_provenance,
              'relay_executable_sha256': accepted['relay_executable_sha256'], 'live_preflight': preflight,
              'capabilities': capabilities, 'historical_alias_sha256_preserved': preserved,
              'previous_ios_sha256': previous['ios_receipt']['rust_executable_sha256'],
              'previous_macos_sha256': previous['macos_receipt']['rust_executable_sha256'],
              'ios_sha256': ios['rust_executable_sha256'], 'macos_sha256': mac['rust_executable_sha256'],
              'service_actions': 'none', 'device_actions': 'none'}
    native.write(record_path, record)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('stage_receipt', type=Path)
    backend = parser.add_mutually_exclusive_group(required=True)
    backend.add_argument('--activation-receipt', type=Path)
    backend.add_argument('--accepted-relay', type=Path,
                         help='Client-only: accepted same running backend, including private_config_sha256')
    parser.add_argument('--expected-web-build', required=True)
    args = parser.parse_args()
    try:
        result = promote(args.stage_receipt, args.activation_receipt, args.expected_web_build, args.accepted_relay)
        print(json.dumps({'promoted': True, 'candidate': result['candidate'],
                          'data_protocol': result['data_protocol'], 'device_actions': 'none'}))
    except (ValueError, RuntimeError, KeyError, OSError, json.JSONDecodeError):
        raise SystemExit('Native promotion refused or incomplete; inspect task-local receipts. No service/device action was performed.') from None


if __name__ == '__main__':
    main()
