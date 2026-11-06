#!/usr/bin/env python3
"""Stage an immutable iOS/Mac pair; never select aliases or operate devices.

Coordinate the exclusive build slot before invoking this command. It performs
serial Dioxus builds and local Mac ad-hoc signing only. --check-only performs
read-only input validation. Private build-environment values are never printed.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
TASK = ROOT / '.local/ipad-delivery'


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


inputs = module('native_inputs', 'native-build-inputs.py')
preflight = module('delivery_preflight', 'check-delivery-artifact.py')
services = module('private_services', 'private-relay-services.py')


def now():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    services.atomic_json(path, value)


def verify_executable(manifest, macos=False):
    app = Path(manifest['app']).resolve(strict=True)
    if app.is_relative_to(ROOT / 'target'):
        raise ValueError('Selected bundle must be immutable outside target/')
    info = plistlib.loads((app / ('Contents/Info.plist' if macos else 'Info.plist')).read_bytes())
    if info['CFBundleIdentifier'] != 'town.lunar.rufren':
        raise ValueError('Unexpected selected bundle identifier')
    executable = (app / ('Contents/MacOS' if macos else '') / info['CFBundleExecutable']).resolve(strict=True)
    if not executable.is_relative_to(app) or digest(executable) != manifest['rust_executable_sha256']:
        raise ValueError('Selected frozen executable differs from its receipt')
    return executable


def alias_hashes():
    paths = [TASK / 'frozen-artifact.json']
    paths += [p for p in TASK.glob('frozen-artifact-v*.json')
              if re.fullmatch(r'frozen-artifact-v\d+\.json', p.name)]
    return {p.name: digest(p) for p in sorted(set(paths))}


def source_map():
    result = inputs.collect(ROOT)
    for name in ['stage-native-candidate.py', 'check-delivery-artifact.py']:
        path = Path(__file__).with_name(name)
        result['source_sha256'][str(path.relative_to(ROOT))] = digest(path)
    return result


def assert_equal_maps(before, after, label):
    if before != after:
        raise ValueError(label + ' changed; retained candidate cannot be selected')


def configuration(args):
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,79}', args.name):
        raise ValueError('Use a unique lowercase candidate name with letters, digits and hyphens')
    if not re.fullmatch(r'[0-9a-f]{20}', args.web_build):
        raise ValueError('Expected the accepted 20-character prepared web build hash')
    folder = TASK / ('frozen-' + args.name)
    if folder.exists():
        raise ValueError('Candidate directory already exists; preserve it and choose a new name')
    active = read(TASK / 'frozen-artifact.json')
    preflight.artifact(TASK / 'frozen-artifact.json', active['data_protocol'])
    status = read(TASK / 'status.json')
    mac_path = Path(status['native_mac_artifact_receipt'])
    mac = read(mac_path)
    verify_executable(active)
    verify_executable(mac, True)
    if any(mac.get(key) != active.get(key) for key in ['data_protocol', 'reference_web_build',
                                                       'teacher_prompt_version', 'dialogue_prompt_version']):
        raise ValueError('Selected iOS and Mac receipts do not describe the same release')
    core = (ROOT / 'crates/rufren-core/src/lib.rs').read_text()
    for name in ['EVENT_VERSION', 'SYNC_VERSION']:
        match = re.search(r'pub const ' + name + r':\s*u32\s*=\s*(\d+);', core)
        if not match or int(match.group(1)) != args.sync_version:
            raise ValueError('Current source protocol differs from the requested release')
    for filename, constant, expected in [('teacher.rs', 'TEACHER_PROMPT_VERSION', args.teacher_prompt),
                                          ('dialogue.rs', 'PROMPT_VERSION', args.dialogue_prompt)]:
        text = (ROOT / 'crates/rufren-tutor/src' / filename).read_text()
        match = re.search(constant + r':\s*&str\s*=\s*"([^"]+)"', text)
        if not match or match.group(1) != expected:
            raise ValueError('Expected server policy differs from the frozen source')
    environment_path = TASK / 'build-environment.json'
    build_environment = read(environment_path)
    if set(build_environment) != {'RUFREN_DEV_CA_CERT', 'RUFREN_DEV_RELAY_ORIGINS'}:
        raise ValueError('Unexpected private build-environment keys')
    if not all(isinstance(value, str) and value for value in build_environment.values()):
        raise ValueError('Private build environment is incomplete')
    ca = Path(build_environment['RUFREN_DEV_CA_CERT']).resolve(strict=True)
    web_path = ROOT / 'target/dx/rufren-app/debug/web/public/offline-build.json'
    if read(web_path)['version'] != args.web_build:
        raise ValueError('Prepared web artifact differs from the accepted reference')
    pack = read(ROOT / 'crates/rufren-content/data/beginner-v1.json')
    reader_path = ROOT / 'crates/rufren-content/content/reader-v1.json'
    reader = read(reader_path) if reader_path.exists() else None
    return {'folder': folder, 'active': active, 'mac': mac, 'mac_receipt': str(mac_path),
            'aliases': alias_hashes(), 'build_environment': build_environment,
            'environment_sha256': digest(environment_path), 'ca': ca,
            'ca_sha256': digest(ca), 'web_path': web_path, 'web_sha256': digest(web_path),
            'pack': pack, 'reader': reader}


def find_app(platform):
    candidates = []
    for app in (ROOT / 'target/dx/rufren-app/debug' / platform).glob('*.app'):
        info = app / ('Contents/Info.plist' if platform == 'macos' else 'Info.plist')
        if info.exists() and plistlib.loads(info.read_bytes()).get('CFBundleIdentifier') == 'town.lunar.rufren':
            candidates.append(app)
    if len(candidates) != 1:
        raise ValueError('Cannot uniquely identify the actual Dioxus bundle')
    return candidates[0]


def stage(args, config):
    folder = config['folder']
    folder.mkdir(mode=0o700)
    receipt_path = folder / 'receipt.json'
    source_path = folder / 'source.json'
    receipt = {'stage': 'capturing', 'candidate': args.name, 'started_at': now(),
               'artifact_aliases_changed': False, 'device_actions': 'none'}
    write(receipt_path, receipt)

    def run(name, command):
        receipt['stage'] = name
        write(receipt_path, receipt)
        started = time.monotonic()
        log = folder / (name + '.log')
        with log.open('x') as output:
            result = subprocess.run(command, cwd=ROOT, env=environment,
                                    stdout=output, stderr=subprocess.STDOUT)
        if result.returncode:
            raise RuntimeError('Native step failed; inspect retained candidate logs')
        return {'log': str(log), 'elapsed_seconds': round(time.monotonic() - started, 3)}

    environment = os.environ.copy()
    environment.update(config['build_environment'])
    environment['CARGO_BUILD_JOBS'] = '2'
    try:
        captured = source_map()
        source = {**captured, 'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  'started_at': now(), 'event_protocol': args.sync_version, 'sync_protocol': args.sync_version,
                  'physical_database_schema': 1, 'teacher_prompt_version': args.teacher_prompt,
                  'dialogue_prompt_version': args.dialogue_prompt, 'reference_web_build': args.web_build,
                  'content_revision': config['pack']['revision'], 'lesson_count': len(config['pack']['lessons']),
                  'ca_sha256': config['ca_sha256'], 'build_environment_sha256': config['environment_sha256'],
                  'server_prompt_source_sha256': captured['source_sha256']['crates/rufren-tutor/src/teacher.rs'],
                  'dialogue_prompt_source_sha256': captured['source_sha256']['crates/rufren-tutor/src/dialogue.rs'],
                  'prompt_binding': 'Expected server policy; server-feature code is absent from the client build',
                  'preserved_active': {'ios_receipt': config['active'], 'macos_receipt': config['mac'],
                                       'macos_receipt_path': config['mac_receipt'], 'alias_sha256': config['aliases']}}
        if config['reader']:
            source['reader_content'] = {key: config['reader'][key] for key in ['id', 'version']}
            source['reader_content']['document_count'] = len(config['reader']['documents'])
        write(source_path, source)
        common = {'data_protocol': args.sync_version, 'event_version': args.sync_version,
                  'physical_database_schema': 1, 'teacher_prompt_version': args.teacher_prompt,
                  'dialogue_prompt_version': args.dialogue_prompt, 'teacher_prompt_binding': 'expected-server-prompt-not-client-embedded',
                  'content_lineage': source['content_revision'], 'lesson_count': source['lesson_count'],
                  'reference_web_build': args.web_build, 'source_evidence': str(source_path)}
        ios_build = run('ios-build', ['sh', 'scripts/platform/build-native.sh', 'ios-device'])
        ios = find_app('ios')
        ios_check = run('ios-check', ['python3', 'scripts/platform/check-apple-bundle.py', 'ios', str(ios)])
        macho = run('ios-macho', ['/usr/bin/vtool', '-show-build', str(ios / 'rufren-app')])
        macho_text = Path(macho['log']).read_text()
        if 'platform IOS' not in macho_text or 'minos 15.0' not in macho_text:
            raise ValueError('Unexpected physical-iOS deployment target')
        frozen_ios = folder / 'Rufren.app'
        shutil.copytree(ios, frozen_ios, symlinks=True)
        ios_manifest = {**common, 'app': str(frozen_ios), 'rust_executable_sha256': digest(frozen_ios / 'rufren-app'),
                        'source_build_log': ios_build['log'], 'build_elapsed_seconds': ios_build['elapsed_seconds'],
                        'bundle_check': ios_check, 'macho_evidence': macho['log'], 'ca_sha256': config['ca_sha256'],
                        'built_at': now(), 'device_installation': 'not attempted'}
        write(folder / 'ios.json', ios_manifest)
        mac_build = run('macos-build', ['sh', 'scripts/platform/build-native.sh', 'macos'])
        mac = find_app('macos')
        mac_sign = run('macos-sign', ['sh', 'scripts/platform/sign-macos-local.sh', str(mac)])
        frozen_mac = folder / 'Rufren-macos.app'
        shutil.copytree(mac, frozen_mac, symlinks=True)
        mac_check = run('macos-check', ['python3', 'scripts/platform/check-apple-bundle.py', 'macos', str(frozen_mac), '--signed'])
        mac_manifest = {**common, 'app': str(frozen_mac),
                        'rust_executable_sha256': digest(frozen_mac / 'Contents/MacOS/rufren-app'),
                        'source_build_log': mac_build['log'], 'build_elapsed_seconds': mac_build['elapsed_seconds'],
                        'signature': {'kind': 'ad-hoc', 'sign_evidence': mac_sign, 'frozen_verification': mac_check},
                        'built_at': now(), 'native_ui_regression': 'not exercised'}
        write(folder / 'macos.json', mac_manifest)
        assert_equal_maps(captured['source_sha256'], source_map()['source_sha256'], 'Source inputs')
        assert_equal_maps(config['aliases'], alias_hashes(), 'Artifact aliases')
        if digest(TASK / 'build-environment.json') != config['environment_sha256'] or digest(config['ca']) != config['ca_sha256'] or digest(config['web_path']) != config['web_sha256']:
            raise ValueError('Build configuration, CA or reference web changed')
        verify_executable(config['active'])
        verify_executable(config['mac'], True)
        source.update(completed_at=now(), source_hashes_unchanged=True, ios_build=ios_build, macos_build=mac_build)
        write(source_path, source)
        preflight.artifact(folder / 'ios.json', args.sync_version, args.teacher_prompt, args.dialogue_prompt)
        verify_executable(mac_manifest, True)
        receipt.update(stage='staged', completed_at=now(), source_inputs=len(captured['source_sha256']),
                       source_receipt=str(source_path), source_receipt_sha256=digest(source_path),
                       ios_manifest=str(folder / 'ios.json'), ios_manifest_sha256=digest(folder / 'ios.json'),
                       macos_manifest=str(folder / 'macos.json'), macos_manifest_sha256=digest(folder / 'macos.json'),
                       current_artifacts_and_aliases_preserved=True)
        write(receipt_path, receipt)
        return receipt
    except BaseException as error:
        receipt.update(stage='failed-retained', error_type=type(error).__name__,
                       error='Candidate retained for inspection; no active alias was written')
        write(receipt_path, receipt)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('name')
    parser.add_argument('--expected-sync-version', dest='sync_version', required=True, type=int)
    parser.add_argument('--expected-teacher-prompt', dest='teacher_prompt', required=True)
    parser.add_argument('--expected-dialogue-prompt', dest='dialogue_prompt', required=True)
    parser.add_argument('--web-build', required=True)
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    try:
        config = configuration(args)
        if args.check_only:
            print(json.dumps({'valid': True, 'build_started': False, 'source_inputs': len(source_map()['source_sha256']),
                              'active_protocol': config['active']['data_protocol'], 'candidate_protocol': args.sync_version}))
        else:
            print(json.dumps(stage(args, config)))
    except (ValueError, RuntimeError, KeyError, OSError, json.JSONDecodeError):
        raise SystemExit('Native staging refused or failed; inspect retained task-local candidate evidence. No active alias was written.') from None


if __name__ == '__main__':
    main()
