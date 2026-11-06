"""保存GitHub实际API响应/作业日志；不发送消息、不触发手动工作流。"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

TASK = Path(__file__).resolve().parents[1]
SOURCE = TASK / 'source'
ROOT = SOURCE / 'docs/DevDocs/assets/20o-b2-a4/entry-20261106'
REPO = 'repos/Programming-Language-Xiao/Xiao'
COMMIT = subprocess.check_output(['cmd', '/c', 'git', 'rev-parse', 'HEAD'], cwd=SOURCE).decode().strip()
STAMP = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S')
ENV = os.environ.copy()
if '--git-proxy' in sys.argv:
    proxy = subprocess.check_output(['cmd', '/c', 'git', 'config', '--get', 'http.proxy'], cwd=SOURCE).decode().strip()
    ENV.update(HTTP_PROXY=proxy, HTTPS_PROXY=proxy)


def get(endpoint, name):
    path = ROOT / name
    assert not path.exists(), name
    result = subprocess.run(['gh', 'api', endpoint], cwd=SOURCE, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    path.write_bytes(result.stdout)
    if result.returncode:
        (ROOT / (name + '.stderr.log')).write_bytes(result.stderr)
        raise SystemExit('GitHub查询失败：' + name)
    return result.stdout


runs = json.loads(get(REPO + '/actions/runs?head_sha=' + COMMIT + '&per_page=30', 'ci-poll-' + STAMP + '.json'))['workflow_runs']
print(json.dumps([{'id': run['id'], 'name': run['name'], 'status': run['status'], 'conclusion': run['conclusion'],
                  'head_sha': run['head_sha']} for run in runs], ensure_ascii=False), flush=True)
if 'jobs' in sys.argv:
    for run in runs:
        jobs = json.loads(get(REPO + '/actions/runs/' + str(run['id']) + '/jobs?per_page=100',
                              'ci-jobs-' + str(run['id']) + '-' + STAMP + '.json'))['jobs']
        print(json.dumps([{'id': job['id'], 'name': job['name'], 'status': job['status'], 'conclusion': job['conclusion'],
                          'active_step': next((step['name'] for step in job.get('steps', []) if step['status'] == 'in_progress'), None)}
                         for job in jobs], ensure_ascii=False), flush=True)
if 'final' in sys.argv:
    expected = {'Xiao workspace gates': 'workspace', 'Xiao maintenance regression': 'maintenance'}
    chosen = {}
    for title, role in expected.items():
        candidates = [run for run in runs if run['name'] == title]
        assert len(candidates) == 1, (title, [run['name'] for run in runs])
        run = candidates[0]
        assert run['status'] == 'completed' and run['conclusion'] == 'success', title
        chosen[role] = run
    for role, run in chosen.items():
        get(REPO + '/actions/runs/' + str(run['id']), 'ci-' + role + '-run.json')
        jobs = json.loads(get(REPO + '/actions/runs/' + str(run['id']) + '/jobs?filter=all&per_page=100', 'ci-' + role + '-jobs.json'))['jobs']
        if role == 'maintenance':
            selected = [job for job in jobs if 'workspace default tests' in job['name']]
            assert selected
            job = max(selected, key=lambda item: (item.get('run_attempt', 1), item['id']))
            assert job['conclusion'] == 'success' and job['head_sha'] == COMMIT
            log = 'ci-kernel-linux.log'
            raw = get(REPO + '/actions/jobs/' + str(job['id']) + '/logs', log)
            (ROOT / 'ci-kernel-linux.json').write_text(json.dumps({'code_commit': COMMIT, 'run_id': run['id'],
                'job_id': job['id'], 'job_name': job['name'], 'run_attempt': job.get('run_attempt', 1), 'conclusion': job['conclusion'],
                'log': log, 'log_sha256': hashlib.sha256(raw).hexdigest(),
                'scope': 'Linux默认workspace的Runtime内核；不冒充A5语言或A6性能'}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
            for platform, label in [('windows', 'Windows'), ('linux', 'Linux'), ('macos', 'macOS')]:
                candidates = [job for job in jobs if job['name'] == 'Native differential regression (' + label + ')']
                assert candidates, platform
                job = max(candidates, key=lambda item: (item.get('run_attempt', 1), item['id']))
                assert job['conclusion'] == 'success' and job['head_sha'] == COMMIT, platform
                log = 'ci-native-' + platform + '.log'
                raw = get(REPO + '/actions/jobs/' + str(job['id']) + '/logs', log)
                (ROOT / ('ci-native-' + platform + '.json')).write_text(json.dumps({'code_commit': COMMIT,
                    'run_id': run['id'], 'run_attempt': job.get('run_attempt', 1), 'job_id': job['id'],
                    'job_name': job['name'], 'conclusion': job['conclusion'], 'log': log,
                    'log_sha256': hashlib.sha256(raw).hexdigest(),
                    'scope': '现有B1/A1原生回归，不是A5新转换三平台语言证据'}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
