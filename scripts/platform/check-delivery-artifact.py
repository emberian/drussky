#!/usr/bin/env python3
"""Read-only frozen iOS artifact + optional private-TLS relay compatibility preflight.

No signing, device, account or service mutations. Bootstrap credentials are never
sent: /health is public. --offline validates only the frozen artifact, not rollout.
"""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import re
import ssl
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]


def artifact(path, expected_sync_version, expected_teacher_prompt=None, expected_dialogue_prompt=None):
    manifest = json.loads(path.read_text())
    if type(expected_sync_version) is not int or expected_sync_version < 1:
        raise ValueError('Expected sync version must be a positive integer')
    if type(manifest.get('data_protocol')) is not int or manifest['data_protocol'] != expected_sync_version:
        raise ValueError('Frozen artifact data protocol differs from the selected rollout')
    if expected_teacher_prompt and manifest.get('teacher_prompt_version') != expected_teacher_prompt:
        raise ValueError('Frozen artifact teacher prompt differs from the selected rollout')
    if expected_dialogue_prompt and manifest.get('dialogue_prompt_version') != expected_dialogue_prompt:
        raise ValueError('Frozen artifact dialogue prompt differs from the selected rollout')
    if type(manifest.get('lesson_count')) is not int or manifest['lesson_count'] < 1:
        raise ValueError('Frozen artifact lesson count is invalid')
    evidence = json.loads(Path(manifest['source_evidence']).read_text())
    if evidence.get('source_hashes_unchanged') is not True:
        raise ValueError('Serialized frozen build has not recorded completed stable source verification')
    if evidence.get('sync_protocol', evidence.get('event_protocol')) != expected_sync_version:
        raise ValueError('Build-source evidence and frozen artifact protocols differ')
    if manifest.get('teacher_prompt_version') and evidence.get('teacher_prompt_version') != manifest['teacher_prompt_version']:
        raise ValueError('Build-source evidence and frozen artifact teacher prompts differ')
    if manifest.get('dialogue_prompt_version') and evidence.get('dialogue_prompt_version') != manifest['dialogue_prompt_version']:
        raise ValueError('Build-source evidence and frozen artifact dialogue prompts differ')
    if evidence.get('content_revision') != manifest.get('content_lineage') or evidence.get('lesson_count') != manifest['lesson_count']:
        raise ValueError('Build-source evidence and frozen artifact content differ')
    if evidence.get('ca_sha256') != manifest.get('ca_sha256'):
        raise ValueError('Build-source evidence and frozen artifact CA differ')
    if 'event_version' in manifest and evidence.get('event_protocol') != manifest['event_version']:
        raise ValueError('Build-source evidence and frozen artifact event versions differ')
    app = Path(manifest['app']).resolve(strict=True)
    if app.is_relative_to(ROOT / 'target'):
        raise ValueError('Delivery must select a frozen archive, not mutable target output')
    info = plistlib.loads((app / 'Info.plist').read_bytes())
    if info.get('CFBundleIdentifier') != 'town.lunar.rufren' or info.get('CFBundleSupportedPlatforms') != ['iPhoneOS']:
        raise ValueError('Frozen bundle is not the Rufren physical-iOS app')
    executable = (app / info['CFBundleExecutable']).resolve(strict=True)
    if not executable.is_relative_to(app) or not executable.is_file():
        raise ValueError('Frozen executable is outside its app')
    expected_hash = manifest.get('rust_executable_sha256', '')
    if not re.fullmatch('[0-9a-f]{64}', expected_hash):
        raise ValueError('Frozen executable SHA-256 is invalid')
    with executable.open('rb') as source:
        actual_hash = hashlib.file_digest(source, 'sha256').hexdigest()
    if actual_hash != expected_hash:
        raise ValueError('Frozen executable differs from its receipt')
    return manifest


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, file, code, message, headers, new_url):
        raise ValueError('Private relay health must not redirect')


def relay_health(bootstrap, ca, expected_sync_version):
    endpoint = json.loads(bootstrap.read_text()).get('RUFREN_RELAY_URL', '')
    url = urllib.parse.urlsplit(endpoint)
    if url.scheme != 'https' or not url.hostname or url.username or url.password or url.query or url.fragment or url.path not in ('', '/'):
        raise ValueError('Bootstrap must select a plain HTTPS relay origin')
    # Reject ambient proxy settings: inspect the exact configured private origin.
    opener = urllib.request.build_opener(
        urllib.request.ProxyHandler({}), NoRedirect(),
        urllib.request.HTTPSHandler(context=ssl.create_default_context(cafile=str(ca))))
    request = urllib.request.Request(endpoint.rstrip('/') + '/health', headers={'Accept': 'text/plain'}, method='GET')
    try:
        with opener.open(request, timeout=10) as response:
            if response.status != 200:
                raise ValueError('Private relay health did not return HTTP 200')
            body = response.read(128).decode('ascii').strip()
    except (urllib.error.URLError, TimeoutError, OSError, UnicodeError) as error:
        raise ValueError('Private relay TLS/health preflight failed; inspect its task-local service state') from error
    if body != 'rufren-relay/' + str(expected_sync_version):
        raise ValueError('Running relay sync version differs from the frozen artifact; do not provision/install')
    return expected_sync_version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest', type=Path)
    parser.add_argument('--expected-sync-version', type=int, required=True)
    parser.add_argument('--expected-teacher-prompt')
    parser.add_argument('--expected-dialogue-prompt')
    parser.add_argument('--offline', action='store_true', help='Static artifact validation only; never sufficient for installation')
    parser.add_argument('--bootstrap', type=Path)
    parser.add_argument('--ca-cert', type=Path)
    args = parser.parse_args()
    if not args.offline and (not args.bootstrap or not args.ca_cert):
        parser.error('Live preflight requires --bootstrap and --ca-cert (or explicitly use --offline)')
    try:
        manifest = artifact(args.manifest, args.expected_sync_version, args.expected_teacher_prompt, args.expected_dialogue_prompt)
        if not args.offline:
            with args.ca_cert.open('rb') as certificate:
                if hashlib.file_digest(certificate, 'sha256').hexdigest() != manifest['ca_sha256']:
                    raise ValueError('Preflight CA differs from the CA recorded in the frozen app build')
            relay_health(args.bootstrap, args.ca_cert, manifest['data_protocol'])
        print(json.dumps({'artifact_verified': True, 'relay_verified': not args.offline,
                          'data_protocol': manifest['data_protocol'],
                          'teacher_prompt_version': manifest.get('teacher_prompt_version'),
                          'dialogue_prompt_version': manifest.get('dialogue_prompt_version'),
                          'content_lineage': manifest.get('content_lineage'),
                          'lesson_count': manifest['lesson_count'],
                          'rust_executable_sha256': manifest['rust_executable_sha256']}))
    except json.JSONDecodeError:
        raise SystemExit('Frozen artifact/relay preflight refused: malformed task JSON')
    except ValueError as error:
        # These are fixed validation messages, never bootstrap values or headers.
        raise SystemExit('Frozen artifact/relay preflight refused: ' + str(error))
    except (KeyError, FileNotFoundError, plistlib.InvalidFileException):
        raise SystemExit('Frozen artifact/relay preflight refused: incomplete task receipt or bundle metadata')


if __name__ == '__main__':
    main()
