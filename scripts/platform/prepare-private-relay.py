#!/usr/bin/env python3
"""Prepare task-private CA/TLS/bootstrap files; print no secrets or device identifiers."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def write(path, value):
    payload = json.dumps(value, indent=2) + '\n' if not isinstance(value, str) else value
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w') as output:
        output.write(payload)


def run(arguments):
    result = subprocess.run(arguments, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    if result.returncode:
        raise RuntimeError(f'{arguments[0]} failed; private task files were retained for inspection')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--lan-ip', required=True)
    parser.add_argument('--tail-ip')
    parser.add_argument('--hostname', required=True)
    parser.add_argument('--tail-hostname')
    parser.add_argument('--port', type=int, default=9443)
    parser.add_argument('--upstream', default='127.0.0.1:8787')
    args = parser.parse_args()
    import ipaddress
    ips = [args.lan_ip] + ([args.tail_ip] if args.tail_ip else [])
    for value in ips:
        address = ipaddress.ip_address(value)
        if address.is_unspecified or address.is_loopback or address.is_multicast or (not address.is_private and not address in ipaddress.ip_network('100.64.0.0/10')):
            raise ValueError('TLS listeners must use private LAN/overlay addresses')
    names = list(dict.fromkeys([args.hostname, args.hostname.removesuffix('.local')] + ([args.tail_hostname] if args.tail_hostname else [])))
    if any(not name or any(character not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-' for character in name) for name in names):
        raise ValueError('Invalid TLS DNS name')
    folder = ROOT / '.local/ipad-delivery'
    folder.mkdir(parents=True, exist_ok=True)
    folder.chmod(0o700)
    manifest = folder / 'private-relay.json'
    if manifest.exists():
        existing = json.loads(manifest.read_text())
        if existing['ips'] != ips or existing['names'] != names or existing['port'] != args.port:
            raise ValueError('Existing task TLS configuration differs; refusing to overwrite its identity')
        print('Existing private TLS/bootstrap configuration retained.')
        return
    ca_key, ca = folder / 'ca-key.pem', folder / 'ca.pem'
    leaf_key, leaf, request = folder / 'relay-key.pem', folder / 'relay.pem', folder / 'relay.csr'
    if any(path.exists() for path in [ca_key, ca, leaf_key, leaf, request]):
        raise ValueError('Partial TLS identity already exists; inspect it before choosing a new identity')
    previous = os.umask(0o077)
    try:
        run(['openssl', 'ecparam', '-name', 'prime256v1', '-genkey', '-noout', '-out', str(ca_key)])
        run(['openssl', 'req', '-new', '-x509', '-sha256', '-days', '3650', '-key', str(ca_key), '-out', str(ca), '-subj', '/CN=Rufren private development CA', '-addext', 'basicConstraints=critical,CA:TRUE', '-addext', 'keyUsage=critical,keyCertSign,cRLSign'])
        run(['openssl', 'ecparam', '-name', 'prime256v1', '-genkey', '-noout', '-out', str(leaf_key)])
        run(['openssl', 'req', '-new', '-sha256', '-key', str(leaf_key), '-out', str(request), '-subj', '/CN=' + args.hostname])
        extension = folder / 'relay-extensions.cnf'
        write(extension, 'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid,issuer\nsubjectAltName=' + ','.join(['DNS:' + name for name in names] + ['IP:' + value for value in ips]) + '\n')
        run(['openssl', 'x509', '-req', '-sha256', '-days', '365', '-in', str(request), '-CA', str(ca), '-CAkey', str(ca_key), '-set_serial', '0x' + secrets.token_hex(16), '-out', str(leaf), '-extfile', str(extension)])
        run(['openssl', 'verify', '-x509_strict', '-CAfile', str(ca), str(leaf)])
        for path in [ca_key, leaf_key]:
            path.chmod(0o600)
        origins = [f'https://{host}:{args.port}' for host in names + ips]
        endpoint = origins[0]
        token = secrets.token_urlsafe(32)
        write(folder / 'bootstrap.json', {'RUFREN_RELAY_URL': endpoint, 'RUFREN_SYNC_TOKEN': token})
        write(folder / 'relay-environment.json', {'RUFREN_RELAY_BIND': args.upstream, 'RUFREN_RELAY_DB': str(folder / 'library.sqlite'), 'RUFREN_RELAY_ASSETS': str(folder / 'assets'), 'RUFREN_SYNC_TOKEN': token, 'RUFREN_ALLOWED_ORIGINS': 'http://127.0.0.1:8080,http://localhost:8080,http://127.0.0.1:8091,http://localhost:8091'})
        write(folder / 'build-environment.json', {'RUFREN_DEV_CA_CERT': str(ca), 'RUFREN_DEV_RELAY_ORIGINS': ','.join(origins)})
        config = folder / 'Caddyfile'
        write(config, '{\n storage file_system {\n  root ' + str(folder / 'caddy-storage') + '\n }\n admin off\n auto_https off\n persist_config off\n}\n' + ', '.join(origins) + ' {\n bind ' + ' '.join(ips) + '\n tls ' + str(leaf) + ' ' + str(leaf_key) + '\n reverse_proxy ' + args.upstream + '\n header Cache-Control no-store\n}\n')
        run(['caddy', 'validate', '--config', str(config), '--adapter', 'caddyfile'])
        write(manifest, {'version': 1, 'endpoint': endpoint, 'ips': ips, 'names': names, 'port': args.port, 'ca_sha256': hashlib.sha256(ca.read_bytes()).hexdigest(), 'ca_certificate': str(ca), 'config': str(config)})
        print('Prepared and validated private TLS/bootstrap files; endpoint ' + endpoint)
    finally:
        os.umask(previous)


if __name__ == '__main__':
    main()
