#!/usr/bin/env python3
"""Start/status/stop only Rufren's task-private relay and TLS proxy; never print secrets."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / '.local/ipad-delivery'


def process_command(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'command='], capture_output=True, text=True)
    return result.stdout.strip() if result.returncode == 0 else ''


def expected(service):
    return str(ROOT / 'target/debug/rufren-relay') if service == 'relay' else str(Path(shutil.which('caddy')).resolve())


def owned_pid(service):
    path = FOLDER / (service + '.pid')
    if not path.exists():
        return None
    pid = int(path.read_text())
    command = process_command(pid)
    if not command:
        path.unlink()  # Only this manager's stale PID marker, after confirming no process exists.
        return None
    if expected(service) not in command:
        raise RuntimeError('PID marker no longer identifies this task service; refusing to act')
    result = subprocess.run(['lsof', '-a', '-p', str(pid), '-d', 'cwd', '-Fn'], capture_output=True, text=True)
    if 'n' + str(FOLDER) not in result.stdout.splitlines():
        raise RuntimeError('Service working directory does not match this task; refusing to act')
    return pid


def start(service, provider_environment):
    if owned_pid(service):
        print(service + ' is already running.')
        return
    environment = os.environ.copy()
    if service == 'relay':
        if provider_environment:
            environment.update(json.loads(Path(provider_environment).read_text()))
        environment.update(json.loads((FOLDER / 'relay-environment.json').read_text()))
        command = [expected(service)]
    else:
        environment['XDG_DATA_HOME'] = str(FOLDER / 'caddy-data')
        environment['XDG_CONFIG_HOME'] = str(FOLDER / 'caddy-config')
        command = [expected(service), 'run', '--config', str(FOLDER / 'Caddyfile'), '--adapter', 'caddyfile']
    descriptor = os.open(FOLDER / (service + '.log'), os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    with os.fdopen(descriptor, 'ab') as output:
        process = subprocess.Popen(command, cwd=FOLDER, env=environment, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
    (FOLDER / (service + '.pid')).write_text(str(process.pid))
    (FOLDER / (service + '.pid')).chmod(0o600)
    manifest = json.loads((FOLDER / 'private-relay.json').read_text())
    address = ('127.0.0.1', 8787) if service == 'relay' else (manifest['ips'][0], manifest['port'])
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError(service + ' exited during startup; inspect its private task log')
        try:
            with socket.create_connection(address, timeout=.2):
                print(service + ' started; private task PID/log recorded.')
                return
        except OSError:
            time.sleep(.1)
    raise RuntimeError(service + ' has not opened its task listener; inspect its private log')


def stop(service):
    pid = owned_pid(service)
    if not pid:
        print(service + ' is not running.')
        return
    # Re-read immediately before stopping; never kill by a command pattern.
    if expected(service) not in process_command(pid):
        raise RuntimeError('Service PID changed; refusing to stop it')
    os.kill(pid, signal.SIGINT)
    for _ in range(100):
        if not process_command(pid):
            (FOLDER / (service + '.pid')).unlink()
            print(service + ' stopped.')
            return
        time.sleep(.1)
    print(service + ' is still shutting down; its PID marker and log were retained.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['start', 'status', 'stop'])
    parser.add_argument('service', choices=['relay', 'tls-proxy', 'all'])
    parser.add_argument('--provider-environment', help='Private JSON environment prepared by the teacher/speech owner')
    args = parser.parse_args()
    services = ['relay', 'tls-proxy'] if args.service == 'all' else [args.service]
    for service in services:
        if args.action == 'start':
            start(service, args.provider_environment)
        elif args.action == 'stop':
            stop(service)
        else:
            print(service + ': ' + ('running' if owned_pid(service) else 'stopped'))


if __name__ == '__main__':
    main()
