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
ROOT = SOURCE / 'docs/DevDocs/assets/20o-b2-a5/entry-20261106'
REPO = 'repos/Programming-Language-Xiao/Xiao'
COMMIT = 'f21715d38069923653438fce8a7f3f8445f49164'
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
        assert len(candidates) == 1, title
        run = candidates[0]
        assert run['status'] == 'completed' and run['conclusion'] == 'success', title
        chosen[role] = run
    def execution(job):
        return (job['name'], job['head_sha'], job['started_at'], job['completed_at'],job['runner_id'],job['runner_name'],
                tuple((step['number'],step['name'],step.get('conclusion'),step.get('started_at'),step.get('completed_at')) for step in job['steps']))
    for role, run in chosen.items():
        get(REPO + '/actions/runs/' + str(run['id']), 'ci-' + role + '-run.json')
        jobs=json.loads(get(REPO + '/actions/runs/' + str(run['id']) + '/jobs?filter=all&per_page=100','ci-'+role+'-jobs.json'))['jobs']
        if role != 'maintenance': continue
        for platform, title in [('vm-linux','Rust workspace default tests (Linux)'),('native-windows','Native differential regression (Windows)'),('native-linux','Native differential regression (Linux)'),('native-macos','Native differential regression (macOS)')]:
            candidates=[job for job in jobs if job['name']==title]
            latest=max(candidates,key=lambda job:(job['run_attempt'],job['id']))
            same=[job for job in candidates if execution(job)==execution(latest)]
            origin=min(same,key=lambda job:(job['run_attempt'],job['id']))
            assert origin['conclusion']==latest['conclusion']=='success' and origin['head_sha']==COMMIT
            log='ci-'+platform+'.log'
            raw=get(REPO+'/actions/jobs/'+str(origin['id'])+'/logs',log)
            record={'code_commit':COMMIT,'run_id':run['id'],'job_id':origin['id'],'job_name':origin['name'],
                'run_attempt':origin['run_attempt'],'aggregate_job_id':latest['id'],'aggregate_run_attempt':latest['run_attempt'],
                'execution_retained':origin['id']!=latest['id'],'conclusion':origin['conclusion'],'log':log,'log_sha256':hashlib.sha256(raw).hexdigest(),
                'execution_identity':{'started_at':origin['started_at'],'completed_at':origin['completed_at'],'runner_id':origin['runner_id'],'runner_name':origin['runner_name']},
                'scope':'实际日志使用该次执行的原始作业ID；重跑复制的绿色映射不冒充新执行。A5功能/清理与完整示例，不作为A6性能。'}
            (ROOT/('ci-'+platform+'.json')).write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
