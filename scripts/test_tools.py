#!/usr/bin/env python3
"""End-to-end tests using only synthetic content and loopback sockets."""
import json
import pathlib
import socket
import subprocess
import tempfile
import sys

root = pathlib.Path(__file__).resolve().parents[1]
binary = root / 'target' / 'debug' / ('empire-tools.exe' if sys.platform == 'win32' else 'empire-tools')

def run(*args, ok=True):
    result = subprocess.run([str(binary), *map(str,args)], capture_output=True, text=True, timeout=30)
    assert (result.returncode == 0) == ok, (args, result.stdout, result.stderr)
    return result.stdout

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
proc = subprocess.Popen([str(binary), 'relay', '127.0.0.1:0', '2'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
clients = []
try:
    line = proc.stdout.readline()
    assert 'listening on' in line, line
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
    proc.terminate()
    proc.wait(timeout=5)
