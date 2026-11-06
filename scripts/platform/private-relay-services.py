#!/usr/bin/env python3
"""Manage only task-private relay/TLS processes; retain executable provenance, never secrets."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import socket
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / '.local/ipad-delivery'


def digest(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def atomic_json(path, value):
    temporary = path.with_name(path.name + '.pending-' + uuid.uuid4().hex)
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w') as output:
        output.write(json.dumps(value, indent=2) + '\n')
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary, path)


def process_command(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'stat=', '-o', 'command='], capture_output=True, text=True)
    if result.returncode or not result.stdout.strip():
        return ''
    state, _, command = result.stdout.strip().partition(' ')
    return '' if state.startswith('Z') else command.strip()


def process_cwd(pid):
    result = subprocess.run(['lsof', '-a', '-p', str(pid), '-d', 'cwd', '-Fn'], capture_output=True, text=True)
    return next((line[1:] for line in result.stdout.splitlines() if line.startswith('n')), None)


def expected(service):
    """Legacy ownership fallback, not proof of the bytes of a running image."""
    return str(ROOT / 'target/debug/rufren-relay') if service == 'relay' else str(Path(shutil.which('caddy')).resolve())


def receipt(service):
    path = FOLDER / (service + '-process.json')
    if not path.exists():
        return None
    value = json.loads(path.read_text())
    if value.get('version') != 1 or value.get('service') != service or value.get('cwd') != str(FOLDER):
        raise RuntimeError('Private process receipt has an unexpected format or working directory')
    if type(value.get('pid')) is not int or not Path(value.get('executable', '')).is_absolute():
        raise RuntimeError('Private process receipt has invalid ownership metadata')
    return value


def matches_command(command, executable):
    try:
        arguments = shlex.split(command)
        if not arguments:
            return False
        actual = Path(arguments[0])
        if not actual.is_absolute():
            actual = FOLDER / actual
        return str(actual.resolve()) == str(Path(executable).resolve())
    except ValueError:
        return False


def owned_pid(service):
    marker = FOLDER / (service + '.pid')
    if not marker.exists():
        return None
    pid = int(marker.read_text())
    if pid <= 1:
        raise RuntimeError('Invalid private service PID marker; refusing to act')
    command = process_command(pid)
    if not command:
        marker.unlink()  # This manager's stale marker only, after proving no live process.
        return None
    saved = receipt(service)
    executable = saved['executable'] if saved and saved['pid'] == pid else expected(service)
    if not matches_command(command, executable):
        raise RuntimeError('PID marker does not identify the selected task executable; refusing to act')
    if process_cwd(pid) != str(FOLDER):
        raise RuntimeError('Service working directory differs from this task; refusing to act')
    return pid


def service_status(service):
    pid = owned_pid(service)
    if not pid:
        return {'service': service, 'running': False}
    saved = receipt(service)
    saved = saved if saved and saved['pid'] == pid else None
    executable = saved['executable'] if saved else expected(service)
    return {'service': service, 'running': True, 'pid': pid, 'executable': executable,
            'ownership': 'recorded' if saved else 'legacy-command-and-cwd',
            'recorded_sha256': saved.get('executable_sha256') if saved else None,
            'on_disk_matches_receipt': (Path(executable).is_file() and digest(executable) == saved['executable_sha256']) if saved else None}


def start(service, provider_environment=None, relay_executable=None, relay_sha256=None):
    if owned_pid(service):
        print(service + ' is already running; no executable was replaced.')
        return
    previous = receipt(service)
    if service == 'relay' and not relay_executable and not previous:
        raise RuntimeError('A new relay start requires an explicit accepted immutable executable')
    executable = str(Path(relay_executable).resolve(strict=True)) if service == 'relay' and relay_executable else (previous['executable'] if previous else expected(service))
    if service == 'relay' and Path(executable).is_relative_to(ROOT / 'target'):
        raise RuntimeError('Explicit relay selection must use an immutable accepted copy outside target/')
    if not Path(executable).is_file() or not os.access(executable, os.X_OK):
        raise RuntimeError('Selected private service executable is unavailable')
    executable_hash = digest(executable)
    if relay_sha256 and service == 'relay' and executable_hash != relay_sha256:
        raise RuntimeError('Selected relay executable does not match its accepted SHA-256')
    if previous and not relay_executable and executable_hash != previous['executable_sha256']:
        raise RuntimeError('Recorded executable changed on disk; select an accepted immutable relay explicitly')
    environment = os.environ.copy()
    provider_path = None
    if service == 'relay':
        if provider_environment:
            provider_path = Path(provider_environment).resolve(strict=True)
        elif previous and previous.get('provider_environment'):
            provider_path = Path(previous['provider_environment'])
            if digest(provider_path) != previous['provider_environment_sha256']:
                raise RuntimeError('Recorded provider configuration changed; select its accepted file explicitly')
        if provider_path:
            environment.update(json.loads(provider_path.read_text()))
        environment.update(json.loads((FOLDER / 'relay-environment.json').read_text()))
        command = [executable]
        host, port = environment.get('RUFREN_RELAY_BIND', '127.0.0.1:8787').rsplit(':', 1)
        address = (host.strip('[]'), int(port))
    else:
        environment['XDG_DATA_HOME'] = str(FOLDER / 'caddy-data')
        environment['XDG_CONFIG_HOME'] = str(FOLDER / 'caddy-config')
        command = [executable, 'run', '--config', str(FOLDER / 'Caddyfile'), '--adapter', 'caddyfile']
        manifest = json.loads((FOLDER / 'private-relay.json').read_text())
        address = (manifest['ips'][0], manifest['port'])
    descriptor = os.open(FOLDER / (service + '.log'), os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    with os.fdopen(descriptor, 'ab') as output:
        process = subprocess.Popen(command, cwd=FOLDER, env=environment, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
    (FOLDER / (service + '.pid')).write_text(str(process.pid))
    (FOLDER / (service + '.pid')).chmod(0o600)
    atomic_json(FOLDER / (service + '-process.json'), {
        'version': 1, 'service': service, 'pid': process.pid, 'cwd': str(FOLDER),
        'executable': executable, 'executable_sha256': executable_hash,
        'selection': 'explicit-immutable' if relay_executable and service == 'relay' else 'recorded-or-legacy-default',
        'provider_environment': str(provider_path) if provider_path else None,
        'provider_environment_sha256': digest(provider_path) if provider_path else None,
        'started_at': datetime.now(timezone.utc).isoformat()})
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError(service + ' exited during startup; inspect its retained private log/receipt')
        try:
            with socket.create_connection(address, timeout=.2):
                print(service + ' started; selected executable/hash and private PID/log recorded.')
                return
        except OSError:
            time.sleep(.1)
    raise RuntimeError(service + ' did not open its listener; private PID/log/receipt retained')


def stop(service):
    pid = owned_pid(service)
    if not pid:
        print(service + ' is not running.')
        return
    # Revalidate both exact executable and cwd immediately before signaling.
    if owned_pid(service) != pid:
        raise RuntimeError('Service PID changed; refusing to stop it')
    os.kill(pid, signal.SIGINT)
    for _ in range(100):
        if not process_command(pid):
            (FOLDER / (service + '.pid')).unlink(missing_ok=True)
            print(service + ' stopped; executable provenance receipt retained.')
            return
        time.sleep(.1)
    raise RuntimeError(service + ' is still shutting down; its PID/log were retained')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['start', 'status', 'stop'])
    parser.add_argument('service', choices=['relay', 'tls-proxy', 'all'])
    parser.add_argument('--provider-environment', help='Private accepted provider JSON; values are never printed')
    parser.add_argument('--relay-executable', type=Path, help='Immutable accepted executable outside target/')
    parser.add_argument('--relay-sha256', help='Expected SHA-256 of the accepted relay executable')
    args = parser.parse_args()
    if args.relay_sha256 and not args.relay_executable:
        parser.error('--relay-sha256 requires --relay-executable')
    if args.action != 'start' and (args.relay_executable or args.relay_sha256 or args.provider_environment):
        parser.error('Executable/provider selection applies only to start')
    for service in ['relay', 'tls-proxy'] if args.service == 'all' else [args.service]:
        if args.action == 'start':
            start(service, args.provider_environment, args.relay_executable, args.relay_sha256)
        elif args.action == 'stop':
            stop(service)
        else:
            state = service_status(service)
            print(service + ': ' + ('running' if state['running'] else 'stopped'))


if __name__ == '__main__':
    main()
