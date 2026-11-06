#!/usr/bin/env python3
"""Plan or execute an accepted private relay4→5 cutover; never replace history.

Default `plan` is read-only. Execution stops only an ownership-checked relay,
backs up its quiescent DB+originals, verifies, and starts a hash-pinned immutable
candidate. TLS/config identities and artifact aliases are not changed here.
Rollback4 is allowed only when every logical DB row still equals the snapshot
and no event5 exists. Otherwise preserve all history and fail closed for fix-forward.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import time
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / '.local/ipad-delivery'
spec = importlib.util.spec_from_file_location('private_services', Path(__file__).with_name('private-relay-services.py'))
services = importlib.util.module_from_spec(spec)
spec.loader.exec_module(services)
check_spec = importlib.util.spec_from_file_location('delivery_preflight', Path(__file__).with_name('check-delivery-artifact.py'))
preflight = importlib.util.module_from_spec(check_spec)
check_spec.loader.exec_module(preflight)


def digest(path):
    return services.digest(path)


def private_write(path, value):
    services.atomic_json(path, value)


def config_fingerprint():
    names = ['bootstrap.json', 'relay-environment.json', 'provider-environment.json',
             'build-environment.json', 'private-relay.json', 'Caddyfile', 'ca.pem',
             'ca-key.pem', 'relay.pem', 'relay-key.pem']
    return {name: digest(FOLDER / name) for name in names}


def logical_database(path):
    """Exact SQLite user-row strings/blobs, independent of WAL/header layout."""
    uri = 'file:' + urllib.parse.quote(str(Path(path).resolve())) + '?mode=ro'
    with sqlite3.connect(uri, uri=True) as db:
        db.execute('BEGIN')  # One read snapshot for every table/version/count.
        schema = db.execute('PRAGMA user_version').fetchone()[0]
        if schema != 1:
            raise ValueError('Unexpected physical DB schema; no cutover action permitted')
        tables = db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetchall()
        state = {}
        for (name,) in tables:
            quoted = '"' + name.replace('"', '""') + '"'
            columns = [row[1] for row in db.execute('PRAGMA table_info(' + quoted + ')')]
            ordered = ','.join(str(i + 1) for i in range(len(columns)))
            state[name] = {'columns': columns, 'rows': db.execute('SELECT * FROM ' + quoted + ' ORDER BY ' + ordered).fetchall()}
        count, cursor = db.execute('SELECT COUNT(*),COALESCE(MAX(seq),0) FROM events').fetchone()
        versions = [json.loads(row[0])['version'] for row in db.execute('SELECT json FROM events')]
    encoded = json.dumps(state, ensure_ascii=False, separators=(',', ':'), default=lambda value: {'sqlite_blob_hex': value.hex()}).encode()
    return {'sha256': hashlib.sha256(encoded).hexdigest(), 'physical_schema': schema,
            'event_count': count, 'cursor': cursor, 'max_event_version': max(versions, default=0)}


def rollback_is_safe(snapshot, current):
    return snapshot['sha256'] == current['sha256'] and current['max_event_version'] <= 4


def accepted_candidate(path, native_path):
    accepted = json.loads(path.read_text())
    if accepted.get('accepted') is not True or accepted.get('data_protocol') != 5:
        raise ValueError('Root acceptance must explicitly select data protocol5')
    executable = Path(accepted['relay_executable']).resolve(strict=True)
    if executable.is_relative_to(ROOT / 'target') or digest(executable) != accepted['relay_executable_sha256']:
        raise ValueError('Candidate must be an accepted immutable executable outside target/')
    native = preflight.artifact(native_path, 5, accepted['teacher_prompt_version'], accepted['dialogue_prompt_version'])
    source = json.loads(Path(native['source_evidence']).read_text())
    if source['server_prompt_source_sha256'] != accepted['teacher_prompt_source_sha256'] or source['dialogue_prompt_source_sha256'] != accepted['dialogue_prompt_source_sha256']:
        raise ValueError('Accepted relay policy-source hashes differ from frozen client pairing')
    response_path = Path(accepted['accepted_dialogue_response']).resolve(strict=True)
    raw = response_path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != accepted['accepted_dialogue_response_sha256']:
        raise ValueError('Accepted dialogue response bytes changed')
    response = json.loads(raw)
    actual_tag = response['provenance']['prompt_version']
    if actual_tag != accepted['accepted_dialogue_response_prompt_version'] or actual_tag.split('+', 1)[0] != accepted['dialogue_prompt_version']:
        raise ValueError('Exact accepted dialogue prompt provenance does not match policy')
    if response['provenance']['response_id'] != accepted['accepted_dialogue_response_id']:
        raise ValueError('Accepted dialogue response identity changed')
    return accepted, executable


def authenticated_statuses():
    bootstrap = json.loads((FOLDER / 'bootstrap.json').read_text())
    origin = bootstrap['RUFREN_RELAY_URL'].rstrip('/')
    # Reuse strict private-CA/no-proxy/no-redirect handling; no inference routes.
    import ssl
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), preflight.NoRedirect(),
        urllib.request.HTTPSHandler(context=ssl.create_default_context(cafile=str(FOLDER / 'ca.pem'))))
    results = {}
    for name, route in [('teacher', '/api/teacher/status'), ('speech', '/api/speech/status'), ('recognition', '/api/speech/recognition/status')]:
        request = urllib.request.Request(origin + route, headers={'Authorization': 'Bearer ' + bootstrap['RUFREN_SYNC_TOKEN']})
        with opener.open(request, timeout=15) as response:
            if response.status != 200:
                raise ValueError('Authenticated capability status failed')
            value = json.loads(response.read(65536))
        if value.get('enabled') is not True:
            raise ValueError('Required private capability is disabled')
        results[name] = {key: value[key] for key in ['enabled', 'provider', 'model', 'lesson_flow', 'dialogue'] if key in value}
    if results['teacher'].get('lesson_flow') != 'guided_v1' or results['teacher'].get('dialogue') != 'threaded_v1':
        raise ValueError('Candidate is missing guided/dialogue capability')
    expected_providers = {'teacher': 'claude_cli', 'speech': 'local_russian', 'recognition': 'local_qwen3_asr'}
    if any(results[name].get('provider') != provider for name, provider in expected_providers.items()):
        raise ValueError('Accepted private teacher/speech/recognition provider changed')
    return results


def run_backup(command, source, destination=None, assets=None):
    arguments = ['python3', str(ROOT / 'scripts/relay-backup.py'), command, str(source)]
    if destination is not None:
        arguments.append(str(destination))
    if assets is not None:
        arguments.extend(['--assets', str(assets)])
    result = subprocess.run(arguments, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError('Coherent DB/audio backup or verification failed; private data was retained')
    # The backup CLI prints a human summary; its durable JSON contract is the
    # destination manifest. Only `verify` emits a JSON response on stdout.
    if command == 'backup':
        if destination is None:
            raise ValueError('Backup destination is required')
        return json.loads(Path(destination).with_suffix(Path(destination).suffix + '.json').read_text())
    return json.loads(result.stdout)


def execute(args):
    accepted, executable = accepted_candidate(args.accepted_relay, args.native_manifest)
    previous = json.loads(args.previous_activation.read_text())
    rollback = args.rollback_executable.resolve(strict=True)
    if rollback.is_relative_to(ROOT / 'target') or digest(rollback) != previous['relay_executable_sha256']:
        raise ValueError('Immutable rollback executable differs from accepted data4 binary')
    active = json.loads((FOLDER / 'frozen-artifact.json').read_text())
    if active['data_protocol'] != 4:
        raise ValueError('Active artifact is no longer data4; inspect current rollout before proceeding')
    before_status = services.service_status('relay')
    proxy_before = services.service_status('tls-proxy')
    if not before_status['running'] or not proxy_before['running']:
        raise ValueError('Existing owned relay/TLS proxy must be running before planned cutover')
    accepted_running = (before_status['ownership'] == 'recorded' and before_status['recorded_sha256'] == previous['relay_executable_sha256'] and before_status['on_disk_matches_receipt'])
    if not accepted_running and before_status['pid'] != previous['relay_pid']:
        raise ValueError('Running relay does not match accepted data4 provenance')
    preflight.relay_health(FOLDER / 'bootstrap.json', FOLDER / 'ca.pem', 4)
    config_before = config_fingerprint()
    aliases_before = {name: digest(FOLDER / name) for name in ['frozen-artifact.json', 'frozen-artifact-v3.json', 'frozen-artifact-v4.json']}
    environment = json.loads((FOLDER / 'relay-environment.json').read_text())
    db = Path(environment['RUFREN_RELAY_DB']); assets = Path(environment['RUFREN_RELAY_ASSETS'])
    if logical_database(db)['max_event_version'] > 4:
        raise ValueError('Existing history already contains future events; preserve it and resolve forward')
    output = args.output.resolve()
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    report = {'stage': 'prepared', 'started_at': datetime.now(timezone.utc).isoformat(),
              'accepted_relay_receipt': str(args.accepted_relay.resolve()),
              'native_candidate_receipt': str(args.native_manifest.resolve()),
              'candidate_executable': str(executable), 'candidate_sha256': accepted['relay_executable_sha256'],
              'rollback_executable': str(rollback), 'rollback_sha256': previous['relay_executable_sha256'],
              'config_hashes_before': config_before, 'artifact_alias_hashes_before': aliases_before,
              'old_owned_process': before_status, 'physical_database_schema': 1,
              'teacher_prompt_version': accepted['teacher_prompt_version'], 'dialogue_prompt_version': accepted['dialogue_prompt_version']}
    report_path = output / 'receipt.json'
    private_write(report_path, report)
    stopped = False; snapshot = None
    try:
        services.stop('relay'); stopped = True
        if services.owned_pid('relay') is not None:
            raise RuntimeError('Old owned relay did not stop; cutover refused')
        # Stop before the snapshot: no writes can fall between capture and cutover.
        snapshot_db = output / 'pre-data5.sqlite'
        report['backup'] = run_backup('backup', db, snapshot_db, assets)
        report['backup_verified'] = run_backup('verify', snapshot_db)
        snapshot = logical_database(snapshot_db)
        if logical_database(db) != snapshot:
            raise ValueError('Live source changed while relay was stopped; do not cut over')
        report.update(stage='quiescent-snapshot-verified', snapshot=snapshot, snapshot_database=str(snapshot_db))
        private_write(report_path, report)
        services.start('relay', str(FOLDER / 'provider-environment.json'), executable, accepted['relay_executable_sha256'])
        preflight.relay_health(FOLDER / 'bootstrap.json', FOLDER / 'ca.pem', 5)
        report['capabilities'] = authenticated_statuses()
        if config_fingerprint() != config_before:
            raise ValueError('Private configuration or CA changed during cutover')
        if services.service_status('tls-proxy')['pid'] != proxy_before['pid']:
            raise ValueError('TLS proxy process changed during relay-only cutover')
        if any(digest(FOLDER / name) != value for name, value in aliases_before.items()):
            raise ValueError('Artifact aliases changed outside this relay-only cutover')
        running = services.service_status('relay')
        if running['executable'] != str(executable) or running['recorded_sha256'] != accepted['relay_executable_sha256'] or not running['on_disk_matches_receipt']:
            raise ValueError('Running candidate executable provenance failed')
        report.update(stage='active-data5-awaiting-root-alias-promotion', completed_at=datetime.now(timezone.utc).isoformat(),
                      relay_pid=running['pid'], data_protocol=5, accepted=True,
                      private_config_preserved=True, artifact_aliases_preserved=True,
                      running_candidate=running, logical_database_after=logical_database(db))
        private_write(report_path, report)
        print(json.dumps({'activated': True, 'data_protocol': 5, 'receipt': str(report_path), 'artifact_aliases_changed': False, 'model_calls': 0}))
    except BaseException as error:
        report.update(stage='cutover-failed-history-retained', error_type=type(error).__name__)
        try:
            if stopped:
                running = services.service_status('relay')
                if running['running']:
                    if running.get('executable') != str(executable):
                        raise RuntimeError('Unexpected running process; no rollback signal sent')
                    services.stop('relay')
                current = logical_database(db)
                report['logical_database_at_failure'] = current
                # Snapshot failure occurs before candidate start. If no snapshot was
                # finalized, there was no candidate write; still verify no event5.
                safe = rollback_is_safe(snapshot, current) if snapshot else current['max_event_version'] <= 4
                if safe and config_fingerprint() == config_before:
                    services.start('relay', str(FOLDER / 'provider-environment.json'), rollback, previous['relay_executable_sha256'])
                    preflight.relay_health(FOLDER / 'bootstrap.json', FOLDER / 'ca.pem', 4)
                    report.update(stage='safe-data4-restart-no-history-restored', rollback_started=True,
                                  rollback_owned_process=services.service_status('relay'))
                else:
                    report.update(rollback_started=False, recovery='Preserve all current DB/audio and snapshot files; repair/read forward with data5. No old binary or restore was applied over changed history.')
        except BaseException as recovery_error:
            report.update(recovery_error_type=type(recovery_error).__name__, rollback_started=False)
        private_write(report_path, report)
        raise SystemExit('Cutover did not complete; inspect private receipt. No history was overwritten/restored.') from None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['plan', 'execute'], nargs='?', default='plan')
    parser.add_argument('--accepted-relay', type=Path)
    parser.add_argument('--native-manifest', type=Path)
    parser.add_argument('--previous-activation', type=Path, default=FOLDER / 'relay-v6-data4-activation.json')
    parser.add_argument('--rollback-executable', type=Path, default=ROOT / '.local/teacher-revisit-v6-live/rufren-relay')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.action == 'plan':
        previous = json.loads(args.previous_activation.read_text())
        rollback = args.rollback_executable.resolve(strict=True)
        valid = digest(rollback) == previous['relay_executable_sha256']
        print(json.dumps({'action': 'plan-only-no-service-change', 'rollback_executable': str(rollback),
                          'rollback_sha256': digest(rollback), 'rollback_matches_accepted_data4': valid,
                          'owned_relay': services.service_status('relay'), 'owned_tls_proxy': services.service_status('tls-proxy'),
                          'execution_requires': ['accepted immutable relay5 receipt and final native manifest',
                            'root accepted UI/consumer/live dialogue quality', 'new output directory for quiescent DB+audio snapshot'],
                          'rollback_rule': 'Only exact unchanged logical DB snapshot with no5events; otherwise preserve everything and fix forward',
                          'artifact_aliases_changed': False}, indent=2))
        return
    if not args.accepted_relay or not args.native_manifest or not args.output:
        parser.error('execute requires --accepted-relay, --native-manifest and a new --output directory')
    execute(args)


if __name__ == '__main__':
    main()
