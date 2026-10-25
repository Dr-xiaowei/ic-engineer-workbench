"""Transfer previously built/scanned Windows files to an existing release draft."""
import hashlib
import io
import json
import os
from pathlib import PurePosixPath
import time
import urllib.parse
import urllib.request
import zipfile

REPO = 'Dr-xiaowei/ic-engineer-workbench'
BASE = 'https://api.github.com/repos/' + REPO
RUN = int(os.environ['SOURCE_RUN'])
RELEASE = int(os.environ['RELEASE_ID'])
SOURCE = os.environ['SOURCE_COMMIT']
VERSION = os.environ['RELEASE_VERSION']
TOKEN = os.environ['GH_TOKEN']

class Redirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, url):
        redirected = super().redirect_request(req, fp, code, msg, headers, url)
        if redirected is not None:
            redirected.remove_header('Authorization')
        return redirected

opener = urllib.request.build_opener(Redirect)

def request(path, body=None, url=None):
    target = url or BASE + path
    assert target.startswith(BASE + '/') or target.startswith('https://uploads.github.com/repos/' + REPO + '/releases/')
    headers = {'Authorization': 'Bearer ' + TOKEN, 'Accept': 'application/vnd.github+json', 'User-Agent': 'ICWB-release-transfer'}
    if body is not None:
        headers['Content-Type'] = 'application/octet-stream'
    return opener.open(urllib.request.Request(target, data=body, headers=headers), timeout=90)

def api(path, body=None, url=None):
    with request(path, body, url) as response:
        return json.load(response)

for attempt in range(40):
    run = api('/actions/runs/' + str(RUN))
    assert run['head_sha'] == SOURCE, 'Unexpected build source'
    if run['status'] == 'completed':
        assert run['conclusion'] == 'success', 'Build/security gates did not pass'
        break
    if attempt == 39:
        raise RuntimeError('Build has not completed; no files published')
    time.sleep(15)

artifacts = api('/actions/runs/' + str(RUN) + '/artifacts')['artifacts']
artifact, = [a for a in artifacts if a['name'] == f'windows-v{VERSION}-{RUN}']
assert not artifact['expired'] and artifact['size_in_bytes'] <= 160 * 1024 * 1024
with request('/actions/artifacts/' + str(artifact['id']) + '/zip') as response:
    archive = response.read(160 * 1024 * 1024 + 1)
assert len(archive) <= 160 * 1024 * 1024
assert 'sha256:' + hashlib.sha256(archive).hexdigest() == artifact['digest'], 'Build artifact digest mismatch'

names = {f'normal/ic-workbench-v{VERSION}-windows-x64-setup.exe', f'demo/ic-workbench-demo-v{VERSION}-windows-x64-setup.exe', 'manifest.json', 'SHA256SUMS.txt'}
with zipfile.ZipFile(io.BytesIO(archive)) as bundle:
    files = [info for info in bundle.infolist() if not info.is_dir()]
    assert len(files) == len(names) and {f.filename for f in files} == names
    assert sum(f.file_size for f in files) < 160 * 1024 * 1024
    assert all((f.external_attr >> 16) & 0o170000 != 0o120000 for f in files)
    manifest = json.loads(bundle.read('manifest.json'))
    assert manifest['version'] == VERSION and manifest['channel'] == 'release'
    assert manifest['status'] == 'built-for-authorized-release' and manifest['architecture'] == 'x64'
    assert len(manifest['files']) == 2
    release = api('/releases/' + str(RELEASE))
    assert release['draft'] and release['tag_name'] == 'v' + VERSION
    assert release['target_commitish'] == SOURCE
    for item in manifest['files']:
        assert item['path'] in names and item['path'].endswith('.exe')
        data = bundle.read(item['path'])
        digest = hashlib.sha256(data).hexdigest()
        assert len(data) == item['bytes'] and digest == item['sha256']
        name = PurePosixPath(item['path']).name
        existing = [a for a in release['assets'] if a['name'] == name]
        if existing:
            asset, = existing
        else:
            url = f'https://uploads.github.com/repos/{REPO}/releases/{RELEASE}/assets?' + urllib.parse.urlencode({'name': name})
            asset = api('', data, url)
        assert asset['state'] == 'uploaded' and asset['size'] == len(data)
        assert asset['digest'] == 'sha256:' + digest
        print(json.dumps({'name': name, 'bytes': len(data), 'sha256': digest}), flush=True)
print('Existing scanned Windows distributions transferred; no build or functional tests executed.')
