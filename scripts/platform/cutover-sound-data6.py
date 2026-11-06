#!/usr/bin/env python3
"""Plan or execute a receipt-pinned private relay transition.

The historical default remains5→6 Sound; --transition-profile reader-data7 selects
6→7 Reader with Teacher6/dialogue4; review-timing-data8 selects7→8 with the same policies. Default plan changes nothing. Execute requires explicit accepted relay/native/
consumer receipts and an immutable rollback binary. It stops only the recorded
relay, verifies a quiescent SQLite+original-audio backup, and preserves TLS,
configuration and artifact aliases. Failure may restart the old binary only when
the exact logical snapshot is unchanged and contains no target-version events; it never restores over
changed history. No inference endpoint is called.
"""
import argparse
from contextlib import closing
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import ssl
import subprocess
import sys
import urllib.parse
import urllib.request

spec = importlib.util.spec_from_file_location('dialogue_cutover', Path(__file__).with_name('cutover-dialogue-data5.py'))
prior = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prior)
ROOT, FOLDER, services, preflight = prior.ROOT, prior.FOLDER, prior.services, prior.preflight
DEFAULT_PROFILE = 'sound-data6'
TRANSITIONS = {
    DEFAULT_PROFILE: {'from': 5, 'to': 6, 'consumer': 'sound'},
    'review-timing-data8': {'from': 7, 'to': 8, 'consumer': 'timing',
                          'teacher': 'rufren-ai-teacher-v6', 'dialogue': 'rufren-ai-dialogue-v4'},
    'reader-data7': {'from': 6, 'to': 7, 'consumer': 'reader',
                     'teacher': 'rufren-ai-teacher-v6', 'dialogue': 'rufren-ai-dialogue-v4'},
}


def transition(name=DEFAULT_PROFILE):
    if name not in TRANSITIONS:
        raise ValueError('Unsupported explicit relay transition profile')
    return TRANSITIONS[name]


def shared_source_hashes(values):
    return {name: sha for name, sha in values.items()
            if name in ('Cargo.toml', 'Cargo.lock') or name.startswith((
                'crates/rufren-core/', 'crates/rufren-store/', 'crates/rufren-relay/',
                'crates/rufren-tutor/', 'crates/rufren-tutor-contract/',
                'crates/rufren-speech/', 'crates/rufren-audio/'))}


def pinned_json(path, expected_hash):
    path = Path(path).resolve(strict=True)
    if prior.digest(path) != expected_hash:
        raise ValueError('Selected receipt bytes differ from their accepted SHA-256')
    return json.loads(path.read_text())


def logical_database(path, max_version=6):
    uri = 'file:' + urllib.parse.quote(str(Path(path).resolve())) + '?mode=ro'
    with closing(sqlite3.connect(uri, uri=True)) as db:
        db.execute('BEGIN')
        schema = db.execute('PRAGMA user_version').fetchone()[0]
        if schema != 1:
            raise ValueError('Unexpected physical schema; preserve history')
        tables = db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetchall()
        state = {}
        for (name,) in tables:
            quoted = '"' + name.replace('"', '""') + '"'
            columns = [row[1] for row in db.execute('PRAGMA table_info(' + quoted + ')')]
            ordered = ','.join(str(i + 1) for i in range(len(columns)))
            state[name] = {'columns': columns, 'rows': db.execute('SELECT * FROM ' + quoted + ' ORDER BY ' + ordered).fetchall()}
        count, cursor = db.execute('SELECT COUNT(*),COALESCE(MAX(seq),0) FROM events').fetchone()
        versions = [json.loads(row[0]).get('version') for row in db.execute('SELECT json FROM events')]
    if type(max_version) is not int or any(type(version) is not int or not 1 <= version <= max_version for version in versions):
        raise ValueError('Unexpected event version; preserve history and resolve forward')
    encoded = json.dumps(state, ensure_ascii=False, separators=(',', ':'), default=lambda value: {'sqlite_blob_hex': value.hex()}).encode()
    return {'sha256': hashlib.sha256(encoded).hexdigest(), 'physical_schema': schema,
            'event_count': count, 'cursor': cursor, 'max_event_version': max(versions, default=0)}


def rollback_is_safe(snapshot, current, previous_version=5):
    return (snapshot['sha256'] == current['sha256']
            and type(current['max_event_version']) is int
            and type(previous_version) is int and current['max_event_version'] <= previous_version)


def worker_source_fingerprint():
    # A file snapshot, not an attestation of code loaded by an already running
    # Python worker or of its installed model/dependency environment.
    folder = ROOT / 'scripts/speech'
    paths = set(folder.glob('*.py')) | set(folder.glob('*requirements.lock'))
    return {str(path.relative_to(ROOT)): prior.digest(path) for path in sorted(paths)}


class IncompleteAudioBackup(RuntimeError):
    pass


def run_backup(command, source, destination=None, assets=None):
    arguments = [sys.executable, str(ROOT / 'scripts/relay-backup.py'), command, str(source)]
    if destination is not None:
        arguments.append(str(destination))
    if assets is not None:
        arguments.extend(['--assets', str(assets)])
    result = subprocess.run(arguments, capture_output=True, text=True)
    if result.returncode:
        if 'referenced recording(s) have not completed recording sync' in result.stderr:
            raise IncompleteAudioBackup('Sync pending originals from their originating devices before retrying into a new destination; facts were retained')
        raise RuntimeError('Coherent SQLite/audio backup failed; retained files require inspection')
    return json.loads(Path(destination).with_suffix(Path(destination).suffix + '.json').read_text()) if command == 'backup' else json.loads(result.stdout)


def accepted_candidate(path, native_path, transition_profile=DEFAULT_PROFILE):
    profile = transition(transition_profile)
    target = profile['to']
    accepted = json.loads(path.read_text())
    if accepted.get('accepted') is not True or type(accepted.get('data_protocol')) is not int or accepted['data_protocol'] != target:
        raise ValueError(f'Root acceptance must explicitly select data protocol{target}')
    if profile['consumer'] in ('reader', 'timing') and (accepted.get('teacher_prompt_version') != profile['teacher'] or accepted.get('dialogue_prompt_version') != profile['dialogue']):
        raise ValueError('Selected transition requires exact Teacher6/dialogue4 policy identities')
    executable = Path(accepted['relay_executable']).resolve(strict=True)
    if executable.is_relative_to(ROOT / 'target') or prior.digest(executable) != accepted['relay_executable_sha256']:
        raise ValueError('Candidate must be an accepted immutable executable outside target/')
    pinned_json(native_path, accepted['native_manifest_sha256'])
    native = preflight.artifact(native_path, target, accepted['teacher_prompt_version'], accepted['dialogue_prompt_version'])
    source = pinned_json(native['source_evidence'], accepted['source_evidence_sha256'])
    if source['server_prompt_source_sha256'] != accepted['teacher_prompt_source_sha256'] or source['dialogue_prompt_source_sha256'] != accepted['dialogue_prompt_source_sha256']:
        raise ValueError('Accepted relay policy-source hashes differ from frozen client pairing')
    # Accepted source hashes bind the immutable relay build to the same frozen
    # core/store/transport sources as the client, independent of later tree edits.
    shared = shared_source_hashes(source['source_sha256'])
    if not shared or accepted['relay_source_sha256'] != shared:
        raise ValueError('Accepted relay and frozen native shared-source identities differ')
    if profile['consumer'] in ('reader', 'timing'):
        build = pinned_json(accepted['relay_build_receipt'], accepted['relay_build_receipt_sha256'])
        if (Path(build['binary']).resolve(strict=True) != executable
                or build.get('binary_sha256') != accepted['relay_executable_sha256']
                or type(build.get('sync_version')) is not int or build['sync_version'] != target
                or type(build.get('event_version')) is not int or build['event_version'] != target
                or build.get('teacher_policy') != profile['teacher'] or build.get('dialogue_policy') != profile['dialogue']
                or shared_source_hashes(build['source_sha256']) != shared
                or build.get('prompt_source_sha256') != accepted['dialogue_prompt_source_sha256']
                or prior.digest(Path(build['prompt_source'])) != build['prompt_source_sha256']):
            raise ValueError('Selected candidate build receipt differs from the actual accepted binary/shared sources/policy')
    workers = worker_source_fingerprint()
    if (accepted.get('worker_sources_reviewed') is not True
            or accepted.get('worker_source_scope') != 'files-only'
            or not workers or accepted.get('worker_source_sha256') != workers):
        raise ValueError('Worker source-file snapshot needs exact explicit root review; loaded worker code is not attested')
    captured_workers = {name: sha for name, sha in source['source_sha256'].items() if name.startswith('scripts/speech/')}
    if any(workers.get(name) != sha for name, sha in captured_workers.items()):
        raise ValueError('Captured native worker sources differ from the accepted file snapshot')
    consumer = pinned_json(accepted['consumer_receipt'], accepted['consumer_receipt_sha256'])
    if (consumer.get('passed') is not True or type(consumer.get('protocol')) is not int or consumer['protocol'] != target
            or consumer.get('relay_executable_sha256') != accepted['relay_executable_sha256']
            or consumer.get('preview_build') != source['reference_web_build']
            or consumer.get('coherent_snapshot_restored') is not True
            or consumer.get('legacy_protocol_refused') is not True
            or consumer.get('owned_service_stopped') is not True
            or type(consumer.get('original_take_records')) is not int or consumer['original_take_records'] < 0
            or not isinstance(consumer.get('played_original_asset_ids'), list)):
        raise ValueError('Accepted consumer must prove matching protocol/binary/web, coherent restore and legacy refusal')
    if profile['consumer'] != 'timing' and (consumer['original_take_records'] < 1 or not consumer['played_original_asset_ids']):
        raise ValueError('Historical Sound/Reader consumers require originals and actual playback')
    if profile['consumer'] == 'timing':
        validate_timing_receipts(accepted, consumer)
        return accepted, executable
    if profile['consumer'] == 'sound':
        if (type(consumer.get('sound_trials')) is not int or consumer['sound_trials'] < 1
                or type(consumer.get('sound_completed_trials')) is not int or consumer['sound_completed_trials'] < 1
                or not consumer.get('empty_app_sound_trials')):
            raise ValueError('Sound consumer is missing saved and unfinished trial restoration')
    else:
        reader = consumer.get('empty_app_reader')
        if (type(consumer.get('reader_documents')) is not int or consumer['reader_documents'] < 2
                or not isinstance(reader, dict) or not isinstance(reader.get('documents'), list)
                or len(reader['documents']) != consumer['reader_documents']
                or any(not isinstance(doc, dict) for doc in reader['documents'])
                or not {'ai_authored', 'learner_provided'}.issubset({doc.get('source_kind') for doc in reader['documents']})
                or not reader.get('excerpts') or not reader.get('audio') or not reader.get('producer_local_keys_absent_before_open')
                or not isinstance(reader.get('dialogues'), list)
                or any(not isinstance(path, dict) for path in reader['dialogues'])
                or not any(len({path.get('chunk_id') for path in reader['dialogues'] if path.get('document_id') == document.get('document_id')}) >= 2 for document in reader['documents'])):
            raise ValueError('Reader consumer must restore authored/custom originals, exact excerpts, distinct chunk anchors, local-state isolation and voice')
    return accepted, executable


def validate_timing_receipts(accepted, consumer):
    """Bind restored timing observations to a pinned, actual two-client UI producer."""
    producer = pinned_json(accepted['producer_receipt'], accepted['producer_receipt_sha256'])
    timing = consumer.get('empty_app_timing')
    if (producer.get('ok') is not True or producer.get('errors') != [] or producer.get('forbidden') != []
            or not isinstance(producer.get('checks'), list) or not producer['checks']
            or consumer.get('mode') != 'timing' or consumer.get('inference_calls') != 0
            or consumer.get('forbidden_requests') != 0 or consumer.get('errors') != []
            or consumer.get('compact_event_json_identical') is not True
            or consumer.get('retry_cursor_unchanged') is not True
            or not isinstance(timing, dict)):
        raise ValueError('Timing needs pinned actual producer and exact inference-free restored consumer')
    events = producer.get('events')
    if not isinstance(events, list) or not events or any(not isinstance(event, dict) for event in events):
        raise ValueError('Timing producer has no immutable event observations')
    source_hash = hashlib.sha256(json.dumps(events, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
    if consumer.get('timing_source_events_sha256') != source_hash:
        raise ValueError('Restored timing history differs from the actual producer event bytes')
    by_id = {event['id']: event for event in events}
    if len(by_id) != len(events) or consumer.get('events') != len(events):
        raise ValueError('Timing producer/consumer event identities differ')
    starts, tasks, cadence, proof = (timing.get(key) for key in ('starts', 'tasks', 'cadence', 'producer_checks'))
    if (not isinstance(starts, list) or not starts or not isinstance(tasks, list) or not tasks
            or not isinstance(cadence, dict) or not isinstance(proof, dict)):
        raise ValueError('Timing restored cadence, exact task states and immutable starts are required')
    observed_starts = []
    for start in starts:
        if not isinstance(start, dict):
            raise ValueError('Malformed restored start')
        event = by_id.get(start.get('event_id'), {})
        snapshot = {key: value for key, value in start.items() if key != 'event_id'}
        if event.get('fact') != {'kind': 'review_started', 'data': snapshot}:
            raise ValueError('Restored immutable start differs from the producer event')
        observed_starts.append(event)
    if (producer.get('starts') != observed_starts
            or {event['id'] for event in observed_starts} != {event['id'] for event in events if event.get('fact', {}).get('kind') == 'review_started'}):
        raise ValueError('Restored starts omit or change producer pinned start observations')
    for key, kind in [('divergent_cadence_ids', 'review_cadence_changed'), ('divergent_timing_ids', 'review_timing_changed')]:
        ids = proof.get(key)
        if not isinstance(ids, list) or len(ids) != 2 or len(set(ids)) != 2:
            raise ValueError('Timing must retain both divergent causal choices')
        pair = [by_id.get(id, {}) for id in ids]
        if any(event.get('fact', {}).get('kind') != kind for event in pair) or pair[0].get('device') == pair[1].get('device'):
            raise ValueError('Timing divergent choices must come from two actual producer clients')
        a, b = [event['fact']['data'] for event in pair]
        if ids[0] in b['parents'] or ids[1] in a['parents']:
            raise ValueError('Timing choices were sequential rather than independent causal branches')
        if kind == 'review_cadence_changed' and a['cadence'] == b['cadence']:
            raise ValueError('Timing cadence fixture has no conflicting preferences')
        if kind == 'review_timing_changed' and (a['target'] != b['target'] or a['action'] == b['action']):
            raise ValueError('Timing fixture must conflict on the same exact task')
    late = proof.get('late_arrival_ids')
    prior_ids = proof.get('prior_start_event_ids')
    if (not isinstance(prior_ids, list) or not prior_ids
            or any(id not in {start['event_id'] for start in starts} for id in prior_ids)
            or not isinstance(late, list) or not late or any(id not in by_id for id in late)
            or any(id in start.get('evidence_ids', []) for id in late for start in starts if start['event_id'] in prior_ids)):
        raise ValueError('Late history must survive without rewriting earlier start evidence')
    if not any(by_id[id]['fact']['kind'] in ('passage_revised', 'reader_document_recorded', 'teacher_document_recorded', 'stimulus_presented', 'attempt_context_recorded') for id in late):
        raise ValueError('Timing recovery needs actual late source/support history, not only a later preference')
    resumed = proof.get('paused_draft_resumed', {})
    if (not resumed.get('before') or resumed.get('before') != resumed.get('after')
            or producer.get('draft') != resumed['before']
            or not any(start.get('session') == resumed.get('session') for start in starts)):
        raise ValueError('Timing producer must resume the original pinned unfinished response')
    for state in [cadence, *tasks]:
        if (not isinstance(state, dict) or not isinstance(state.get('heads'), list)
                or any(type(state.get(key)) is not bool for key in ('conflicted', 'pending'))):
            raise ValueError('Restored timing state is missing explicit causal heads/conflict status')
    if any(type(task.get('paused')) is not bool or not isinstance(task.get('target'), dict) or 'effective_due_at_ms' not in task
           or (task.get('effective_due_at_ms') is not None and type(task['effective_due_at_ms']) is not int)
           for task in tasks):
        raise ValueError('Restored exact task due/paused states are malformed')
    if consumer['original_take_records'] and not consumer['played_original_asset_ids']:
        raise ValueError('Timing fixture with originals must retain actual playback evidence')


def authenticated_statuses(expected_protocol):
    # Validate exact HTTPS origin and CA before attaching credentials. No ambient
    # proxies or redirect hops are permitted, even to another trusted hostname.
    preflight.relay_health(FOLDER / 'bootstrap.json', FOLDER / 'ca.pem', expected_protocol)
    bootstrap = json.loads((FOLDER / 'bootstrap.json').read_text())
    origin = bootstrap['RUFREN_RELAY_URL'].rstrip('/')
    url = urllib.parse.urlsplit(origin)
    if url.scheme != 'https' or not url.hostname or url.username or url.password or url.query or url.fragment or url.path not in ('', '/'):
        raise ValueError('Authenticated status requires an exact plain HTTPS relay origin')
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), preflight.NoRedirect(),
        urllib.request.HTTPSHandler(context=ssl.create_default_context(cafile=str(FOLDER / 'ca.pem'))))
    results = {}
    for name, route in [('teacher', '/api/teacher/status'), ('speech', '/api/speech/status'), ('recognition', '/api/speech/recognition/status')]:
        request = urllib.request.Request(origin + route, headers={'Authorization': 'Bearer ' + bootstrap['RUFREN_SYNC_TOKEN']})
        with opener.open(request, timeout=15) as response:
            body = response.read(65537)
            if response.status != 200 or len(body) > 65536:
                raise ValueError('Authenticated capability status failed or exceeded its bound')
            value = json.loads(body)
        if value.get('enabled') is not True:
            raise ValueError('Required private capability is disabled')
        results[name] = {key: value[key] for key in ['enabled', 'provider', 'model', 'lesson_flow', 'dialogue'] if key in value}
    expected = {'teacher': 'claude_cli', 'speech': 'local_russian', 'recognition': 'local_qwen3_asr'}
    if (any(results[name].get('provider') != provider for name, provider in expected.items())
            or results['teacher'].get('lesson_flow') != 'guided_v1'
            or results['teacher'].get('dialogue') != 'threaded_v1'):
        raise ValueError('Accepted private provider or guided/dialogue capability changed')
    return results


def alias_fingerprint():
    return {path.name: prior.digest(path) for path in sorted(FOLDER.glob('frozen-artifact*.json'))}


def execute(args):
    selected = getattr(args, 'transition_profile', DEFAULT_PROFILE)
    profile = transition(selected)
    previous_version, target = profile['from'], profile['to']
    database_state = lambda path: logical_database(path, target)
    accepted, executable = accepted_candidate(args.accepted_relay, args.native_manifest, selected)
    native = json.loads(args.native_manifest.read_text())
    if prior.digest(FOLDER / 'ca.pem') != native['ca_sha256']:
        raise ValueError('Private CA differs from the frozen native trust certificate')
    previous = json.loads(args.previous_activation.read_text())
    rollback = args.rollback_executable.resolve(strict=True)
    if (type(previous.get('data_protocol')) is not int or previous['data_protocol'] != previous_version
            or previous.get('accepted') is not True or rollback.is_relative_to(ROOT / 'target')
            or prior.digest(rollback) != previous['relay_executable_sha256']):
        raise ValueError(f'Rollback must be the accepted immutable data{previous_version} executable')
    active = json.loads((FOLDER / 'frozen-artifact.json').read_text())
    if type(active.get('data_protocol')) is not int or active['data_protocol'] != previous_version:
        raise ValueError(f'Active artifact is no longer data{previous_version}; inspect current rollout')
    before, proxy = services.service_status('relay'), services.service_status('tls-proxy')
    if (not before['running'] or not proxy['running'] or before['ownership'] != 'recorded'
            or before['recorded_sha256'] != previous['relay_executable_sha256'] or not before['on_disk_matches_receipt']):
        raise ValueError(f'Running relay/TLS provenance does not match accepted data{previous_version}')
    authenticated_statuses(previous_version)
    config, aliases, workers = prior.config_fingerprint(), alias_fingerprint(), worker_source_fingerprint()
    environment = json.loads((FOLDER / 'relay-environment.json').read_text())
    db, assets = Path(environment['RUFREN_RELAY_DB']), Path(environment['RUFREN_RELAY_ASSETS'])
    if database_state(db)['max_event_version'] > previous_version:
        raise ValueError('Existing history contains future events; resolve forward')
    output = args.output.resolve()
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    report = {'stage': 'prepared', 'transition_profile': selected, 'from_protocol': previous_version,
              'started_at': datetime.now(timezone.utc).isoformat(),
              'accepted_relay_receipt': str(args.accepted_relay.resolve()),
              'native_candidate_receipt': str(args.native_manifest.resolve()),
              'candidate_executable': str(executable), 'relay_executable_sha256': accepted['relay_executable_sha256'],
              'rollback_executable': str(rollback), 'rollback_sha256': previous['relay_executable_sha256'],
              'config_hashes_before': config, 'artifact_alias_hashes_before': aliases,
              'worker_source_sha256': workers, 'worker_source_scope': 'files-only', 'running_worker_code_attested': False,
              'old_owned_process': before, 'physical_database_schema': 1, 'model_calls': 0}
    receipt = output / 'receipt.json'
    prior.private_write(receipt, report)
    stopped = False
    baseline = None
    snapshot = None
    try:
        services.stop('relay')
        stopped = True
        if services.owned_pid('relay') is not None:
            raise RuntimeError('Owned relay did not stop; cutover refused')
        baseline = database_state(db)
        snapshot_db = output / f'pre-data{target}.sqlite'
        report['backup'] = run_backup('backup', db, snapshot_db, assets)
        report['backup_verified'] = run_backup('verify', snapshot_db)
        snapshot = database_state(snapshot_db)
        if database_state(db) != snapshot or baseline != snapshot:
            raise ValueError('Source changed while relay stopped; cutover refused')
        report.update(stage='quiescent-snapshot-verified', snapshot=snapshot, snapshot_database=str(snapshot_db))
        prior.private_write(receipt, report)
        services.start('relay', str(FOLDER / 'provider-environment.json'), executable, accepted['relay_executable_sha256'])
        report['capabilities'] = authenticated_statuses(target)
        if prior.config_fingerprint() != config or alias_fingerprint() != aliases or worker_source_fingerprint() != workers or services.service_status('tls-proxy')['pid'] != proxy['pid']:
            raise ValueError('Private configuration, aliases or TLS proxy changed')
        running = services.service_status('relay')
        if running['executable'] != str(executable) or running['recorded_sha256'] != accepted['relay_executable_sha256'] or not running['on_disk_matches_receipt']:
            raise ValueError('Running candidate executable provenance failed')
        report.update(stage=f'active-data{target}-awaiting-root-alias-promotion', completed_at=datetime.now(timezone.utc).isoformat(),
                      relay_pid=running['pid'], data_protocol=target, accepted=True, running_candidate=running,
                      private_config_preserved=True, artifact_aliases_preserved=True, logical_database_after=database_state(db))
        prior.private_write(receipt, report)
        print(json.dumps({'activated': True, 'data_protocol': target, 'receipt': str(receipt), 'artifact_aliases_changed': False, 'model_calls': 0}))
    except BaseException as error:
        report.update(stage='cutover-failed-history-retained', error_type=type(error).__name__)
        if isinstance(error, IncompleteAudioBackup):
            report['backup_problem'] = 'Recording facts are valid but their original audio has not completed sync. Sync originals from originating devices, then retry with a new output directory; saved facts and partial backup files were retained.'
        try:
            if stopped:
                running = services.service_status('relay')
                if running['running']:
                    if running.get('executable') != str(executable):
                        raise RuntimeError('Unexpected process; no rollback signal sent')
                    services.stop('relay')
                current = database_state(db)
                report['logical_database_at_failure'] = current
                safe = rollback_is_safe(snapshot or baseline, current, previous_version) if snapshot or baseline else False
                if safe and prior.config_fingerprint() == config and alias_fingerprint() == aliases and worker_source_fingerprint() == workers:
                    services.start('relay', str(FOLDER / 'provider-environment.json'), rollback, previous['relay_executable_sha256'])
                    authenticated_statuses(previous_version)
                    report.update(stage=f'safe-data{previous_version}-restart-no-history-restored', rollback_started=True,
                                  rollback_owned_process=services.service_status('relay'))
                else:
                    report.update(rollback_started=False, recovery=f'Retain DB/audio and snapshots; repair forward with data{target}. No old binary or restore was applied over changed history.')
        except BaseException as recovery_error:
            report.update(recovery_error_type=type(recovery_error).__name__, rollback_started=False)
        prior.private_write(receipt, report)
        raise SystemExit('Cutover incomplete; inspect private receipt. No history was overwritten/restored.') from None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['plan', 'execute'], nargs='?', default='plan')
    parser.add_argument('--transition-profile', choices=sorted(TRANSITIONS), default=DEFAULT_PROFILE)
    for name in ['accepted-relay', 'native-manifest', 'previous-activation', 'rollback-executable', 'output']:
        parser.add_argument('--' + name, type=Path)
    args = parser.parse_args()
    profile = transition(args.transition_profile)
    if args.action == 'plan':
        print(json.dumps({'action': 'plan-only-no-service-change', 'transition_profile': args.transition_profile,
                          'from_protocol': profile['from'], 'to_protocol': profile['to'],
                          'teacher_prompt_version': profile.get('teacher'), 'dialogue_prompt_version': profile.get('dialogue'),
                          'requires': ['accepted immutable relay/native/source/consumer receipts', f'accepted immutable{profile["from"]} rollback binary', 'new coherent snapshot directory'],
                          'rollback_rule': f'Exact unchanged logical DB snapshot and no{profile["to"]} events; otherwise preserve and fix forward',
                          'artifact_aliases_changed': False, 'model_calls': 0}))
        return
    if any(getattr(args, name.replace('-', '_')) is None for name in ['accepted-relay', 'native-manifest', 'previous-activation', 'rollback-executable', 'output']):
        parser.error('execute requires all five explicit receipt/binary/output arguments')
    try:
        execute(args)
    except (ValueError, KeyError, FileNotFoundError, json.JSONDecodeError):
        raise SystemExit('Cutover preflight refused: incomplete or mismatched private receipts; no credentials were printed.') from None


if __name__ == '__main__':
    main()
