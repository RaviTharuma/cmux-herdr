#!/usr/bin/env python3
"""Offline execution of plist argv and release workflow shell (stdlib only)."""
import os
from pathlib import Path
import plistlib
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def workflow_step(name):
    text = (ROOT / '.github/workflows/release.yml').read_text()
    section = text.split('- name: ' + name + '\n', 1)[1]
    script = section.split('        run: |\n', 1)[1]
    lines = []
    for line in script.splitlines():
        if line and not line.startswith('          '):
            break
        lines.append(line[10:])
    return '\n'.join(lines)


class Packaging(unittest.TestCase):
    def test_launchagent_requires_explicit_binding_before_execution(self):
        plist = plistlib.loads((ROOT / 'scripts/com.cmux-herdr.watch.plist').read_bytes())
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            binary = home / '.local/bin/cmux-herdr'
            binary.parent.mkdir(parents=True)
            binary.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$HOME/invoked"\n')
            binary.chmod(0o755)
            env = {'HOME': tmp, 'PATH': '/usr/bin:/bin'}
            args = plist['ProgramArguments']
            result = subprocess.run(args, env=env, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((home / 'invoked').exists())
            socket = home / 'herdr.sock'
            socket.touch()
            env.update(CMUX_WORKSPACE_ID='workspace:explicit', CMUX_SURFACE_ID='surface:explicit',
                       HERDR_SOCKET_PATH=str(socket))
            subprocess.run(args, env=env, check=True)
            self.assertEqual((home / 'invoked').read_text().splitlines(),
                             ['watch', '--interval', '3', '--workspace', 'workspace:explicit'])

    def release_fixture(self, mode):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        dist = root / 'dist'
        dist.mkdir()
        targets = ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu',
                   'x86_64-apple-darwin', 'aarch64-apple-darwin']
        import hashlib
        sums = []
        for target in targets:
            asset = 'cmux-herdr-0.7.0-' + target
            (dist / asset).write_bytes(b'offline artifact')
            sums.append(hashlib.sha256(b'offline artifact').hexdigest() + '  ' + asset)
        (dist / 'SHA256SUMS').write_text('\n'.join(sums) + '\n')
        fake = root / 'gh'
        fake.write_text('''#!/usr/bin/env python3
import os, pathlib, shutil, sys
root = pathlib.Path(os.environ['FIXTURE'])
a = sys.argv[1:]
with (root / 'calls').open('a') as f: f.write(' '.join(a) + '\\n')
if a[:2] == ['release', 'create']:
    if os.environ['MODE'] == 'existing': sys.exit(1)
    assert '--draft' in a and '--verify-tag' in a
    (root / 'draft').mkdir()
    for p in a:
        if p.startswith('dist/'): shutil.copy(p, root / 'draft')
elif a[:2] == ['release', 'download']:
    out = pathlib.Path(a[a.index('--dir') + 1]); out.mkdir(exist_ok=True)
    for p in (root / 'draft').iterdir(): shutil.copy(p, out)
    if os.environ['MODE'] == 'corrupt': (out / 'SHA256SUMS').write_text('corrupt')
elif a[:2] == ['release', 'edit']:
    assert '--draft=false' in a
    (root / 'published').touch()
else: sys.exit(99)
''')
        fake.chmod(0o755)
        env = dict(os.environ, PATH=str(root) + ':' + os.environ['PATH'], FIXTURE=str(root),
                   MODE=mode, GITHUB_REF_NAME='v0.7.0')
        return root, env

    def test_release_publishes_only_complete_verified_draft(self):
        for mode in ['ok', 'existing', 'corrupt', 'missing']:
            with self.subTest(mode=mode):
                root, env = self.release_fixture(mode)
                if mode == 'missing':
                    (root / 'dist/cmux-herdr-0.7.0-aarch64-apple-darwin').unlink()
                result = subprocess.run(['bash', '-c', workflow_step('Publish immutable release')],
                                        cwd=root, env=env, capture_output=True)
                self.assertEqual(result.returncode == 0, mode == 'ok', result.stderr)
                self.assertEqual((root / 'published').exists(), mode == 'ok')
                calls = (root / 'calls').read_text() if (root / 'calls').exists() else ''
                self.assertNotIn('--clobber', calls)
                if mode == 'existing': self.assertEqual(len(calls.splitlines()), 1)


if __name__ == '__main__':
    unittest.main()
