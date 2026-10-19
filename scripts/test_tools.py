#!/usr/bin/env python3
"""End-to-end tests using only synthetic content and loopback sockets."""
import json
import pathlib
import queue
import socket
import subprocess
import tempfile
import sys
import threading
from contextlib import contextmanager

root = pathlib.Path(__file__).resolve().parents[1]
binary = root / 'target' / 'debug' / ('empire-tools.exe' if sys.platform == 'win32' else 'empire-tools')

def run(*args, ok=True):
    result = subprocess.run([str(binary), *map(str,args)], capture_output=True, text=True, timeout=30)
    assert (result.returncode == 0) == ok, (args, result.stdout, result.stderr)
    return result.stdout


def stop_process(proc, timeout=5):
    """Bound shutdown and reap a relay even if it ignores termination."""
    if proc.poll() is None:
        proc.terminate()
    try:
        proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=timeout)


@contextmanager
def relay_process(command, startup_timeout=10, shutdown_timeout=5):
    # A file avoids blocking the child on a full stderr pipe during startup.
    with tempfile.TemporaryFile(mode='w+t', encoding='utf-8', errors='replace') as errors:
        proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=errors, text=True)
        lines = queue.Queue()

        def read_startup():
            try:
                lines.put(proc.stdout.readline())
            except Exception as error:
                lines.put(error)

        # Windows pipes cannot use select(); a daemon reader works on all CI OSes.
        reader = threading.Thread(target=read_startup, daemon=True)
        reader.start()
        failed = False
        try:
            try:
                line = lines.get(timeout=startup_timeout)
            except queue.Empty:
                raise TimeoutError(f'relay startup timed out after {startup_timeout}s') from None
            if isinstance(line, Exception):
                raise RuntimeError('could not read relay startup output') from line
            if 'listening on ' not in line:
                raise RuntimeError(f'relay did not report a listening address: {line!r}')
            yield proc, line
        except BaseException:
            failed = True
            raise
        finally:
            try:
                stop_process(proc, shutdown_timeout)
            except Exception as error:
                if not failed:
                    raise
                print(f'Relay cleanup failed: {error}', file=sys.stderr)
            finally:
                reader.join(timeout=shutdown_timeout)
                if not reader.is_alive():
                    proc.stdout.close()
                if failed:
                    errors.seek(0)
                    diagnostic = errors.read(8193)
                    if len(diagnostic) > 8192:
                        diagnostic = diagnostic[:8192] + '\n<stderr truncated>'
                    print(f'Relay exit code after cleanup: {proc.returncode}; stderr:\n'
                          f'{diagnostic or "<empty>"}', file=sys.stderr)


def main():
    with tempfile.TemporaryDirectory() as directory:
        temp = pathlib.Path(directory)
        content = temp / 'content'
        content.mkdir()
        (content / 'synthetic.bin').write_bytes(b'Independent synthetic content. Not a DE fixture.')
        manifest = temp / 'manifest.json'
        run('inventory-de', content, '123456', manifest)
        run('verify-pack', content, manifest)
        (content / 'synthetic.bin').write_bytes(b'Tampered')
        run('verify-pack', content, manifest, ok=False)
        doc = json.loads(manifest.read_text())
        doc['files'][0]['path'] = '../outside'
        manifest.write_text(json.dumps(doc))
        run('verify-pack', content, manifest, ok=False)
    print('PASS: source inventory, pack integrity, tamper and traversal rejection')

    identity = json.loads(run('identity'))
    with relay_process([str(binary), 'relay', '127.0.0.1:0', '2']) as (_, line):
        clients = []
        try:
            address = line.split('listening on ')[1].split(' ')[0]
            host, port = address.rsplit(':', 1)
            for player in range(2):
                sock = socket.create_connection((host, int(port)), timeout=5)
                sock.settimeout(5)
                f = sock.makefile('rwb')
                clients.append((sock,f))
                f.write((json.dumps({'Hello': {'protocol':1,'identity':identity,'player':player}})+'\n').encode());f.flush()
                assert 'Welcome' in json.loads(f.readline())
            for _,f in clients: assert json.loads(f.readline()) == 'Ready'
            # A connection must never submit another player's command.
            f = clients[0][1]
            f.write((json.dumps({'Submit':{'tick':0,'commands':[{'tick':0,'player':1,'sequence':0,'action':'Resign'}]}})+'\n').encode());f.flush()
            assert 'Error' in json.loads(f.readline())
            for tick in range(3):
                for _,f in reversed(clients):
                    f.write((json.dumps({'Submit':{'tick':tick,'commands':[]}})+'\n').encode());f.flush()
                for _,f in clients: assert json.loads(f.readline()) == {'Turn':{'tick':tick,'commands':[]}}
            for player,(_,f) in enumerate(clients):
                f.write((json.dumps({'Hash':{'tick':3,'hash':('a' if player==0 else 'b')*64}})+'\n').encode());f.flush()
            for _,f in clients: assert 'Desync' in json.loads(f.readline())
            print('PASS: real TCP handshake, two-client turn barrier, spoof rejection, desync broadcast')
        finally:
            for sock,f in clients:
                f.close();sock.close()


if __name__ == '__main__':
    main()
